//! 決定論 spine テストハーネスの背骨の足場（⑶）。
//!
//! 待ちの部品（回収の `settle_bounded` を含む）と `GpuPermit`（⑴）・台本の SHIORI と `hold_support`（⑵）は `areka-runtime` の `spine` にあり、
//! ここは本体の部品も使う足場（`SpineHarness`・`make_world_with_gpu` ほか）とそれを使う
//! spine のテストを持つ。
//!
//! `cargo test --workspace`（外部 CI 無し・ローカル DoD ゲート）で常設観測する、起動〜発話〜
//! 終了の全経路を **sleep 不使用**・**注入 Tick のみ**・**実 GPU（MTA）** で通す。
//!
//! **「sleep 不使用」の射程**: spine で言う sleep 不使用とは
//! **時刻を進めるために sleep しない**ことであり（時刻前進は注入 Tick のみ＝決定論の源）、
//! 待ちの反復の間の短い休みはこれに当たらない。待ちは反復回数でなく壁時計で有界にしてある
//! （反復回数のみの上限は CPU 競合下で数 ms で尽き、製品コードが正常でも偽陽性の赤を出す）。
//! 待ちの形は次のとおりで、打ち切りの決め方と休みの決まりは `areka-runtime` の `spine_wait.rs` の頭と
//! 各関数の説明にある:
//!
//! - **読むだけの待ち**（各反復が系を進めない・別スレッドの到着を読むだけ）→ [`wait_until`]。
//!   休みは芯が持つので、条件の中では眠らない。
//! - **Tick を入れながらの待ち**（毎反復 Tick を注入して系を進めつつ別スレッドの結果も待つ）→
//!   [`wait_until_paced`]（進みの目印は SHIORI の呼び出しの数・汲みが数を返す呼出点はそれと汲んだ数の和）。
//!   条件の中で Tick を入れて汲む。休みは [`wait_until_paced`] が持つので、条件の中では眠らない。
//!   加えて注入時刻が観測を追い越さないこと（頭打ち）が別途必要。
//!   時計を差し替える段の駆動器（`spine_conformance_support.rs` の `run_stage_with`）だけは芯
//!   [`wait_until_with`] を直に使い、同じ [`INJECT_PACE`] で休む。
//! - **回収（settle）**（「尽きるのが正常」の残余回収＝負検証）→ [`settle_bounded`]。時刻前進は
//!   呼出側が前段で注入し終えており、本体は観測しかしない。
//!
//! 本ファイルは本体の部品も使う足場（`SpineHarness`＝実 sink 結線＋GPU World＋frame フェーズ直接駆動）だけを持ち、
//! `#[test]` は兄弟のテストファイルに置く。
//!
//! # なぜ in-crate テストか
//!
//! `areka` は `[[bin]]` のみ（`[lib]` 無し）で、外部 `tests/` から bin 内部項目
//! （`crate::session::{hooks, runtime_boot, ghost_quit}`）へ到達できない。
//! spine は Tick 注入＋GPU readback＋frame フェーズ直接駆動を **in-process** で行う必要があり
//! （bin を起動する外部テストでは不能）、既存 repo 慣行（`areka_runtime::wire_tests`・
//! `areka_placement::spawn` のテスト）に従い **in-crate `#[cfg(test)]` モジュール** として置く。

// `spine` の代わりの親を兼ねる（⑴ ⑵ を子へ渡す）。下の `use` は ⑶ と子のテストが使う。
use areka_runtime::spine::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use areka_actor::ActorHandle;
use areka_emo_compose::{BindSet, PatternState};
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostRuntime, ShioriWiring, TickerMode, boot_with_kanade_stop};
use areka_kanade::{CloseReason, KanadeNotice, MonotonicMs, ShioriBackend};
use areka_sakura::ActorKey;
use areka_seriko::{
    AnimationTable, BindResolver, DisplayCommand, LoopRng, SerikoLoopConfig, SerikoSink,
    SurfaceOutput, SurfaceResolver, spawn_seriko,
};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};

use crate::sample_test_support::acquire_emo2;
use sample_ghost_kit::SampleRoot;
use shiori_host32_host::ExitKind;
use wintf::ecs::{DPI, GraphicsCore, Point, WindowPos, WucGraphicsResource};
use wintf::executor::JoinHandle;

