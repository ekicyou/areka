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
//! → `ghost_session.rs`／`main.rs`（全結線）。左のモジュールは右を import しない。
//!
//! 各サブモジュールが機能の実装を持つ。

pub mod adapter;
pub mod assets;
/// バルーン面 0 の原点画素から背景色を導く（無効表示の混色の相手・要件 4.6）。
mod balloon_background;
pub mod balloon_visibility;
/// `\![change,ghost,…]` の受け口（areka-P0-ghost-shell-balloon-switch）。
mod change_cue;
pub mod consumer_ledger;
pub mod frame;
/// ゴーストの切替の入口（areka-P0-ghost-shell-balloon-switch）。
pub(crate) mod ghost_switch;
pub mod hit_region;
pub mod hover_inject;
mod install_cue;
pub mod move_cue;
mod readme_cue;
/// 読み直しの窓口（areka-P0-mcp-reload）。
pub(crate) mod reload;
pub(crate) mod reload_args;
/// `\![reload,…]`・`\![reloadsurface]` の受け口（areka-P0-mcp-reload）。
mod reload_cue;
/// 実行系の側の起動（受け口の送出端の束・areka-P0-mcp-reload）。
pub(crate) mod runtime_boot;
/// 同じフォルダ名のゴーストを今のゴーストとして扱う（areka-P0-baseware-root-list）。
pub(crate) mod same_folder_pin;
/// 出どころで実行を制限するタグの判定の表（areka-P0-script-security-level）。
pub(crate) mod script_restriction;
/// シェル・バルーンの切替先の名前の解決とインストールの控え（areka-P0-shell-balloon-switch）。
pub(crate) mod shell_balloon_resolve;
/// シェル・バルーンの切替の入口（areka-P0-shell-balloon-switch）。
pub(crate) mod shell_balloon_switch;
/// 箱の束（置き場所の表・別名の写し・在るサーフェス番号・フォントの探す場所・areka-P0-shell-balloon）。
pub(crate) mod shell_box_assets;
/// ghost の読み直しで表示中の面を作り直した窓へ持ち越す（areka-P0-mcp-reload 要件 3.8）。
pub(crate) mod shell_carry;
pub(crate) mod shell_cursor_assets;
pub(crate) mod shell_cursor_table;
mod special_name;
/// シェル・バルーンの差し替えの資産を背景で作る部品と荷物の置き場（areka-P0-shell-balloon-switch）。
pub(crate) mod switch_assets;
/// ゴーストの切替で同梱のゴーストを書ける根へ背景で写す（areka-P0-user-data-root）。
pub(crate) mod switch_copy;
/// `\![change,shell|balloon,…]` の受け口（areka-P0-shell-balloon-switch）。
mod switch_cue;
pub mod talk_clock;
pub mod talk_lifecycle;
pub mod target_map;
mod update_cue;
pub(crate) mod user_break_cue;
pub mod zorder_cue;

// 決定論 spine テストハーネス（R8・task 6.1）。`areka` は [[bin]] のみ（[lib] 無し）で外部
// tests/ から bin 内部項目へ到達できないため、既存 repo 慣行に従い in-crate #[cfg(test)]
// モジュールとして置く（設計 File Structure の tests/emo2_boot_spine_test.rs との差分は spine.rs
// 冒頭 doc 参照）。frame フェーズ直接駆動・Tick 注入・GPU readback を in-process で行う。
#[cfg(test)]
pub(crate) mod spine;

// emo2 検体の共有の受け口（spec: areka-P0-nar-install 要件 1.6）。`emo2_boot` 配下のテストが
// 検体をここから引くので、保持（＝段 ③ の複製）はテストバイナリあたり 1 回で済む。
#[cfg(test)]
pub(crate) mod sample_test_support;

// 偽の SHIORI を持つゴーストを同じ World で起こす統合テストの土台（areka-P0-ghost-shell-balloon-switch）。
#[cfg(test)]
pub(crate) mod ghost_switch_test_support;

// タグ入口の結線（areka-P0-scope-zorder-pinning task 6.2）の檻。受け渡し口・入口の登録・
// 受け渡し構造・相の呼出という 4 点は、削っても判断のテストが 1 本も赤くならない性質を
// 持つので、到達性・相順・字面の 3 方向でここが受け持つ。
#[cfg(test)]
#[path = "zorder_wiring_tests.rs"]
mod zorder_wiring_tests;

// 毎フレームの相の登録先の檻（areka-P0-present-gpu-transform-scale task 4.2・T-N10）。
// 裁定 2026-09-12 で登録先が 1 巡の末尾の段から `Update` へ移った——絵の着地を文字と同じ巡に
// 揃えるための移動であり、実窓を持たない檻には挙動として映らないので、字面と `Schedules` の
// 構造の両方から押さえる。
#[cfg(test)]
#[path = "frame_schedule_tests.rs"]
mod frame_schedule_tests;

#[cfg(test)]
#[path = "seriko_clock_tests.rs"]
mod seriko_clock_tests;

