//! areka-P0-emo2-boot 統合結線モジュール（M-boot: 「emo2 が起動して喋る」最初の可視結果）。
//!
//! 完成済み 5 トラックのエンジン（seriko／sakura／emo-present／emo-text／actor＋dola）を
//! 束ねて「動くアプリ」にする最後の一結線を所有する。新規機構は作らず、シェルアニメーション
//! 側の表示指令を表示層の指令へ変換するアダプタ 1 個＋各エンジンの結線＋二段の観測に徹する
//! （design.md「変更境界」・R10.4）。
//!
//! 依存方向（レイヤ規律・design.md「依存方向（レイヤ規律）」）:
//! `target_map`（純粋・std のみ）→ `adapter`（seriko/emo-present 型）→ `talk_clock`（sakura 型＋clock）
//! → `assets`（parsers/atlas/compose/seriko/emo-present）→ `frame`（bevy_ecs World・emo-present/emo-text 駆動）
//! → `main.rs`（全結線）。左のモジュールは右を import しない。
//!
//! 本ファイル群は Foundation タスク（tasks.md task 1）の骨格であり、各サブモジュールの
//! 機能実装は後続タスク（2〜6）が担う。

pub mod target_map;
pub mod hit_region;
pub mod adapter;
pub mod talk_clock;
pub mod assets;
pub mod frame;
pub mod move_cue;
pub mod consumer_ledger;
pub mod hover_inject;

// 決定論 spine テストハーネス（R8・task 6.1）。`areka` は [[bin]] のみ（[lib] 無し）で外部
// tests/ から bin 内部項目へ到達できないため、既存 repo 慣行に従い in-crate #[cfg(test)]
// モジュールとして置く（設計 File Structure の tests/emo2_boot_spine_test.rs との差分は spine.rs
// 冒頭 doc 参照）。frame フェーズ直接駆動・Tick 注入・GPU readback を in-process で行う。
#[cfg(test)]
mod spine;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::ticker::{LoopTickerConfig, Tick, TickerMsg, spawn_loop_ticker};
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::MountError;
use areka_seriko::{
    AnimationTable, BindResolver, SerikoLoopConfig, SerikoSink, SurfaceResolver, seeded_rng,
    spawn_seriko,
};
use tracing::{error, info, warn};
use wintf::WinApp;
use wintf::ecs::FrameFinalize;

use self::adapter::PresentBridge;
use self::assets::{BootAssets, LoopTables, build_boot_assets};
use self::frame::{Emo2Wiring, emo2_frame_system};
use self::move_cue::{MoveCueSink, MoveDirective};
use self::talk_clock::{ClockedTextSink, TalkClock};

/// 統合結線の構築時（load-time）失敗を観測可能化する誤り型（log-first・R7.3）。
///
/// 各段（mount／shell 読取＋parse／bake／balloon 組立／UI アクター spawn）の失敗を
/// `#[from]` 変換で集約し、呼び手（`wire_emo2_boot`）が `MountError::StartPointMissing` 系は
/// `warn!`・他は `error!` に分類して `LogSink`×2 フォールバック boot へ倒す（design.md
/// 「Error Categories and Responses」）。
///
/// load-time バリアント: 構築入力の組立（`build_boot_assets`・task 2.6）が返す
/// `Mount`／`Decoder`／`ShellRead`／`ShellEmpty`／`Balloon` に加え、UI アクター spawn 失敗
/// （`SpawnUi`・`wire_emo2_boot`＝task 5.1 が充填）を集約する。分類
/// （`MountError::StartPointMissing` 系＝想定内 `warn!`・他＝`error!`）は呼び手
/// （`wire_emo2_boot`／`classify_wiring_error`・task 5.1）が担い、本型はバリアントを観測可能に
/// するのみ（panic せず `Err` を返す・log-first R7.3）。
///
/// `#[from]` は変換元が `std::error::Error` を実装する型のみに用いる。
/// `areka_parsers::package::MountError` は `std::error::Error` 未実装のため
/// `#[from]`/`#[source]` にせず値保持し `Debug` 表示する（placement `PlacementError`
/// と同流儀・working donor 準拠）。
#[derive(Debug, thiserror::Error)]
pub enum BootWiringError {
    /// ゴーストのマウント解決（`areka_parsers::package::resolve`）失敗。
    ///
    /// `MountError`（`StartPointMissing`／`ShellDirMissing` 等）は `std::error::Error`
    /// 未実装のため `#[from]` にせず値保持する（呼び手が variant 内容で warn/error 分類）。
    #[error("ゴーストのマウント解決に失敗: {0:?}")]
    Mount(areka_parsers::package::MountError),