use areka_placement::spawn::test_support::{titles, two_scope_placements};
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};
use areka_placement::test_support::give_fake_handles;

use crate::session::ghost_quit::KanadeNoticeRx;
use areka_runtime::adapter::PresentBridge;
use areka_runtime::assets::{
    BootAssets, LoopTables, actor_keyed_balloon_tables, build_boot_assets,
};
use areka_runtime::frame::test_support::count_level;
use areka_runtime::frame::{
    RuntimeWiring, all_shown, run_attach_phase, run_dpi_phase, run_move_drain_with, run_text_phase,
    run_text_scale_phase,
};
use areka_runtime::move_cue::{MoveCueSink, PositionCue};
use areka_runtime::talk_clock::{ClockedTextSink, TalkClock};
use areka_runtime::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};
use areka_runtime::target_map::{balloon_target, shell_target};

/// `GraphicsCore`＋`WucGraphicsResource` を実資源として載せた wintf World（headless GPU）。
///
/// 本番 UI スレッドは MTA（記憶: areka WUC は MTA スレッドで動く）。テストのスレッドは `runtime_tests/mod.rs` が据えるプロセスの寿命の MTA に居るので、ここでは COM を初期化しない。実 GPU（`GraphicsCore::new()` は D3D の HARDWARE・WARP には落ちない）。
fn make_world_with_gpu() -> (GpuPermit, World) {
    let gpu = GpuPermit::take();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    // 終了系列の完了は `quit_app` を通る。本番の `WinApp` と同じく受け口を据える
    // （無いと `app_exit_unwired` の error を残す）。
    world.insert_non_send(wintf::AppExit::new());
    (gpu, world)
}

// 汲み出しの定石は `areka_runtime::spine` に 1 つだけ置き、兄弟のテストへここから渡す。
pub(super) use areka_runtime::spine::pump_until_idle;

// ===========================================================================
// ログ捕捉（硬化機構の唯一の定義元 `log-capture-kit` への委譲）
//
// `run_attach_phase` の `info!(planned, attached, ...)`（縮退がバグを隠さない檻の
// 観測点）をこのスレッド上で捕捉して件数一致を assert する。行の形は 1 イベント 1 行、
// `level=… target=…` に続けてフィールドを訪問順で ` name=value`。
//
// 捕捉窓の機序（callsite の interest キャッシュがプロセス全体で 1 つであることと、その塞ぎ方）は
// `areka_runtime` の `frame_test_support.rs` の解説と同じで、正本は `log_capture_kit` の crate doc。
// 向こうの `capture_logs` は `cfg(test)` だけで立つため本体側からは読み込めず、ここに同じ 3 行を置く。
// ===========================================================================

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

// ===========================================================================
// SpineHarness（再利用可能な結線ハーネス・兄弟のテストがこの上に構築する）
// ===========================================================================

/// spine ハーネスの SERIKO ループ駆動モード。
///
/// spine は loop ticker を**起動しない**。ループ駆動は `SerikoLoopConfig`（表＋乱数）を seriko へ値渡しで
/// 仕込み、tick は `SerikoSink::send_tick` を直接注入して制御する（決定論・時刻は注入だけで進める）。
enum LoopDriver {
    /// ループ完全不活性（`SerikoLoopConfig::disabled()` 相当＝空表・ダミー乱数）。既存 spine 全テストの
    /// 非退行経路。send_tick を注入してもループは
    /// 表示中 slot の評価対象アニメが常にゼロ＝何も発行しない（従来観測どおり）。
    Inert,
    /// 実 emo2 表（`BootAssets.loop_tables`）＋固定注入乱数列で駆動（まばたき e2e）。
    /// 乱数は抽選と選び（`alternative*`）の 2 つ。
    Live(LoopRng, LoopRng),
}

/// 1 つの面の Show を 1 回だけ、開くまで止める門（seriko のスレッドの上で待つ・テストの足場だけ）。
///
/// 負荷のときにだけ起きる順（別スレッドから来る面の指令が `TalkEnded` より後に届く）を毎回起こす。
pub(crate) struct ShowGate {
    scope: &'static str,
    surface_id: u32,
    open: Arc<(Mutex<bool>, Condvar)>,
}

/// 門の鍵。`open` で開く。捨てても開く（テストが落ちても seriko を止めたままにしない）。
pub(crate) struct ShowGateKey(Arc<(Mutex<bool>, Condvar)>);