// 本物の読み手から合成まで、動く絵をシェルとバルーンで通す（areka-P0-animated-image-playback task 5.4）。
#[cfg(test)]
#[path = "film_playback_e2e_tests.rs"]
mod film_playback_e2e_tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use areka_emo_present::shell_target::ShellLoadError;
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::ticker::{LoopTickerConfig, Tick, TickerMsg, spawn_loop_ticker};
use areka_ghost::{ShioriWiring, TickerMode};
use areka_kanade::{BootOrigin, KanadeNotice, MonotonicMs};
use areka_parsers::package::MountError;
use areka_seriko::{
    AnimationTable, SerikoClock, SerikoLoopConfig, SerikoSink, SurfaceResolver, seeded_rng,
    spawn_seriko_clocked,
};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use tracing::{error, info, warn};
use wintf::ecs::{Update, update_typewriters};

use crate::placement::{AuthorDpi, DefaultSurfaces};

use self::adapter::PresentBridge;
use self::assets::{
    BootAssets, LoopTables, actor_keyed_balloon_tables, build_boot_assets_with_shell,
};
use self::frame::{Emo2Wiring, KanadeNoticeRx, emo2_frame_system, ghost_quit_system};
use self::move_cue::PositionCue;
use self::runtime_boot::CueSinkSources;
use self::talk_clock::TalkClock;
use self::talk_lifecycle::TalkLifecycleSignal;
use self::zorder_cue::ZOrderDirective;