    /// WIC デコーダ（COM）の生成失敗。呼び出しスレッドの COM 未初期化が主因。
    #[error("WIC デコーダの生成に失敗（COM 未初期化？）")]
    Decoder(#[source] windows::core::Error),

    /// shell ファイル（`surfaces.txt`／`descript.txt`）の読み取り失敗（I/O）。
    #[error("shell ファイルの読み取りに失敗: {path}")]
    ShellRead {
        /// 読み取れなかったファイルのパス。
        path: PathBuf,
        /// 元の I/O エラー。
        #[source]
        source: std::io::Error,
    },

    /// `surfaces.txt` は読めたが surface を 1 つも産まなかった（bake/表示対象なし）。
    #[error("surfaces.txt が surface 定義を産まなかった: {path}")]
    ShellEmpty {
        /// surface 定義が空だった `surfaces.txt` のパス。
        path: PathBuf,
    },

    /// バルーン表示対象（`areka_emo_present::build_balloon_target`）の構築失敗。
    ///
    /// `PresentError` は `thiserror` 派生（`std::error::Error` 実装）ゆえ `#[from]` で畳む。
    #[error("バルーン表示対象の構築に失敗")]
    Balloon(#[from] areka_emo_present::PresentError),

    /// バルーン文字層 UI アクター（`areka_emo_text::actor::spawn_emo_text`）の spawn 失敗。
    ///
    /// `spawn_emo_text` は UI（pump）スレッド外呼び出しを検出すると
    /// [`areka_actor::UiSpawnError::NotUiThread`] を返す。本番は `WinApp::new()` 済みの
    /// UI スレッドで `wire_emo2_boot` を呼ぶため通常発生しないが、万一失敗した場合は
    /// 握り潰さず `error!`＋`wired=false` フォールバックへ倒す（log-first・R7.3）。
    /// `UiSpawnError` は `thiserror` 派生（`std::error::Error` 実装）ゆえ `#[source]` で連鎖する。
    #[error("バルーン文字層 UI アクターの spawn に失敗")]
    SpawnUi(#[source] areka_actor::UiSpawnError),
}

// ===========================================================================
// wire_emo2_boot 統合結線（tasks.md task 5.1・design「エントリポイント / main.rs＋
// wire_emo2_boot」）— 完成済み 5 トラックを束ねる最後の一結線（M-boot の心臓部）。
// ===========================================================================

/// `wire_emo2_boot` の結果（design「Service Interface」・main が終了処理で消費する）。
///
/// - `ghost`: boot 成立時の [`areka_ghost::GhostRuntime`]（フォールバック時は `None`）。
/// - `seriko`: seriko アクターの [`areka_actor::ActorHandle`]（フォールバック時は `None`）。
/// - `wired`: 実 sink 結線が成立したか（`false` = `LogSink`×2 フォールバックへ委ねる・R7.3）。
/// - `loop_ticker`: SERIKO ループ ticker（[`spawn_loop_ticker`]）の停止端（フォールバック時 `None`）。
pub struct Emo2BootOutcome {
    /// boot 成立時の ghost ランタイム（フォールバック時 `None`）。
    pub ghost: Option<areka_ghost::GhostRuntime>,
    /// seriko アクターの join ハンドル（フォールバック時 `None`）。
    pub seriko: Option<areka_actor::ActorHandle>,
    /// 実 sink 結線が成立したか（`false` = `LogSink` フォールバック）。
    pub wired: bool,
    /// SERIKO ループ ticker（16ms 実時計・[`spawn_loop_ticker`]）の停止端。
    /// `main`（task 9.5）の終了処理が [`TickerMsg::Close`] を送って ticker を止める
    /// （フォールバック時は ticker を起こさないため `None`）。
    pub loop_ticker: Option<std::sync::mpsc::Sender<TickerMsg>>,
}

/// M-boot の scope 集合を導出する（design DD-12・line 460「placement と同じ入力から自前導出」）。
///
/// emo2 fixture は sakura（scope0）＋kero（scope1）の 2 scope 構成であり、placement の
/// `detect_scopes`（scope0 常設・`kero.*` 存在で scope1）も emo2 に対し `[0, 1]` を返す。
/// placement の実結果は `open_startup_window` の async クロージャへ move 済みで同期参照不能
/// （DD-12）ゆえ、本関数が同じ scope 集合を独立に導出する。導出の二元性（placement の動的
/// `detect_scopes` との厳密一致は取らない）は M1 受容トレードオフとして増分申し送り（design
/// line 460）。窓と資産の不一致は [`frame::plan_attachments`]（DD-12）が missing/unused へ分類し
/// 優しく縮退するため、多少のズレでも hang/crash しない。
fn derive_scopes() -> Vec<u32> {
    vec![0, 1]
}

/// [`BootWiringError`] を「起点不在（良性・`warn!`）」と「それ以外（予期しない・`error!`）」へ
/// 分類して log-first 観測する（design「Error Categories and Responses」構築時・R7.3）。
///
/// `default_ghost_root()` はプレースホルダ subpath でこのサンドボックスでは常態的に不在
/// （＝`MountError::StartPointMissing` は想定内）ゆえ `warn!` どまり。他（読取不能／bake／
/// balloon／`spawn_ui` 失敗等）は真に予期しない失敗として `error!` で区別する（main の
/// `is_benign_placement_error`/`is_benign_boot_error` と同じ分類方針）。boot 自体の
/// `GhostBootError` は別途 [`crate::is_benign_boot_error`]（R7.4）が分類する。
fn classify_wiring_error(err: &BootWiringError) {
    match err {
        // fixture 不在等の想定内（このサンドボックスでは常態）は warn どまり。
        BootWiringError::Mount(MountError::StartPointMissing { .. }) => {
            warn!(
                error = %err,
                "emo2-boot: 構築入力の起点が見つかりません（LogSink フォールバックへ委ねる・R7.3）"
            );
        }
        // 他は真に予期しない失敗として error（読取不能／bake／balloon／spawn_ui 等）。
        _ => {
            error!(
                error = %err,
                "emo2-boot: 構築入力の組立に失敗（LogSink フォールバックへ委ねる・R7.3）"
            );
        }
    }
}

/// `spawn_seriko` が返す [`SerikoSink`] が boot の型境界（単一の出力契約
/// `dola::cue::CueSink + Clone + Send + 'static`）を満たすことをコンパイル時に固定する
/// （upstream `areka-seriko` の `Clone`／`CueSink` 契約の回帰檻）。
///
/// `areka_ghost::boot` は `S: dola::cue::CueSink + Clone + Send + 'static` を要求し、dispatcher は talk
/// ごとに sink を clone する（`areka-ghost` dispatcher.rs — 構造的に `Clone` 必須）。[`SerikoSink`] は
/// upstream `areka-seriko` で `dola::cue::CueSink` を実装し `#[derive(Clone)]` 済み（内側
/// `mpsc::Sender<SerikoMsg>` は常に `Clone`・全 clone は単一 seriko inbox への送信端で配送意味は同一）
/// ゆえ、`spawn_seriko` の戻り値を直接 boot へ渡せる。万一この upstream 契約が外れると本アサーションが
/// コンパイルを止め、境界破れを早期検出する。
const _: fn() = || {
    fn assert_boot_surface_bounds<S: dola::cue::CueSink + Clone + Send + 'static>() {}
    assert_boot_surface_bounds::<SerikoSink>();
};

/// UI 基盤構築後・`run()` 前に呼ぶ統合結線（UI スレッド・design「エントリポイント」7 手順統括）。
///
/// 完成済み 5 トラック（seriko／sakura／emo-present／emo-text／actor＋dola）を束ねて「動くアプリ」
/// にする最後の一結線。`main.rs` 私有型（`ConfigInputs`）へは結合せず解決済みルートパスを受け取る
/// （design「Service Interface」）。**新規機構は作らない**（R10.4）——構築入力の組立と各エンジンの
/// 結線＋二段の観測に徹する。
///
/// # 7 手順（design「wire_emo2_boot の統括」）
/// 1. [`build_boot_assets`]（`scopes` は [`derive_scopes`] で placement と同じ入力から自前導出・
///    DD-12）。`Err` は [`classify_wiring_error`]（起点不在＝`warn!`・他＝`error!`・R7.3）の上
///    `wired=false` を返し、呼び手（`main`）の `LogSink`×2 フォールバック boot へ委ねる。
/// 2. [`EmoPresenter::new`]／[`TextLayerRuntime::new`]（`Rc<RefCell<>>`）／[`spawn_emo_text`]
///    （UI スレッド前提。`Err` は [`BootWiringError::SpawnUi`] 分類＋`wired=false` フォールバック）。
/// 3. [`TalkClock::new`]（`dola::runtime::clock::now` 注入）／[`ClockedTextSink::new`]。
/// 4. `mpsc::channel::<PresentCommand>()`／[`PresentBridge::new`]／[`spawn_seriko`]。
///    **SurfaceResolver 突き合わせ（Task 4.1 申し送り）**: `resolver`（非 Clone）は `spawn_seriko`
///    が値消費し、`static_binds`（Clone）は clone して seriko へ渡す。`Emo2Wiring` が保持する
///    `BootAssets` の `resolver` は attach で読まれないため無害なプレースホルダ（空 alias 表）で埋める。
/// 5. [`areka_ghost::boot`]（`surface_sink`＝[`SerikoSink`] を直渡し・`text_sink`＝`ClockedTextSink`・
///    `shiori`＝`Helper`・`ticker`＝`Real`）。`Err` は既存 [`crate::is_benign_boot_error`]（R7.4）で
///    分類（起点不在＝`warn!`・他＝`error!`）＋`wired=false` フォールバック。
/// 6. [`Emo2Wiring`] を NonSend 挿入・`add_systems(FrameFinalize, emo2_frame_system)`（placement の
///    click-through 登録と同位置・self-gating・順序依存なし）。成立時 `info!`「wire 成立」マーカーを
///    発火（実 fixture smoke＝task 7.1 がこの存在を assert する）。
/// 7. `Emo2BootOutcome{ ghost: Some, seriko: Some, wired: true }` を返す。
///
/// # 失敗（log-first・panic しない・R7.3）
/// いずれの手順の失敗も `warn!`／`error!`＋`Emo2BootOutcome{ ghost: None, seriko: None,
/// wired: false }` へ縮退し、`main` の現行 `LogSink`×2 boot（既存 smoke 温存）へ委ねる。boot 成立後の
/// spawn 済み seriko は、boot 失敗時 `surface_sink` drop で inbox 切断→worker 自然終了ゆえ handle を
/// drop（非 RAII・detach）して打ち切る（hang させない）。
pub fn wire_emo2_boot(
    app: &WinApp,
    ghost_root: &Path,
    balloon_root: &Path,
    helper_exe: &Path,
) -> Emo2BootOutcome {
    /// 実 sink 結線を成立させないフォールバック結果（`main` の `LogSink`×2 boot へ委ねる・R7.3）。
    fn fallback() -> Emo2BootOutcome {
        Emo2BootOutcome {
            ghost: None,
            seriko: None,
            wired: false,
            loop_ticker: None,
        }
    }

    // 手順1: 構築入力の一括組立（scopes は placement と同じ入力から自前導出・DD-12）。
    // 失敗（fixture 不在等）は分類 warn/error の上 wired=false フォールバックへ倒す（R7.3）。
    let scopes = derive_scopes();
    // scaffold（task 4.3 が main.rs から実 author_dpi を渡すよう結線する）: 既定 96＝従来と同一挙動。
    let assets = match build_boot_assets(ghost_root, balloon_root, &scopes, 96, 96) {
        Ok(assets) => assets,
        Err(err) => {
            classify_wiring_error(&err);
            return fallback();
        }
    };

    // 手順2: presenter／文字層ランタイム（Rc<RefCell<>>）／文字層 UI アクター（UI スレッド前提）。
    // spawn_emo_text の Err（UI スレッド外呼び出し検出等）は SpawnUi 分類＋フォールバック（R7.3）。
    let presenter = EmoPresenter::new();
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    let (emo_text_sink, _emo_text_join) = match spawn_emo_text(Rc::clone(&runtime)) {
        Ok(pair) => pair,
        Err(err) => {
            classify_wiring_error(&BootWiringError::SpawnUi(err));
            return fallback();
        }
    };

    // 手順3: talk 時刻源（dola clock 注入）＋時刻付与テキスト sink（emit ごとに epoch 推定）。
    let clock_fn: Arc<dyn Fn() -> f64 + Send + Sync> = Arc::new(dola::runtime::clock::now);
    let clock = TalkClock::new(clock_fn);
    let clocked_text_sink = ClockedTextSink::new(emo_text_sink, clock.clone());

    // 手順4: mpsc＋PresentBridge→spawn_seriko。SurfaceResolver 突き合わせ（!Clone・Task 4.1 申し送り）:
    // resolver（非 Clone）は spawn_seriko が値消費・static_binds（Clone）は clone して seriko と
    // Emo2Wiring の双方へ配る。Emo2Wiring 側 BootAssets の resolver は attach で読まれない（Task 4.1）
    // ため空 alias 表のプレースホルダで埋める（実 resolver は seriko が保持）。
    let (tx, rx) = std::sync::mpsc::channel::<PresentCommand>();
    let bridge = PresentBridge::new(tx);
    // move channel（PresentBridge と同型の配線・task 9.1）: talk スレッドの MoveCueSink が送出端、
    // UI スレッドの Emo2Wiring が受信端（frame 相 drain＝task 9.2 が消費）を保持する。
    let (move_tx, move_rx) = std::sync::mpsc::channel::<MoveDirective>();
    let move_sink = MoveCueSink::new(move_tx);
    let BootAssets {
        shells,
        balloons,
        balloon_model,
        resolver,
        static_binds,
        bind_resolver,
        loop_tables,
        shell_author_dpi,
        balloon_author_dpi,
    } = assets;
    // SERIKO ループ構成（design「本番は実時間・実 entropy 接続」・R7.4）: シェル／バルーンの 2 表は
    // `BootAssets.loop_tables`（task 9.1 が `EmoWorld` スナップショットから `from_world` で構築）を
    // 値移送し、乱数は実 entropy から。seed は `std::hash::RandomState`（プロセスごとにランダム鍵で
    // 初期化される `BuildHasher`）から u64 を実現する（新規 crates.io 依存なし・design 準拠）。空書込みの
    // `finish()` はその hasher のランダム鍵由来値を返すため、実行ごとに異なる seed が得られる。
    let LoopTables {
        shell: shell_table,
        balloon: balloon_table,
    } = loop_tables;
    let seed: u64 = {
        use std::hash::{BuildHasher, Hasher};
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    };
    // 再現可能性の観測点（R7.4/7.5）: 本番 seed を info! に出力し、同一 seed で run を再現可能にする。
    info!(seed, "emo2-boot: SERIKO ループ乱数 seed（RandomState 由来・実 entropy・再現用）");
    let loop_config = SerikoLoopConfig {
        shell_table,
        balloon_table,
        rng: seeded_rng(seed),
    };
    // boot は S: dola::cue::CueSink + Clone を要求する。SerikoSink は upstream `areka-seriko` で
    // `CueSink` を実装し `#[derive(Clone)]` 済み（内側 mpsc::Sender は常に Clone・全 clone は単一 inbox
    // 送信端で配送同一）ゆえ spawn_seriko の戻り値を直接 surface_sink として boot へ渡す（共有 shim は不要）。
    // `bind_resolver`（BootAssets・task 7.1 が MountModel の名前転記から構築した実名前解決表）を
    // seriko の起動へ値渡しで配線する。bind cue（`\![bind]`）の着せ替え名解決はこの実表が担う（task 7.2）。
    // `loop_config`（上で組んだ実表＋実 entropy 乱数）を seriko へ値渡しし、SERIKO ループ統括器を起こす。
    let (surface_sink, seriko_handle) = spawn_seriko(
        resolver,
        static_binds.clone(),
        bind_resolver,
        loop_config,
        bridge,
    );
    let wiring_assets = BootAssets {
        shells,
        balloons,
        balloon_model,
        // attach は resolver を一切読まない（Task 4.1 申し送り）ため無害なプレースホルダ。
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds,
        // 実 bind_resolver は seriko が値消費済み（attach は bind_resolver を読まない）ため空表プレースホルダ。
        bind_resolver: BindResolver::empty(),
        // 実 loop_tables は loop_config へ移送済み（attach は loop_tables を読まない）ため空表プレースホルダ。
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: AnimationTable::empty(),
        },
        // 作者基準 DPI は搬送のみ（本相は値を解釈しない・attach への供給は task 4.2）。
        shell_author_dpi,
        balloon_author_dpi,
    };

    // loop ticker 用の tick 送出端: SerikoSink を 1 本 clone して保持する（surface_sink 本体は下の
    // boot_options が値消費する）。全 clone は単一 seriko inbox への送信端で配送意味は同一（task 9.2）。
    // boot 失敗時はこの clone が inbox を生かし続けないよう明示 drop する（worker 自然終了・下記）。
    let tick_sink = surface_sink.clone();

    // 手順5: boot（実 sink 注入）。Err は既存 is_benign_boot_error 分類（R7.4）＋フォールバック。
    // sinks は broadcast 登録先で、surface（seriko）／text（ClockedTextSink）／move（MoveCueSink）の
    // 3 sink を第 1〜3 要素として渡す（task 9.1）。move_sink は `\![move]` を名前選別で消費し、
    // MoveDirective を move channel 経由で UI スレッド（Emo2Wiring の move_rx）へ送出する。
    let boot_options = GhostBootOptions {
        ghost_root: ghost_root.to_path_buf(),
        default_encoding: DefaultEncoding::Ansi,
        shiori: ShioriWiring::Helper {
            helper_exe: helper_exe.to_path_buf(),
        },
        sinks: vec![
            Box::new(surface_sink),
            Box::new(clocked_text_sink),
            Box::new(move_sink),
        ],
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: Some(crate::default_app_profile_dir()),
        ticker: TickerMode::Real(Default::default()),
    };
    let ghost_runtime = match areka_ghost::boot(boot_options) {
        Ok(runtime) => runtime,
        Err(err) => {
            // R7.4: 既存 main と同一方針で分類（起点不在＝良性 warn・他＝error）。
            if crate::is_benign_boot_error(&err) {
                warn!(
                    error = %err,
                    "emo2-boot: ghost boot の起点が見つかりません（LogSink フォールバックへ委ねる・R7.4）"
                );
            } else {
                error!(
                    error = %err,
                    "emo2-boot: ghost boot に失敗（LogSink フォールバックへ委ねる・R7.4）"
                );
            }
            // spawn 済み seriko の後始末: surface_sink は boot_options 消費で drop 済み。ただし tick 用
            // clone（tick_sink）はまだ生存し inbox を生かし続けるため、ここで明示 drop する。両 Sender が
            // 消えると inbox 切断→worker 自然終了する。ActorHandle は非 RAII（drop で detach）ゆえ join
            // せず drop で打ち切る（万一 boot が dispatcher へ move 後に失敗しても hang しない）。
            drop(tick_sink);
            drop(seriko_handle);
            return fallback();
        }
    };

    // 手順6: Emo2Wiring を NonSend 挿入＋emo2_frame_system を FrameFinalize へ登録（placement の
    // click-through 登録と同位置・self-gating・順序依存なし）。EcsWorld は insert_non_send_resource を
    // 直接持たないため world_mut() 経由で bevy World へ載せる（add_systems は EcsWorld 直メソッド）。
    let wiring = Emo2Wiring::new(presenter, rx, move_rx, runtime, clock, wiring_assets);
    app.world()
        .borrow_mut()
        .world_mut()
        .insert_non_send_resource(wiring);
    app.world()
        .borrow_mut()
        .add_systems(FrameFinalize, emo2_frame_system);

    // SERIKO ループ ticker 起動（design「本番は実時間・実 entropy 接続」・R7.4）: 16ms 実時計
    // （LoopTickerConfig::default）で駆動し、各 Tick を tick_sink（SerikoSink クローン）経由で seriko
    // へ届ける。ghost の loop ticker は seriko を一切知らず、クロージャがその継ぎ目（依存方向: areka が
    // ghost の spawn_loop_ticker を seriko の SerikoSink へ結線・ghost→seriko 依存は張らない）。
    // Tick.now は MonotonicMs（内側 u64・ms）ゆえ `.0` を send_tick へ渡す。
    let (loop_ticker_stop, _loop_ticker_handle) = spawn_loop_ticker(
        LoopTickerConfig::default(),
        Box::new(move |tick: Tick| {
            tick_sink.send_tick(tick.now.0);
        }),
    );

    // wire 成立マーカー（実 fixture smoke＝task 7.1 がこの info! の存在を assert する・R7.3 観測境界）。
    info!("emo2-boot: 実 sink 結線が成立しました（wire 成立）");

    // 手順7: 実 sink 結線成立。ghost/seriko ハンドル＋loop ticker 停止端を main の終了処理へ返す。
    Emo2BootOutcome {
        ghost: Some(ghost_runtime),
        seriko: Some(seriko_handle),
        wired: true,
        loop_ticker: Some(loop_ticker_stop),
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;
    use std::path::Path;
    use wintf::WinApp;

    /// 観測可能な完了条件（tasks.md task 5.1）: 存在しない ghost_root に対し
    /// `wire_emo2_boot` は `wired=false`（`LogSink` フォールバックへ委ねる・R7.3）を返し、
    /// ghost／seriko ハンドルは `None` となる（実 sink 結線を成立させない決定分岐の檻）。
    ///
    /// `WinApp::new()` は headless 構築可能（`crates/wintf/tests/win_app.rs` 実績・COM/DPI のみ・
    /// 窓/ループなし）。`build_boot_assets` が `resolve` 起点不在で早期 `Err` を返すため、
    /// spawn_emo_text／spawn_seriko／boot へは一切到達せず（＝アクタースレッドを起こさない）
    /// 決定論的に判定できる。
    #[test]
    fn wire_emo2_boot_falls_back_to_unwired_on_missing_ghost_root() {
        let app = WinApp::new().expect("headless WinApp::new は成功する（COM/DPI のみ）");
        let missing_ghost = Path::new("C:/areka-nonexistent-ghost-root-emo2boot-5v1");
        let missing_balloon = Path::new("C:/areka-nonexistent-balloon-root-emo2boot-5v1");
        let helper = Path::new("shiori-host32-helper.exe");

        let outcome = wire_emo2_boot(&app, missing_ghost, missing_balloon, helper);

        assert!(
            !outcome.wired,
            "存在しない ghost_root では実 sink 結線は成立せず LogSink フォールバックへ委ねる（R7.3）"
        );
        assert!(
            outcome.ghost.is_none(),
            "フォールバック時は ghost ランタイムを起動しない（None）"
        );
        assert!(
            outcome.seriko.is_none(),
            "フォールバック時は seriko アクターを起こさない（None）"
        );
    }
}