impl ShowGateKey {
    /// 門を開ける（何度呼んでもよい）。
    pub(crate) fn open(&self) {
        let (lock, cvar) = &*self.0;
        *lock.lock().unwrap_or_else(PoisonError::into_inner) = true;
        cvar.notify_all();
    }
}

impl Drop for ShowGateKey {
    fn drop(&mut self) {
        self.open();
    }
}

/// `scope` の面 `surface_id` の Show を 1 回だけ止める門と、その鍵を作る。
pub(crate) fn show_gate(scope: &'static str, surface_id: u32) -> (ShowGate, ShowGateKey) {
    let open = Arc::new((Mutex::new(false), Condvar::new()));
    let gate = ShowGate {
        scope,
        surface_id,
        open: Arc::clone(&open),
    };
    (gate, ShowGateKey(open))
}

/// `PresentBridge` を包む出力。門が無ければ今と同じ（そのまま渡す）。
struct GatedOutput {
    inner: PresentBridge,
    gate: Option<ShowGate>,
}

impl SurfaceOutput for GatedOutput {
    fn send(&mut self, command: DisplayCommand) {
        if let DisplayCommand::Show {
            scope, surface_id, ..
        } = &command
            && let Some(gate) = &self.gate
            && scope.as_str() == gate.scope
            && *surface_id == gate.surface_id
        {
            let (lock, cvar) = &*gate.open;
            let open = lock.lock().unwrap_or_else(PoisonError::into_inner);
            drop(
                cvar.wait_while(open, |open| !*open)
                    .unwrap_or_else(PoisonError::into_inner),
            );
            self.gate = None;
        }
        self.inner.send(command);
    }
}

/// scripted ghost boot＋実 sink 結線＋GPU World＋frame 結線状態を束ねた spine ハーネス。
///
/// `wire_runtime`（production）の結線を、headless GPU World＋合成 `GhostWindows`＋
/// scripted backend＋`TickerMode::Disabled` で**テスト内 in-process** に再現したもの。frame
/// フェーズ（`run_attach_phase` 等）を直接駆動でき、`ghost`/`seriko`/`shiori_handle` で
/// ライフサイクル・発火列を観測できる。
pub(crate) struct SpineHarness {
    /// GPU 資源（`GraphicsCore`/`WucGraphicsResource`）＋合成 `GhostWindows`＋装着スロットを持つ World。
    world: World,
    /// frame 三相結線状態（presenter/rx/runtime/clock/assets）。`run_attach_phase` の駆動対象。
    wiring: RuntimeWiring,
    /// 文字層ランタイムの観測用クローン（兄弟のテストが `present_frame`/`read_back` に使う）。
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// ghost ランタイム（`dispatcher()` への Tick 注入・`shutdown()` の駆動）。
    ghost: GhostRuntime,
    /// seriko worker の join ハンドル（有界 join で終了確認）。
    seriko: ActorHandle,
    /// scripted backend の発火列観測ハンドル（boot 系列・close 系列の照合）。
    shiori_handle: ScriptedShioriHandle,
    /// 文字層 UI アクター（`spawn_emo_text` の pump アクター）の生存ハンドル（drain は `pump_text`）。
    text_pump: JoinHandle<()>,
    /// SERIKO ループ tick の直接注入端。`spawn_seriko` が返す `SerikoSink` の clone で、
    /// `inject_seriko_tick`（`send_tick` 直接注入・loop ticker 不起動）に使う。surface_sink 本体は
    /// boot が sinks 第 1 要素として値消費するため、この clone をハーネスに保持する。全 clone は単一 seriko
    /// inbox への送信端で配送意味は同一。shutdown 時に drop して inbox 切断→worker 自然終了させる。
    tick_sink: SerikoSink,
    /// この起動へ配られた検体の使い捨ての複製。値が木の寿命そのものなので、
    /// 起動したものより後に捨てる順（欄の最後・後片付けでも最後）に置いてある。
    ///
    /// これは**用心であって、裏付けるテストは無い**。順を入れ替えても今のテストは全て
    /// 緑になる——終了処理が書く永続化は失敗しても捨てられ、どのテストも読まないためで
    /// ある。順を守る理由は、木が在るうちに畳むほうが、後から観測を足したときに驚きが
    /// 少ないことだけである。
    sample: SampleRoot,
    /// GPU の装置の許可（[`GpuPermit`]）。World の装置より後に返すので欄の最後。
    gpu: GpuPermit,
}