/// 統合結線の構築時（load-time）失敗を観測可能化する誤り型（log-first・R7.3）。
///
/// 各段（mount／shell 読み込み＝一覧＋読取＋parse＋bake／balloon 組立／UI アクター spawn）の失敗を
/// `#[from]` 変換で集約し、呼び手（`wire_emo2_boot`）が `MountError::StartPointMissing` 系は
/// `warn!`・他は `error!` に分類して `wired=false` を呼び手へ返す（`ghost_session::boot_ghost` は
/// `LogSink`×2 フォールバック boot へ倒し、`ghost_session::boot_ghost_strict` は `Err` を返す・
/// design.md「Error Categories and Responses」）。
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

    /// shell の読み取り失敗（I/O）。`surfaces.txt`／`descript.txt` のほか、シェルの
    /// フォルダそのものの一覧に失敗した場合もここへ畳む（`path` はそのフォルダになる・
    /// 下の `From<ShellLoadError>`）。
    #[error("shell ファイルの読み取りに失敗: {path}")]
    ShellRead {
        /// 読み取れなかったファイルのパス。
        path: PathBuf,
        /// 元の I/O エラー。
        #[source]
        source: std::io::Error,
    },

    /// シェルに面が 1 つも無い——`surfaces.txt` が無いか面を定義せず、面の画像も無い
    /// （bake/表示対象なし・spec: areka-P0-self-alpha-declaration 要件 6.5）。
    #[error("シェルに面が 1 つも無い: {path}")]
    ShellEmpty {
        /// シェルのフォルダ（`surfaces.txt` が無い場合があるため、そのファイルではない）。
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
    /// 握り潰さず `error!`＋`wired=false` を呼び手へ返す（log-first・R7.3）。
    /// `UiSpawnError` は `thiserror` 派生（`std::error::Error` 実装）ゆえ `#[source]` で連鎖する。
    #[error("バルーン文字層 UI アクターの spawn に失敗")]
    SpawnUi(#[source] areka_actor::UiSpawnError),
}

/// シェル読み込みの権威の失敗を、起動結線の既存の枝へ写す（枝の追加 **0 件**）。
///
/// 対応は次のとおりで、失敗の真因のログは権威（`areka_emo_present::shell_target`）が
/// 既に `error!` で出している（ここでは出し直さない・log-first の二重記録を作らない）。
///
/// - [`ShellLoadError::List`]（シェルのフォルダの一覧が取れない）→ [`BootWiringError::ShellRead`]。
///   `path` はシェルのフォルダそのものになる（読めなかった対象がファイルではなくフォルダ）。
/// - [`ShellLoadError::Read`]（`surfaces.txt` が在るのに読めない）→ [`BootWiringError::ShellRead`]。
/// - [`ShellLoadError::Empty`]（シェルに面が 1 つも無い）→ [`BootWiringError::ShellEmpty`]。
///   `path` はシェルのフォルダのまま運ぶ。
impl From<ShellLoadError> for BootWiringError {
    fn from(error: ShellLoadError) -> Self {
        match error {
            ShellLoadError::List { path, source } | ShellLoadError::Read { path, source } => {
                BootWiringError::ShellRead { path, source }
            }
            ShellLoadError::Empty { path } => BootWiringError::ShellEmpty { path },
        }
    }
}

// ===========================================================================
// wire_emo2_boot 統合結線（tasks.md task 5.1・design「エントリポイント / main.rs＋
// wire_emo2_boot」）— 完成済み 5 トラックを束ねる最後の一結線（M-boot の心臓部）。
// ===========================================================================

/// `wire_emo2_boot` の結果（design「Service Interface」・`ghost_session` の結線ありの腕が
/// `GhostSession` へ束ね、終了処理（`GhostSession::shutdown`）で消費する）。
///
/// - `ghost`: boot 成立時の [`areka_ghost::GhostRuntime`]（不成立時は `None`）。
/// - `seriko`: seriko アクターの [`areka_actor::ActorHandle`]（不成立時は `None`）。
/// - `wired`: 実 sink 結線が成立したか（`false` の帰結は呼び手が決める: `boot_ghost` は `LogSink`×2
///   フォールバックへ倒し、`boot_ghost_strict` は `Err` を返す・R7.3）。
/// - `loop_ticker`: SERIKO ループ ticker（[`spawn_loop_ticker`]）の停止端（不成立時 `None`）。
/// - `seriko_sink`: seriko の送り手の複製（不成立時 `None`・持ち主は `GhostSession` だけ）。
/// - `sources`: 受け口の送出端の束（不成立時 `None`）。
pub struct Emo2BootOutcome {
    /// boot 成立時の ghost ランタイム（不成立時 `None`）。
    pub ghost: Option<areka_ghost::GhostRuntime>,
    /// seriko アクターの join ハンドル（不成立時 `None`）。
    pub seriko: Option<areka_actor::ActorHandle>,
    /// 実 sink 結線が成立したか（`false` の帰結は呼び手が決める）。
    pub wired: bool,
    /// SERIKO ループ ticker（16ms 実時計・[`spawn_loop_ticker`]）の停止端。
    /// 終了処理（`GhostSession::shutdown`・task 9.5）が [`TickerMsg::Close`] を送って ticker を止める
    /// （不成立時は ticker を起こさないため `None`）。
    pub loop_ticker: Option<std::sync::mpsc::Sender<TickerMsg>>,
    /// seriko の送り手の複製（シェル・バルーンの差し替えを seriko へ頼む口・不成立時 `None`）。
    /// 持ち主は `GhostSession` だけで、降ろす最初の段（loop ticker の停止）で落ちる——降ろした後も
    /// World に残る置き場（`Emo2Wiring` ほか）に持たせると、seriko の join が戻らない
    /// （areka-P0-shell-balloon-switch 要件 8.5）。
    pub seriko_sink: Option<SerikoSink>,
    /// 受け口の送出端の束（実行系を組み直すたびに複製して sink を組む・不成立時 `None`・
    /// areka-P0-mcp-reload 要件 4.2）。持ち主は `GhostSession` で、降ろす最初の段で落とす。
    pub(crate) sources: Option<CueSinkSources>,
}

/// `wire_emo2_boot` の入力の束（ゴーストごとに変わる値と、SHIORI の結線の差し替え口）。
///
/// `GhostBootOptions` には欄を足さず、`ghost_root`・`shiori`・`ticker`・`app_profile_dir` の
/// 値の出所をここへ移すだけである（要件 5.7）。本番は `Helper { helper_exe }`・`Real` 既定・
/// `Some(default_app_profile_dir())` を詰め、テストは偽の SHIORI（`Custom`）を差す。
/// `ShioriWiring::Custom` は複製できないので値渡しで受ける。
pub struct Emo2BootInputs {
    /// ゴーストの根。
    pub ghost_root: PathBuf,
    /// バルーンの根。
    pub balloon_root: PathBuf,
    /// SHIORI の結線（本番 `Helper`・テスト `Custom`）。
    pub shiori: ShioriWiring,
    /// ticker の起動方式（本番 `Real`・テスト `Disabled`）。
    pub ticker: TickerMode,
    /// アプリのプロファイルの置き場（本番 `Some`・テスト `None`）。
    pub app_profile_dir: Option<PathBuf>,
    /// 起動の由来（ふつう・切替で来た・前回落ちた）。`areka_ghost::boot_with_origin` へ渡る
    /// （`GhostBootOptions` に欄を足さない・要件 8.6）。
    pub boot_origin: BootOrigin,
}

/// M-boot の scope 集合を導出する（design DD-12・line 460「placement と同じ入力から自前導出」）。
///
/// emo2 fixture は sakura（scope0）＋kero（scope1）の 2 scope 構成であり、placement の
/// `detect_scopes`（scope0 常設・`kero.*` 存在で scope1）も emo2 に対し `[0, 1]` を返す。
/// placement の実結果は `ghost_session::prepare_ghost_windows` の async クロージャへ move 済みで同期参照不能
/// （DD-12）ゆえ、本関数が同じ scope 集合を独立に導出する。導出の二元性（placement の動的
/// `detect_scopes` との厳密一致は取らない）は M1 受容トレードオフとして増分申し送り（design
/// line 460）。窓と資産の不一致は [`frame::plan_attachments`]（DD-12）が missing/unused へ分類し
/// 優しく縮退するため、多少のズレでも hang/crash しない。
fn derive_scopes() -> Vec<u32> {
    vec![0, 1]
}

/// [`build_boot_assets_with_shell`] へ作者基準 DPI を供給する**唯一の呼び出し点**
/// （areka-P0-emo-dpi-scaling task 4.3・design Flow 3 手順1/5）。
///
/// `build_boot_assets(.., shell_author_dpi, balloon_author_dpi)` は**隣接する 2 つの `u16`**
/// を取り、入れ替えてもコンパイルが通る——通ったまま「シェルだけバルーンの縮尺で描かれる」
/// 静かな誤表示になる（`frame::AuthorDpis` が attach 側で防ぐのと同じ罠が、こちらは
/// 構築側にある）。名前付きフィールドを持つ [`AuthorDpi`] から**この 1 箇所だけ**で
/// 位置引数へ落とし、その 1 箇所を檻に入れる（`wire_emo2_boot` は生の `u16` を触らない）。
/// `shell` は起動のシェル（`None` は既定のシェル・areka-P0-shell-balloon-switch 要件 6.4）。
fn build_boot_assets_for(
    ghost_root: &Path,
    balloon_root: &Path,
    scopes: &[u32],
    author_dpi: AuthorDpi,
    shell: Option<&str>,
    default_surfaces: &DefaultSurfaces,
) -> Result<BootAssets, BootWiringError> {
    build_boot_assets_with_shell(
        ghost_root,
        balloon_root,
        scopes,
        author_dpi.shell,
        author_dpi.balloon,
        shell,
        default_surfaces,
    )
}

/// [`BootWiringError`] を「起点不在（良性・`warn!`）」と「それ以外（予期しない・`error!`）」へ
/// 分類して log-first 観測する（design「Error Categories and Responses」構築時・R7.3）。
///
/// 起動前の解決（`boot_config::resolve_boot`）が `ghost/master/descript.txt` の実在を確かめてから
/// 結線するので、`MountError::StartPointMissing` は解決後の消失（起動中の削除等）に限られ
/// `warn!` どまり。他（読取不能／bake／
/// balloon／`spawn_ui` 失敗等）は真に予期しない失敗として `error!` で区別する（main の
/// `is_benign_boot_error` と同じ分類方針）。boot 自体の
/// `GhostBootError` は別途 [`crate::is_benign_boot_error`]（R7.4）が分類する。
fn classify_wiring_error(err: &BootWiringError) {
    match err {
        // 解決後の消失（起動中の削除等）に限られる起点不在は warn どまり。
        BootWiringError::Mount(MountError::StartPointMissing { .. }) => {
            warn!(
                error = %err,
                "emo2-boot: 構築入力の起点が見つかりません（実 sink 結線は不成立・帰結は呼び手が記録・R7.3）"
            );
        }
        // 他は真に予期しない失敗として error（読取不能／bake／balloon／spawn_ui 等）。
        _ => {
            error!(
                error = %err,
                "emo2-boot: 構築入力の組立に失敗（実 sink 結線は不成立・帰結は呼び手が記録・R7.3）"
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
/// 1. [`build_boot_assets_for`]（`scopes` は [`derive_scopes`] で placement と同じ入力から自前導出・
///    DD-12。作者基準 DPI は引数 `author_dpi`＝placement の準備が読んだ値をそのまま搬送・
///    task 4.3。シェルは引数 `shell`＝配置の準備が決めた起動のシェルで、手順5 の起動へも
///    同じ値を渡す・areka-P0-shell-balloon-switch 要件 6.4）。`Err` は [`classify_wiring_error`]（起点不在＝`warn!`・他＝`error!`・R7.3）の上
///    `wired=false` を呼び手へ返す（`ghost_session::boot_ghost` は `LogSink`×2 フォールバック boot へ
///    倒し、`ghost_session::boot_ghost_strict` は `Err` を返す）。
/// 2. [`EmoPresenter::new`]／[`TextLayerRuntime::new`]（`Rc<RefCell<>>`）／[`spawn_emo_text`]
///    （UI スレッド前提。`Err` は [`BootWiringError::SpawnUi`] 分類＋`wired=false`）。
/// 3. [`TalkClock::new`]（`dola::runtime::clock::now` 注入）／[`ClockedTextSink::new`]。
/// 4. `mpsc::channel::<PresentCommand>()`／[`PresentBridge::new`]（差し替えの荷物の置き場を
///    `Emo2Wiring` と共有）／[`spawn_seriko_clocked`]（時計 `seriko_clock` は刻みと共有）。
///    **SurfaceResolver 突き合わせ（Task 4.1 申し送り）**: `resolver`（非 Clone）は `spawn_seriko_clocked`
///    が値消費し、`static_binds`（Clone）は clone して seriko へ渡す。`Emo2Wiring` が保持する
///    `BootAssets` の `resolver` は attach で読まれないため無害なプレースホルダ（空 alias 表）で埋める。
/// 5. 受け口の送出端の束（`CueSinkSources`）を組み、実行系の側の起動 `runtime_boot::boot_runtime`
///    （束の複製から 12 本の sink を組んで `areka_ghost::boot_with_origin`・成立後にだけ受信端を据える）を呼ぶ。
///    `shiori`／`ticker`／`app_profile_dir` は [`Emo2BootInputs`] の値＝本番は `Helper`／`Real`。`Err` は既存 [`crate::is_benign_boot_error`]（R7.4）で
///    分類（起点不在＝`warn!`・他＝`error!`）＋`wired=false`。停止通知の送出端は
///    引数 `kanade_stop`（`main` が作った channel の送出端）をそのまま渡す（ここでは channel を
///    作らない・受け口の据え付けは [`wire_kanade_stop`]）。
/// 6. [`Emo2Wiring`] を組み、shell 設定由来の重なりの基底を据えてから（`zorder_descript`＝
///    placement の準備が読んだ `seriko.zorder` の値・要件 5.1／5.2）NonSend 挿入（毎フレームの相の
///    `Update` への登録は [`register_emo2_frame_system`] が `ghost_session::register_systems` から
///    プロセスに 1 回行う・self-gating）。成立時
///    `info!`「wire 成立」マーカーを発火（実 fixture smoke＝task 7.1 がこの存在を assert する）。
/// 7. `Emo2BootOutcome{ ghost: Some, seriko: Some, wired: true }` を返す。
///
/// # 失敗（log-first・panic しない・R7.3）
/// いずれの手順の失敗も `warn!`／`error!`＋`Emo2BootOutcome{ ghost: None, seriko: None,
/// wired: false }` を呼び手へ返す（`ghost_session::boot_ghost` は `LogSink`×2 boot（既存 smoke 温存）へ倒し、
/// `ghost_session::boot_ghost_strict` は `Err` を返す）。boot 成立後の
/// spawn 済み seriko は、boot 失敗時 `surface_sink` drop で inbox 切断→worker 自然終了ゆえ handle を
/// drop（非 RAII・detach）して打ち切る（hang させない）。
pub fn wire_emo2_boot(
    world: &mut World,
    inputs: Emo2BootInputs,
    author_dpi: AuthorDpi,
    zorder_descript: Option<&str>,
    kanade_stop: Sender<KanadeNotice>,
    shell: Option<&str>,
    default_surfaces: &DefaultSurfaces,
) -> Emo2BootOutcome {
    /// 実 sink 結線が成立しなかった結果（`wired=false`・帰結は呼び手が決める: `boot_ghost` は `LogSink`×2
    /// boot へ倒し、`boot_ghost_strict` は `Err` を返す・R7.3）。
    fn fallback() -> Emo2BootOutcome {
        Emo2BootOutcome {
            ghost: None,
            seriko: None,
            wired: false,
            loop_ticker: None,
            seriko_sink: None,
            sources: None,
        }
    }

    // 手順1: 構築入力の一括組立（scopes は placement と同じ入力から自前導出・DD-12）。
    // 失敗（fixture 不在等）は分類 warn/error の上 wired=false を呼び手へ返す（R7.3）。
    let scopes = derive_scopes();
    // 作者基準 DPI（design Flow 3 手順1）は placement の準備が **1 度だけ**読んだ値を
    // `main` から `ghost_session::boot_ghost` 経由で受け取る（採寸 k₀ と attach が同じ宣言を見る・task 4.3）。
    // 隣接 u16 2 引数への落とし込みは [`build_boot_assets_for`] 1 箇所に閉じる。
    // シェルは配置の準備が決めた起動のシェル（`ghost_session::BootShellChoice`）。資産と実行系の
    // 起動（手順5）へ同じ値を渡す（要件 6.4）。
    let assets = match build_boot_assets_for(
        &inputs.ghost_root,
        &inputs.balloon_root,
        &scopes,
        author_dpi,
        shell,
        default_surfaces,
    ) {
        Ok(assets) => assets,
        Err(err) => {
            classify_wiring_error(&err);
            return fallback();
        }
    };

    // 手順2: presenter／文字層ランタイム（Rc<RefCell<>>）／文字層 UI アクター（UI スレッド前提）。
    // spawn_emo_text の Err（UI スレッド外呼び出し検出等）は SpawnUi 分類＋wired=false（R7.3）。
    let presenter = EmoPresenter::new();
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let (emo_text_sink, _emo_text_join) = match spawn_emo_text(Rc::clone(&runtime)) {
        Ok(pair) => pair,
        Err(err) => {
            classify_wiring_error(&BootWiringError::SpawnUi(err));
            return fallback();
        }
    };

    // 手順3: talk 時刻源（dola clock 注入）。時刻付与テキスト sink は実行系の側が束から組む。
    let clock_fn: Arc<dyn Fn() -> f64 + Send + Sync> = Arc::new(dola::runtime::clock::now);
    let clock = TalkClock::new(clock_fn);

    // 手順4: mpsc＋PresentBridge→spawn_seriko_clocked。SurfaceResolver 突き合わせ（!Clone・Task 4.1 申し送り）:
    // resolver（非 Clone）は spawn_seriko_clocked が値消費・static_binds（Clone）は clone して seriko と
    // Emo2Wiring の双方へ配る。Emo2Wiring 側 BootAssets の resolver は attach で読まれない（Task 4.1）
    // ため空 alias 表のプレースホルダで埋める（実 resolver は seriko が保持）。
    let (tx, rx) = std::sync::mpsc::channel::<PresentCommand>();
    // 差し替えの荷物の置き場（areka-P0-shell-balloon-switch task 9.1）: 1 つ作り、表示の橋渡しに
    // 持たせ、Emo2Wiring には弱い参照だけを渡す（置き場を強く持つのは橋渡しだけ＝seriko が倒れれば
    // 荷物の返信の送り手も消える。seriko の送り手は Emo2Wiring に持たせない）。
    let swap_slot = switch_assets::SwapSlot::default();
    let bridge = PresentBridge::new(tx).with_swap_slot(swap_slot.clone());
    // 受信端が窓の側にある 3 本の線（PresentBridge と同型の配線）: talk スレッドの受け口が送出端、
    // UI スレッドの Emo2Wiring が受信端を持つ。送出端は束（`CueSinkSources`）に載り、実行系を
    // 起こすたびに複製されて受け口になる（`runtime_boot::runtime_sinks`）。
    // - move（task 9.1）: frame 相 drain＝task 9.2 が消費。
    // - 表示ライフサイクル（task 4.1）: 会話の開始と、待機を含む占有区間の終端が talk 相対秒で
    //   届き、タイムアウト計測の破棄と起点になる（Requirements 4.1／4.5・design 決定 D4＝α）。
    // - 重なり（areka-P0-scope-zorder-pinning task 6.2）: 解釈前のトークン列を運び、意味付けは
    //   台帳の状態を要するので取り出しの相（run_zorder_drain_phase）が行う。
    let (move_tx, move_rx) = std::sync::mpsc::channel::<PositionCue>();
    let (lifecycle_tx, lifecycle_rx) = std::sync::mpsc::channel::<TalkLifecycleSignal>();
    let (zorder_tx, zorder_rx) = std::sync::mpsc::channel::<ZOrderDirective>();
    let BootAssets {
        shells,
        balloons,
        resolver,
        static_binds,
        bind_resolver,
        loop_tables,
        shell_author_dpi,
        balloon_author_dpi,
        boxes,
        cursors,
    } = assets;
    // SERIKO ループ構成（design「本番は実時間・実 entropy 接続」・R7.4）: シェル／バルーンの 2 表は
    // `BootAssets.loop_tables`（task 9.1 が `EmoWorld` スナップショットから `from_world` で構築）を
    // 値移送し、乱数は実 entropy から。seed は `std::hash::RandomState`（プロセスごとにランダム鍵で
    // 初期化される `BuildHasher`）から u64 を実現する（新規 crates.io 依存なし・design 準拠）。空書込みの
    // `finish()` はその hasher のランダム鍵由来値を返すため、実行ごとに異なる seed が得られる。
    let LoopTables {
        shell: shell_table,
        balloon: balloon_scope_tables,
    } = loop_tables;
    // バルーン表の転送（要件 5.6）: `LoopTables.balloon` は既に scope 別の写像（`build_boot_assets` の
    // 構築ループ内で単一導出済み）ゆえ、ここは scope キーをアクタ鍵語彙へ写す値移送だけを行う。
    let balloon_tables = actor_keyed_balloon_tables(balloon_scope_tables);
    let seed: u64 = {
        use std::hash::{BuildHasher, Hasher};
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    };
    // 再現可能性の観測点（R7.4/7.5）: 本番 seed を info! に出力し、同一 seed で run を再現可能にする。
    info!(
        seed,
        "emo2-boot: SERIKO ループ乱数 seed（RandomState 由来・実 entropy・再現用）"
    );
    let loop_config = SerikoLoopConfig {
        shell_table,
        balloon_tables,
        rng: seeded_rng(seed),
    };
    // boot は S: dola::cue::CueSink + Clone を要求する。SerikoSink は upstream `areka-seriko` で
    // `CueSink` を実装し `#[derive(Clone)]` 済み（内側 mpsc::Sender は常に Clone・全 clone は単一 inbox
    // 送信端で配送同一）ゆえ spawn_seriko_clocked の戻り値を直接 surface sink として束へ載せる（共有 shim は不要）。
    // `bind_resolver`（BootAssets・task 7.1 が MountModel の名前転記から構築した実名前解決表）を
    // seriko の起動へ値渡しで配線する。bind cue（`\![bind]`）の着せ替え名解決はこの実表が担う（task 7.2）。
    // `loop_config`（上で組んだ実表＋実 entropy 乱数）を seriko へ値渡しし、SERIKO ループ統括器を起こす。
    // 刻みと seriko が読む 1 つの時計（spec: areka-P0-animated-image-playback 要件 1.6・2.9）。
    // 刻みの既定と同じ `GetTickCount64`（ミリ秒）。台本の時計 `clock`（`TalkClock`）とは別の値。
    let seriko_clock: SerikoClock = Arc::new(tick_count_ms);
    let (surface_sink, seriko_handle) = spawn_seriko_clocked(
        resolver,
        static_binds.clone(),
        bind_resolver.clone(),
        loop_config,
        bridge,
        Some(Arc::clone(&seriko_clock)),
    );
    let wiring_assets = BootAssets {
        shells,
        balloons,
        // attach は resolver を一切読まない（Task 4.1 申し送り）ため無害なプレースホルダ。
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds,
        // 実 bind_resolver の写し: attach は ghost の読み直しで持ち越す着せ替えの名前を引くのにだけ使う
        // （areka-P0-mcp-reload 要件 3.8）。名前の解決そのものは seriko の写しが担う。
        bind_resolver,
        // 実 loop_tables は loop_config へ移送済み（attach は loop_tables を読まない）ため空表プレースホルダ。
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
        // 作者基準 DPI は搬送のみ（本相は値を解釈しない・attach への供給は task 4.2）。
        shell_author_dpi,
        balloon_author_dpi,
        // 箱の束は装着の相が文字の層へ渡す（seriko は読まない）。
        boxes,
        // シェルのカーソルも装着の相が箱の束の隣で入れる。
        cursors,
    };

    // loop ticker 用の tick 送出端: SerikoSink を 1 本 clone して保持する。全 clone は単一 seriko
    // inbox への送信端で配送意味は同一（task 9.2）。boot 失敗時はこの clone が inbox を生かし続けない
    // よう明示 drop する（worker 自然終了・下記）。
    let tick_sink = surface_sink.clone();
    // セッションへ返す複製（シェル・バルーンの差し替えの口・areka-P0-shell-balloon-switch 要件 8.5）。
    // 持ち主は `GhostSession` だけで、降ろす最初の段で落ちる。boot 失敗時は下で tick_sink と一緒に落とす。
    let session_sink = surface_sink.clone();
    // 受け口の送出端の束（areka-P0-mcp-reload 要件 4.2）: 窓の側がここで 1 度組み、実行系を
    // 起こすたびに複製から 12 本の sink を組む。
    let sources = CueSinkSources {
        surface: surface_sink,
        text: emo_text_sink,
        clock: clock.clone(),
        move_tx,
        lifecycle_tx,
        zorder_tx,
    };

    // 手順5: 実行系の側の起動（`runtime_boot::boot_runtime`）。sink の並び・boot の失敗の分類と記録・
    // 受信端が窓の側に無い線（readme・中断・ghost の切替・shell/balloon の切替・読み直し）の
    // 据え付けは向こうが持ち、Err のとき World は変わっていない。停止通知の送出端は引数
    // `kanade_stop`（`main` が作った channel の送出端）をそのまま渡す（受け口の据え付けは
    // [`wire_kanade_stop`]）。これが在ることで、終了挨拶を再生し終えてから窓が閉じる。
    let Ok(ghost_runtime) = runtime_boot::boot_runtime(world, inputs, &sources, kanade_stop, shell)
    else {
        // spawn 済み seriko の後始末: tick 用 clone（tick_sink）・セッションへ返す複製（session_sink）・
        // 束（sources）が inbox を生かし続けるため、ここで明示 drop する。全 Sender が消えると inbox
        // 切断→worker 自然終了する。ActorHandle は非 RAII（drop で detach）ゆえ join せず drop で
        // 打ち切る（万一 boot が dispatcher へ move 後に失敗しても hang しない）。
        drop(tick_sink);
        drop(session_sink);
        drop(sources);
        drop(seriko_handle);
        return fallback();
    };

    // 手順6: Emo2Wiring を NonSend 挿入（emo2_frame_system の登録は [`register_emo2_frame_system`]）。
    // 受け取った bevy World へ直接載せる。
    let mut wiring = Emo2Wiring::new(
        presenter,
        rx,
        move_rx,
        lifecycle_rx,
        zorder_rx,
        runtime,
        clock,
        wiring_assets,
    );
    wiring.swap_slot = std::sync::Arc::downgrade(&swap_slot);
    // shell 設定（`seriko.zorder`）由来の基底を、World へ載せる前に据える（要件 5.1／5.2／
    // 5.3／5.4・areka-P0-scope-zorder-pinning task 6.3）。ここはまだこの結線状態で走る
    // 最初の `Update` の手前であり、取り出しの相も 1 度も走っていない——ゆえに基底は**タグの実行を待たずに**
    // 最初の維持の巡から効く。解釈できない値は理由とともに記録され、グループを 1 本も
    // 載せずに起動が続く（この呼出は失敗を返さない）。
    wiring.seed_zorder_descript_base(zorder_descript);
    world.insert_non_send(wiring);

    // SERIKO ループ ticker 起動（design「本番は実時間・実 entropy 接続」・R7.4）: 16ms で、seriko と
    // 同じ時計 `seriko_clock`（[`loop_ticker_config`]）で駆動し、各 Tick を tick_sink（SerikoSink クローン）経由で seriko
    // へ届ける。ghost の loop ticker は seriko を一切知らず、クロージャがその継ぎ目（依存方向: areka が
    // ghost の spawn_loop_ticker を seriko の SerikoSink へ結線・ghost→seriko 依存は張らない）。
    // Tick.now は MonotonicMs（内側 u64・ms）ゆえ `.0` を send_tick へ渡す。
    let (loop_ticker_stop, _loop_ticker_handle) = spawn_loop_ticker(
        loop_ticker_config(&seriko_clock),
        Box::new(move |tick: Tick| {
            tick_sink.send_tick(tick.now.0);
        }),
    );

    // wire 成立マーカー（実 fixture smoke＝task 7.1 がこの info! の存在を assert する・R7.3 観測境界）。
    info!("emo2-boot: 実 sink 結線が成立しました（wire 成立）");

    // 手順7: 実 sink 結線成立。ghost/seriko ハンドル＋loop ticker 停止端＋束を呼び手へ返す。
    Emo2BootOutcome {
        ghost: Some(ghost_runtime),
        seriko: Some(seriko_handle),
        wired: true,
        loop_ticker: Some(loop_ticker_stop),
        seriko_sink: Some(session_sink),
        sources: Some(sources),
    }
}

/// 刻みの既定（`areka-ghost` の `LoopTickerConfig::default`）と同じ時刻の基準＝OS 起動からの経過ミリ秒。
fn tick_count_ms() -> u64 {
    // SAFETY: GetTickCount64 は引数もポインタも取らないカウンタの読み取りで、前提条件はない。
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

/// SERIKO ループの刻みの構成を、seriko と同じ時計から組む（周期は既定のまま）。
fn loop_ticker_config(clock: &SerikoClock) -> LoopTickerConfig {
    let clock = Arc::clone(clock);
    LoopTickerConfig {
        clock: Box::new(move || MonotonicMs(clock())),
        ..LoopTickerConfig::default()
    }
}

/// 毎フレームの相（`emo2_frame_system`）を `Update` へ登録する（self-gating）。
///
/// 相の登録先は `Update`——上流の `Update` 鎖（表示構成の検知 → モニタ表の更新 → 依存する
/// 部品の無効化 → 文字送りの更新）の**最後の系より後**という 1 点だけを指定する
/// （裁定 2026-09-12・要件 2.4）。1 巡のスケジュールは
/// `Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup → Draw
///  → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize`
/// の順で回るので、相がここで挿した描画命令と配置は、同じ巡の伝播（`PostLayout`）・面の
/// 生成（`PreRenderSurface`）・描画（`RenderSurface`）が拾う。末尾の段に載せると絵の着地が
/// 次の巡へずれ、文字だけが 1 コマ先に出る。
/// 重なりの鎖の適用系（末尾の段の 3 本目）との前後は、`.before` ではなくこの相順そのものが
/// 担う（段をまたぐ順序指定は書けない）——`Update` は末尾の段より前なので、相が組んだ
/// 望む鎖は同じ巡のうちに適用系が読む（要件 14.5 の「そのイベントへの応答としての完了」）。
/// 上流の鎖より後に置くのは、拡大率の相が読むモニタ表がその巡の最新であるためで、DPI 遷移の
/// 原子性（完了 spec `dpi-transition-atomicity`「モニタ表更新 → emo の相 → 窓書込の flush」）を
/// 移動の前後で変えない。
pub fn register_emo2_frame_system(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, emo2_frame_system.after(update_typewriters));
}

/// 運行の通知（停止ほか）の受け口を World に据え、終了相を `Update` に登録する（受け口はプロセスに 1 つ・
/// 要件 6.4）。
///
/// 呼び手は `ghost_session::register_systems` で、プロセスに 1 回、[`register_emo2_frame_system`] の
/// 直後に呼ぶ。終了相は毎フレームの相より前に走る。順序の相手（毎フレームの相）が登録されていない
/// World でも登録は通る（`frame_schedule_tests` の 1 本がこの形を固定する）。
pub fn wire_kanade_stop(world: &mut World, rx: Receiver<KanadeNotice>) {
    world.insert_non_send(KanadeNoticeRx(rx));
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, ghost_quit_system.before(emo2_frame_system));
}

#[cfg(test)]
mod wire_tests {
    use super::sample_test_support::{emo2_balloon_root, emo2_root};
    use super::*;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    /// 取り違え防止の檻（task 4.3・要件 1.1）: `build_boot_assets` への隣接 `u16` 2 引数
    /// （`shell_author_dpi`／`balloon_author_dpi`）が [`AuthorDpi`] の各フィールドと
    /// **正しく対応**していることを、意図的に**異なる 2 値**で固定する。
    ///
    /// 入れ替えは型検査を通り抜けるため、この檻だけが誤りを検出できる
    /// （192 と 144 を入れ替えると両アサーションが落ちる）。
    /// `wire_emo2_boot` 本体は生の `u16` を触らず、この関数だけが位置引数へ落とす。
    #[test]
    fn build_boot_assets_for_pairs_shell_and_balloon_author_dpi() {
        // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE 等は無視・assets.rs 同流儀）。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let assets = build_boot_assets_for(
            &emo2_root(),
            &emo2_balloon_root(),
            &[0, 1],
            AuthorDpi {
                shell: 192,
                balloon: 144,
            },
            None,
            &DefaultSurfaces::builtin(),
        )
        .expect("emo2 fixture の BootAssets 組立は成功する");

        assert_eq!(
            assets.shell_author_dpi, 192,
            "shell 引数には AuthorDpi::shell が渡る（balloon の 144 ではない）"
        );
        assert_eq!(
            assets.balloon_author_dpi, 144,
            "balloon 引数には AuthorDpi::balloon が渡る（shell の 192 ではない）"
        );
    }

    /// 観測可能な完了条件（tasks.md task 5.1）: 存在しない ghost_root に対し
    /// `wire_emo2_boot` は `wired=false`（帰結は呼び手が決める・R7.3）を返し、
    /// ghost／seriko ハンドルは `None` となる（実 sink 結線を成立させない決定分岐の檻）。
    ///
    /// 素の `World::new()` で足りる。`build_boot_assets` が `resolve` 起点不在で早期 `Err` を返すため、
    /// World には一切触れず、spawn_emo_text／spawn_seriko／boot へも到達しない（＝アクタースレッドを
    /// 起こさない）ので決定論的に判定できる。
    #[test]
    fn wire_emo2_boot_falls_back_to_unwired_on_missing_ghost_root() {
        let mut world = World::new();
        let inputs = Emo2BootInputs {
            ghost_root: PathBuf::from("C:/areka-nonexistent-ghost-root-emo2boot-5v1"),
            balloon_root: PathBuf::from("C:/areka-nonexistent-balloon-root-emo2boot-5v1"),
            shiori: ShioriWiring::Helper {
                helper_exe: PathBuf::from("shiori-host32-helper.exe"),
            },
            ticker: TickerMode::Disabled,
            app_profile_dir: None,
            boot_origin: BootOrigin::Plain,
        };

        let outcome = wire_emo2_boot(
            &mut world,
            inputs,
            AuthorDpi::DEFAULT,
            None,
            std::sync::mpsc::channel().0,
            None,
            &DefaultSurfaces::builtin(),
        );

        assert!(
            !outcome.wired,
            "存在しない ghost_root では実 sink 結線は成立せず wired=false を返す（R7.3）"
        );
        assert!(
            outcome.ghost.is_none(),
            "不成立時は ghost ランタイムを起動しない（None）"
        );
        assert!(
            outcome.seriko.is_none(),
            "不成立時は seriko アクターを起こさない（None）"
        );
    }
}