impl SpineHarness {
    /// 標準台本（boot 系列＋`\s[0]\e` OnBoot＋ForceQuit close 系列）で spine ハーネスを起動する。
    ///
    /// `on_boot` は OnBoot GET が返す応答スクリプト。足場の煙テストは最小 talk（`\s[0]\e`）を渡す。
    /// SERIKO ループは **不活性**（`LoopDriver::Inert`）で駆動する——既存 spine 全テストの非退行経路。
    /// ループを実表で活性化する SERIKO ループの檻は [`Self::boot_live`]。
    fn boot(on_boot: &str) -> SpineHarness {
        let (backend, shiori_handle) = Self::standard_backend(on_boot);
        Self::boot_with(backend, shiori_handle, LoopDriver::Inert)
    }

    /// 実 emo2 表＋固定注入乱数列で SERIKO ループを**活性化**して spine ハーネスを起動する。
    ///
    /// `rng` は 1/N 抽選の固定注入列（決定論・実 entropy 非依存）、`pick_rng` は選びの注入列。tick は loop ticker を起動せず
    /// [`Self::inject_seriko_tick`]（`send_tick` 直接注入）で制御する（時刻は注入だけで進める）。まばたき e2e が
    /// 使う（実 kero/sakura まばたき 1 周の full golden は `spine_seriko_loop_tests.rs`）。
    fn boot_live(on_boot: &str, rng: LoopRng, pick_rng: LoopRng) -> SpineHarness {
        let (backend, shiori_handle) = Self::standard_backend(on_boot);
        Self::boot_with(backend, shiori_handle, LoopDriver::Live(rng, pick_rng))
    }

    /// 標準 scripted backend（boot 系列＋OnBoot talk＋ForceQuit close 系列）を組む。
    ///
    /// boot 系列（OnInitialize→[username prefetch]→OnFirstBoot→OnBoot→[OnTranslate]→basewareversion）＋ shutdown
    /// （`GhostRuntime::shutdown` は常に ForceQuit 経路＝OnClose NOTIFY→Unload・ghost spine S1 と同旨）を
    /// 台本化する。OnSecondChange は kanade へ Tick を送らないため不要。
    /// username prefetch GET（OnInitialize 後・OnFirstBoot 前）は `build()` が
    /// 既定 `Ok(None)`（NoContent＝カスタム username 無し）を自動補填するため明示台本化しない。OnTranslate の GET も、台本に無ければ偽の SHIORI が 204 で答える。
    pub(crate) fn standard_backend(on_boot: &str) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(on_boot.to_string())))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
            .build()
    }

    /// 任意の scripted backend＋ループ駆動モードで spine ハーネスを起動する（兄弟のテストが独自台本を注入する口）。
    fn boot_with(
        backend: ScriptedShioriBackend,
        shiori_handle: ScriptedShioriHandle,
        driver: LoopDriver,
    ) -> SpineHarness {
        Self::boot_with_gate(backend, shiori_handle, driver, None)
    }

    /// [`Self::boot_with`] に、seriko の出力の門（[`show_gate`]）を足して起動する。
    fn boot_with_gate(
        backend: ScriptedShioriBackend,
        shiori_handle: ScriptedShioriHandle,
        driver: LoopDriver,
        gate: Option<ShowGate>,
    ) -> SpineHarness {
        // ── headless GPU World（実 GPU）＋合成 GhostWindows（scope [0,1]） ──
        let (gpu, mut world) = make_world_with_gpu();
        spawn_ghost_windows(&mut world, &two_scope_placements(), &titles());
        // 本体の口を据える（本番の `ghost_session::register_systems` と同じ口）。
        crate::session::hooks::install(&mut world);

        // ── 構築入力（実 emo2 検体・COM はプロセスの寿命の MTA（runtime_tests/mod.rs）） ──
        // 検体はこの起動専用に取得する。起動記録の無い新品が配られるので、同じ
        // プロセスで何度起動しても毎回 OnFirstBoot から始まる台本が成り立つ。
        // 作者基準 DPI は emo2 検体の実測既定（shell/balloon とも無宣言＝96）。
        let sample = acquire_emo2();
        let assets = build_boot_assets(
            sample.folder(),
            sample
                .balloon("emo2-kakukaku")
                .expect("emo2 の同梱バルーン"),
            &[0, 1],
            96,
            96,
        )
        .expect("emo2 検体の BootAssets 組立は成功する");

        // ── presenter／文字層ランタイム／実 EmoTextSink（テストスレッド＝UI pump スレッド） ──
        let presenter = EmoPresenter::new();
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (emo_text_sink, text_pump) = spawn_emo_text(Rc::clone(&runtime));

        // ── talk 時刻源（本番同様 dola clock 注入）＋実 ClockedTextSink<EmoTextSink> ──
        let clock_fn: Arc<dyn Fn() -> f64 + Send + Sync> = Arc::new(dola::runtime::clock::now);
        let clock = TalkClock::new(clock_fn);
        let clocked_text_sink = ClockedTextSink::new(emo_text_sink, clock.clone());

        // ── mpsc＋PresentBridge→spawn_seriko（実 SurfaceOutput 経路）。SurfaceResolver は
        //    非 Clone ゆえ spawn_seriko が値消費し、RuntimeWiring 側は空 alias のプレースホルダで埋める
        //    （attach は resolver を読まない・wire_runtime 手順4 と同型）。 ──
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let bridge = PresentBridge::new(tx);
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
        // SERIKO ループ構成: 実 emo2 表（`BootAssets.loop_tables`＝`EmoWorld` スナップショットから
        // `from_world` で構築）＋固定注入乱数列（`driver`）で `loop_config` を組む。既存 spine テストは
        // Inert（空表・ダミー乱数）でループ完全不活性。実表＋固定 rng で駆動するのは `boot_live` の
        // 檻だけ（本番の `wire_runtime` は実 entropy）。
        let LoopTables {
            shell: shell_table,
            balloon: balloon_scope_tables,
        } = loop_tables;
        // バルーン表の転送: production `wire_runtime` と同型で、`build_boot_assets` が
        // scope ごとに導出済みの写像を、scope キーだけアクタ鍵語彙へ写して渡す（値移送）。
        let balloon_tables = actor_keyed_balloon_tables(balloon_scope_tables);
        let loop_config = match driver {
            LoopDriver::Inert => SerikoLoopConfig::disabled(),
            LoopDriver::Live(rng, pick_rng) => SerikoLoopConfig {
                shell_table,
                balloon_tables,
                rng,
                pick_rng,
            },
        };
        // 実名前解決表（BootAssets.bind_resolver・emo2 fixture の MountModel から構築）＋
        // loop_config を seriko の起動へ値渡しで配線する（production wire_runtime と同型）。
        let (surface_sink, seriko) = spawn_seriko(
            resolver,
            static_binds.clone(),
            bind_resolver,
            loop_config,
            GatedOutput {
                inner: bridge,
                gate,
            },
        );
        // loop tick 直接注入端: SerikoSink を 1 本 clone してハーネスへ保持する（欄 `tick_sink` の説明を参照）。
        let tick_sink = surface_sink.clone();
        let wiring_assets = BootAssets {
            shells,
            balloons,
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds,
            // 実 bind_resolver は seriko が値消費済み（attach は bind_resolver を読まない）ため空表プレースホルダ。
            bind_resolver: BindResolver::empty(),
            // 実 loop_tables は loop_config へ移送済み（attach は loop_tables を読まない）ため空表プレースホルダ。
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: BTreeMap::new(),
            },
            // 作者基準 DPI は搬送のみ（本相は値を解釈しない）。
            shell_author_dpi,
            balloon_author_dpi,
            boxes,
            cursors,
        };

        // ── move channel＋実 MoveCueSink（wire_runtime 手順4 と同型） ──
        // talk スレッドの MoveCueSink が送出端、UI スレッドの RuntimeWiring が受信端 move_rx（frame 相
        // drain＝run_move_drain_phase）を保持する。送出端は throwaway でなく実 MoveCueSink を
        // 登録する。本番の sink（`runtime_boot.rs` の `runtime_sinks`）のうち、spine は先頭 4 本（surface・text・
        // move・lifecycle）だけを同じ順で渡す（Z 順・readme・切替・読み直し・窓の状態などの残りは渡さず、その cue は
        // 届け先が無い）。
        let (move_tx, move_rx) = mpsc::channel::<PositionCue>();
        let move_sink = MoveCueSink::new(move_tx);

        // ── lifecycle channel＋実 BalloonLifecycleSink（wire_runtime 手順4 と同型） ──
        // talk スレッドの BalloonLifecycleSink が送出端、UI スレッドの RuntimeWiring が受信端
        // lifecycle_rx（バルーン可視性相が drain）を保持する。本番の 4 本目と同じ実 sink を
        // throwaway でなく登録する。
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let lifecycle_sink = BalloonLifecycleSink::new(lifecycle_tx, clock.clone());

        // ── scripted boot（実 sink 注入・TickerMode::Disabled＝Tick 注入で駆動） ──
        // sinks は broadcast 登録先で surface（seriko）／text（ClockedTextSink）／move（MoveCueSink）／
        // lifecycle（BalloonLifecycleSink）の 4 sink を第 1〜4 要素として渡す（本番の `runtime_sinks` のうち先頭 4 本だけを同じ順で・残りは渡さない）。
        // 起動の選択肢は本番の窓の在るゴーストと同じ組み立て（strict の判定・FromSylphya・Ansi）を使う。
        // App スコープ root は不要（None＝App 層不在縮退・ghost/shell スコープは emo2 mount 由来）。
        let options = crate::session::runtime_boot::runtime_boot_options(
            sample.folder().to_path_buf(),
            ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            vec![
                Box::new(surface_sink),
                Box::new(clocked_text_sink),
                Box::new(move_sink),
                Box::new(lifecycle_sink),
            ],
            None,
            TickerMode::Disabled,
        );
        // 停止通知の channel（本番 `wire_runtime` と同型）: 送出端は kanade へ、受信端は
        // World の受け口（`KanadeNoticeRx`）へ挿す。終了相を回さないテストでは読まれないだけで無害である。
        let (kanade_stop_tx, kanade_stop_rx) = mpsc::channel::<KanadeNotice>();
        let ghost = boot_with_kanade_stop(options, Some(kanade_stop_tx))
            .expect("scripted boot は解決可能な emo2 ghost_root で成功する");

        // ── frame 三相結線状態（wire_runtime 手順6 相当・System 登録はせず直接駆動する） ──
        // RuntimeWiring は move の受信端 move_rx を保持し frame 相 drain（run_move_drain_phase）に
        // 備える。move の spine e2e は上の実 MoveCueSink 経由で cue→channel→drain を通す。
        // 表示ライフサイクルの受信端 lifecycle_rx も同様に保持し、可視性相の drain に備える。
        let wiring = RuntimeWiring::new(
            presenter,
            rx,
            move_rx,
            lifecycle_rx,
            mpsc::channel::<areka_runtime::zorder_cue::ZOrderDirective>().1,
            Rc::clone(&runtime),
            clock,
            wiring_assets,
        );
        world.insert_non_send(KanadeNoticeRx(kanade_stop_rx));

        SpineHarness {
            world,
            wiring,
            runtime,
            ghost,
            seriko,
            shiori_handle,
            text_pump,
            tick_sink,
            sample,
            gpu,
        }
    }

    /// 装着の相（`run_attach_phase`）を 1 回回し、装着の前提を確かめる（縮退がバグを隠さない檻）。
    ///
    /// 確かめるのは 3 つ: 装着の相のログを捕まえる・`planned=2 attached=2`（足場が立てる emo2 の
    /// 2 scope の全部が装着された）・ERROR 0 件（装着失敗・log-first）。
    #[track_caller]
    fn attach_all(&mut self) {
        let logs = capture_logs(|| run_attach_phase(&mut self.wiring, &mut self.world));
        assert!(
            logs.iter()
                .any(|l| l.contains("planned=2") && l.contains("attached=2")),
            "装着の前提（planned=2 attached=2＝足場の 2 scope の全部の装着）が観測できない: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "ERROR"),
            0,
            "装着の相で ERROR が出た（装着失敗・log-first）: {logs:?}"
        );
    }

    /// 文字層 UI アクターの pending メッセージを headless に drain する（実 ClockedTextSink 経路）。
    fn pump_text(&self) {
        pump_until_idle();
    }

    /// seriko ループへ tick を 1 発直接注入する（loop ticker 不起動・`SerikoSink::send_tick`）。
    ///
    /// 本番（`areka-runtime` の lib.rs の `wire_runtime`）は `spawn_loop_ticker` の worker スレッドが実時刻で `send_tick` するが、spine は
    /// 決定論のため ticker を起動せず、テストスレッドから注入時刻のみで `send_tick` を直接呼ぶ（時刻を進めるための sleep は無い）。
    /// dispatcher 経路（`inject_dispatcher_tick`）は talk/cue clock を進めるのみで seriko ループには届かない
    /// （ghost 側にループ結線なし）——ループ tick はこの直接注入だけが供給する。
    fn inject_seriko_tick(&self, now_ms: u64) {
        self.tick_sink.send_tick(now_ms);
    }

    /// dispatcher へ Tick を 1 発注入する（時刻は注入時刻のみで進める）。
    fn inject_dispatcher_tick(&self, now: u64) {
        self.ghost
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive to accept an injected Tick");
    }

    /// 正規終了＋全ハンドル有界 join でハーネスを畳む（hang させない）。
    ///
    /// `GhostRuntime::shutdown(User)` は ForceQuit 経路（OnClose NOTIFY→Unload）で ghost 一式を
    /// join し、dispatcher が保持する `SerikoSink` クローンを drop する→seriko worker の inbox 切断→
    /// 自然終了。続けて seriko を有界 join する（ghost spine S1/S2 の後片付け技法）。
    fn shutdown_bounded(self) {
        // 許可を先頭で束縛する（巻き戻りでも後から束縛した装置の世界より後に返る）。
        let SpineHarness {
            gpu,
            world,
            wiring,
            runtime,
            ghost,
            seriko,
            shiori_handle,
            text_pump,
            tick_sink,
            sample,
        } = self;

        // 降ろしは偽の SHIORI の呼び出し（OnClose・Unload）の数を進みの目印にして待つ。
        // 進んでいる限り負荷で遅いだけでは打ち切らず、30 秒進まなければ `［止まった］` で panic する。
        let calls = || shiori_handle.call_count();
        run_bounded_watching("spine ghost shutdown", Progress::Count(&calls), move || {
            // 正規 close（`CloseReason::User`）。ForceQuit ゆえ OnClose は NOTIFY で消化される。
            // 降ろしの失敗は捨てずに落とす（待つ側へ `［相手が居ない］` の赤として届く）。
            ghost
                .shutdown(CloseReason::User { scope: 0 })
                .expect("spine ghost shutdown");
        });
        // loop tick 直接注入端の clone を明示 drop: ghost.shutdown が dispatcher 保持の
        // SerikoSink クローンを drop しても、ハーネス保持の tick_sink clone が生きていると seriko inbox が
        // 切断されず worker が終了しない。全 Sender drop で自然終了させるため seriko join の前に drop する。
        drop(tick_sink);
        join_bounded("spine seriko join", seriko).expect(
            "seriko worker should terminate once all SerikoSink clones drop after shutdown",
        );

        // 残り（!Send・テストスレッド常駐）を明示 drop（UI アクター/presenter/Rc runtime/World）。
        drop(wiring);
        drop(world);
        drop(runtime);
        drop(text_pump);
        // 検体の複製は最後に捨てる（`SpineHarness::sample` の用心・テストの裏付けは無い）。
        drop(sample);
        drop(gpu);
    }
}

// テーマ単位のテストモジュール接続宣言。
// 本ファイルは足場（標準の台本・GPU World・SpineHarness・ログ捕捉）を保持し、
// 観測ケースはテーマごとの兄弟ファイル `spine_<テーマ>_tests.rs` に置く。
#[path = "spine_boot_smoke_tests.rs"]
mod boot_smoke_tests;
#[path = "spine_close_wiring_tests.rs"]
mod close_wiring_tests;
#[path = "spine_conformance_lap_tests.rs"]
mod conformance_lap_tests;
#[path = "spine_conformance_script.rs"]
mod conformance_script;
#[path = "spine_conformance_support.rs"]
mod conformance_support;
#[path = "spine_display_tests.rs"]
mod display_tests;
#[path = "spine_move_cue_tests.rs"]
mod move_cue_tests;
#[path = "spine_seriko_loop_tests.rs"]
mod seriko_loop_tests;
#[path = "spine_talk_close_tests.rs"]
mod talk_close_tests;
#[path = "spine_test_support.rs"]
mod test_support;
#[path = "spine_text_scale_tests.rs"]
mod text_scale_tests;
#[path = "spine_third_scope_talk_tests.rs"]
mod third_scope_talk_tests;
