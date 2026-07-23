//! 決定論 spine テストハーネス（R8・tasks.md task 6.1）。
//!
//! `cargo test --workspace`（外部 CI 無し・ローカル DoD ゲート）で常設観測する、起動〜発話〜
//! 終了の全経路を **sleep 不使用**・**注入 Tick のみ**・**headless GPU（WARP・MTA）** で通す
//! 決定論 spine の**土台**。本 task（6.1）はハーネス（scripted `ShioriBackend`＋実 sink 結線＋
//! GPU World＋frame フェーズ直接駆動）と、boot→Tick→attach 到達をスモークレベルで固定する
//! `#[test]` を所有する。豊富な観測（S1 ピクセル readback・S2 typewriter・S3 `\b` 配送・
//! S5 close 握手）は後続 task（6.2/6.3）が本ハーネスの上に構築する。
//!
//! # 構造上の逸脱（設計 File Structure との差分・CONCERNS 相当）
//!
//! design.md の File Structure は spine を外部結合テスト `tests/emo2_boot_spine_test.rs` に置くが、
//! `areka` は `[[bin]]` のみ（`[lib]` 無し）で外部 `tests/` から bin 内部項目
//! （`crate::emo2_boot::{PresentBridge, Emo2Wiring, run_attach_phase, ...}`）へ到達できない。
//! spine は Tick 注入＋GPU readback＋frame フェーズ直接駆動を **in-process** で行う必要があり
//! （bin を起動する外部テストでは不能）、既存 repo 慣行（`emo2_boot::wire_tests`・
//! `placement::spawn` のテスト）に従い **in-crate `#[cfg(test)]` モジュール** として置く。
//!
//! # 依存の申し送り（CONCERNS 相当）
//!
//! scripted `ShioriBackend` は `RequestError`/`ExitKind`/`ShutdownError`/`HelperStatus`
//! （`shiori_host32_host` 由来）をトレイト戻り値型として**名前で**参照する。`areka_kanade` は
//! これら host32 型を re-export しないため、`areka/Cargo.toml` の `[dev-dependencies]` へ
//! `shiori-host32-host`（既に areka-ghost/areka-kanade 経由で推移ビルド済み・x64・新規外部依存
//! ではない）を追加した。

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use areka_actor::{ActorError, ActorHandle};
use areka_emo_compose::BindSet;
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
use areka_emo_text::actor::{spawn_emo_text, TextLayerRuntime};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{boot, GhostBootOptions, GhostRuntime, ShioriWiring, TickerMode};
use areka_kanade::{CloseReason, MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::ActorKey;
use areka_seriko::{
    spawn_seriko, AnimationTable, BindResolver, LoopRng, SerikoLoopConfig, SerikoSink,
    SurfaceResolver,
};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use tracing::field::{Field, Visit};
use tracing_subscriber::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{GraphicsCore, Point, WindowHandle, WindowPos, WucGraphicsResource};
use wintf::executor::{FilterResult, JoinHandle, MessageLoop};

use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{spawn_ghost_windows, GhostWindows};

use super::adapter::PresentBridge;
use super::assets::{build_boot_assets, BootAssets, LoopTables};
use super::frame::{run_attach_phase, run_move_drain_phase, run_text_phase, Emo2Wiring};
use super::move_cue::{MoveCueSink, MoveDirective};
use super::talk_clock::{ClockedTextSink, TalkClock};
use super::target_map::{balloon_target, shell_target};

// ===========================================================================
// ScriptedShioriBackend（DD-11・areka 側 spine ローカルの最小 fake）
//
// `areka_kanade::ShioriBackend` を実装する台本 fake。プロセス spawn・実窓・i686 成果物を
// 一切要さない（純 x64・R8.6）。応答・終了結果は事前登録し、`status()` は固定値を返す。
// トレイト形状・戻り値型は `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の
// `ScriptedShioriBackend`（task 4.1 成果物）を正典として写した最小版。
// ===========================================================================

/// backend が受領した 1 呼出の記録（照合用）。`Get`/`Notify` は id・references を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordedCall {
    /// GET 呼出（応答を要するイベント）。
    Get { id: String, references: Vec<String> },
    /// NOTIFY 呼出（片道イベント）。
    Notify { id: String, references: Vec<String> },
    /// unload（正規 clean shutdown）呼出。
    Unload,
    /// status（非ブロッキング死活問い合わせ）呼出。
    Status,
}

/// [`ScriptedShioriBackend`] を組み立てるビルダー（台本の事前登録）。
///
/// GET/NOTIFY は id ごとに応答列（`VecDeque`）を積み、呼出のたびに先頭から 1 件消費する
/// （`RequestError`/`ShutdownError` は `Clone` 非実装ゆえ値を使い切り消費する設計）。
struct ScriptedShioriBackendBuilder {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
}

impl ScriptedShioriBackendBuilder {
    /// 空の台本（既定 status=`Running`）から開始する。
    fn new() -> Self {
        Self {
            get_scripts: HashMap::new(),
            notify_scripts: HashMap::new(),
            unload_script: None,
        }
    }

    /// `id` に対する GET 応答を 1 件、応答列の末尾へ積む（複数回で FIFO 消費）。
    fn get(mut self, id: impl Into<String>, response: Result<Option<String>, RequestError>) -> Self {
        self.get_scripts.entry(id.into()).or_default().push_back(response);
        self
    }

    /// `id` に対する NOTIFY 応答を 1 件、応答列の末尾へ積む。
    fn notify(mut self, id: impl Into<String>, response: Result<(), RequestError>) -> Self {
        self.notify_scripts.entry(id.into()).or_default().push_back(response);
        self
    }

    /// `unload()` の結果を台本化する（一度きり消費・`Option::take` で払い出す）。
    fn unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
        self.unload_script = Some(response);
        self
    }

    /// backend 本体（アクタースレッドへ move する側）と、テストが照合に使う
    /// [`ScriptedShioriHandle`] のペアを構築する。
    fn build(self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = ScriptedShioriBackend {
            get_scripts: self.get_scripts,
            notify_scripts: self.notify_scripts,
            unload_script: self.unload_script,
            status: HelperStatus::Running,
            calls: Arc::clone(&calls),
        };
        let handle = ScriptedShioriHandle { calls };
        (backend, handle)
    }
}

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・R8.1/R8.6）。
struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    status: HelperStatus,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriBackend {
    /// ビルダー起点。
    fn builder() -> ScriptedShioriBackendBuilder {
        ScriptedShioriBackendBuilder::new()
    }
}

impl ShioriBackend for ScriptedShioriBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.calls.lock().expect("calls mutex poisoned").push(RecordedCall::Get {
            id: id.to_string(),
            references: references.to_vec(),
        });
        self.get_scripts.get_mut(id).and_then(VecDeque::pop_front).unwrap_or_else(|| {
            panic!("ScriptedShioriBackend::get(\"{id}\"): no scripted response left")
        })
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.calls.lock().expect("calls mutex poisoned").push(RecordedCall::Notify {
            id: id.to_string(),
            references: references.to_vec(),
        });
        self.notify_scripts.get_mut(id).and_then(VecDeque::pop_front).unwrap_or_else(|| {
            panic!("ScriptedShioriBackend::notify(\"{id}\"): no scripted response left")
        })
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.calls.lock().expect("calls mutex poisoned").push(RecordedCall::Unload);
        self.unload_script
            .take()
            .unwrap_or_else(|| panic!("ScriptedShioriBackend::unload(): no scripted response configured"))
    }

    fn status(&mut self) -> HelperStatus {
        self.calls.lock().expect("calls mutex poisoned").push(RecordedCall::Status);
        self.status
    }
}

/// [`ScriptedShioriBackend`] をテスト側から観測するためのハンドル（`Arc` 共有）。
///
/// backend 本体を別スレッド（shiori actor）へ move した後も、このハンドルから発火列を照合できる。
#[derive(Clone)]
struct ScriptedShioriHandle {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriHandle {
    /// 受領記録（非 Status のみ）のスナップショットを返す（死活監視ノイズを除外）。
    fn non_status_calls(&self) -> Vec<RecordedCall> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect()
    }
}

// ===========================================================================
// GPU / fixture / 有界待機ヘルパ（draw_readback_test／ghost spine 定石の踏襲）
// ===========================================================================

/// `GraphicsCore`＋`WucGraphicsResource` を実資源として載せた wintf World（headless GPU・R8.4）。
///
/// 本番 UI スレッドは MTA（記憶: areka WUC は MTA スレッドで動く）。WARP 可（`GraphicsCore::new()`）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// emo2 fixture ルート（`CARGO_MANIFEST_DIR`＝`crates/areka` 相対・assets.rs テストと同一規約）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（assets.rs テストと同一規約）。
fn emo2_balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

/// scope0/scope1 の 2 スコープぶんの合成配置（placement::spawn テストの emo2 相当値を踏襲）。
///
/// attach フェーズは窓 `Entity` のみを消費し `WindowPos`/寸法は読まないため、位置値は attach の
/// 成否に無関係（`GhostWindows::scopes()` が `[0,1]` を返すことだけが load-bearing）。
fn two_scope_placements() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 733 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 708 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            anchor: Anchor::Bottom,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1063 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1044 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            anchor: Anchor::Bottom,
        },
    ]
}

/// 窓タイトル（emo2 相当）。
fn titles() -> GhostTitles {
    GhostTitles::from_scope_titles([(0, "むらさき".to_string()), (1, "エモ".to_string())])
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （draw_readback_test/attach_wiring の決定論パターン——WM_QUIT は posted が尽きた後に配送）。
///
/// 実 `ClockedTextSink<EmoTextSink>` の UI ドレイン（`spawn_emo_text` の pump アクター）を
/// headless に駆動できることを裏付ける定石。sink 生存中は複数セッション実行できる。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// クロージャ `f` を別スレッドで実行し有界時間で完了を観測する（ghost spine の `run_bounded` 同旨）。
fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    let (done_tx, done_rx) = mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// `ActorHandle::join` を有界時間で観測する（ghost spine の `join_bounded` 同旨）。
fn join_bounded(what: &str, timeout: Duration, handle: ActorHandle) -> Result<(), ActorError> {
    let (res_tx, res_rx) = mpsc::sync_channel::<Result<(), ActorError>>(0);
    std::thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
    }
}

// ===========================================================================
// ログ捕捉（frame.rs/adapter.rs の確立パターンを単一ファイル境界内へ最小複製）
//
// スレッドローカル `with_default` ゆえ `cargo test` 並行実行でも他スレッドの subscriber と
// 干渉しない。`run_attach_phase` の `info!(planned, attached, ...)`（DD-12 の縮退がバグを
// 隠さない檻の観測点）をこのスレッド上で捕捉して件数一致を assert する。
// ===========================================================================

/// イベントの `level`＋各フィールドを 1 行文字列へ整形して共有 Vec へ push する最小 Layer。
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
    fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let meta = ev.metadata();
        let mut line = format!("level={} target={}", meta.level(), meta.target());
        struct V<'a>(&'a mut String);
        impl Visit for V<'_> {
            fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = write!(self.0, " {}={:?}", f.name(), v);
            }
        }
        ev.record(&mut V(&mut line));
        self.0.lock().unwrap().push(line);
    }
}

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, f);
    let guard = logs.lock().unwrap();
    guard.clone()
}

/// 捕捉行のうち指定 level（例 `"ERROR"`）の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|l| l.contains(&needle)).count()
}

// ===========================================================================
// SpineHarness（再利用可能な結線ハーネス・6.2/6.3 がこの上に構築する）
// ===========================================================================

/// spine ハーネスの SERIKO ループ駆動モード（task 9.4・design「結線・資産・実機経路（spine.rs）」）。
///
/// spine は loop ticker を**起動しない**。ループ駆動は `SerikoLoopConfig`（表＋乱数）を seriko へ値渡しで
/// 仕込み、tick は `SerikoSink::send_tick` を直接注入して制御する（決定論・sleep 不使用・R7.2/7.3）。
enum LoopDriver {
    /// ループ完全不活性（`SerikoLoopConfig::disabled()` 相当＝空表・ダミー乱数）。既存 spine 全テストの
    /// 非退行経路（設計 Testing Strategy E2E-3・Implementation Notes）。send_tick を注入してもループは
    /// 表示中 slot の評価対象アニメが常にゼロ＝何も発行しない（従来観測どおり）。
    Inert,
    /// 実 emo2 表（`BootAssets.loop_tables`）＋固定注入乱数列で駆動（まばたき e2e・R7.2/7.3）。
    Live(LoopRng),
}

/// scripted ghost boot＋実 sink 結線＋GPU World＋frame 結線状態を束ねた spine ハーネス。
///
/// `wire_emo2_boot`（task 5.1・production）の結線を、headless GPU World＋合成 `GhostWindows`＋
/// scripted backend＋`TickerMode::Disabled` で**テスト内 in-process** に再現したもの。frame
/// フェーズ（`run_attach_phase` 等）を直接駆動でき、`ghost`/`seriko`/`shiori_handle` で
/// ライフサイクル・発火列を観測できる。
struct SpineHarness {
    /// GPU 資源（`GraphicsCore`/`WucGraphicsResource`）＋合成 `GhostWindows`＋装着スロットを持つ World。
    world: World,
    /// frame 三相結線状態（presenter/rx/runtime/clock/assets）。`run_attach_phase` の駆動対象。
    wiring: Emo2Wiring,
    /// 文字層ランタイムの観測用クローン（6.2/6.3 が `present_frame`/`read_back` に使う）。
    #[allow(dead_code)]
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// ghost ランタイム（`dispatcher()` への Tick 注入・`shutdown()` の駆動）。
    ghost: GhostRuntime,
    /// seriko worker の join ハンドル（有界 join で終了確認）。
    seriko: ActorHandle,
    /// scripted backend の発火列観測ハンドル（boot 系列・close 系列の照合）。
    shiori_handle: ScriptedShioriHandle,
    /// 文字層 UI アクター（`spawn_emo_text` の pump アクター）の生存ハンドル（drain は `pump_text`）。
    #[allow(dead_code)]
    text_pump: JoinHandle<()>,
    /// SERIKO ループ tick の直接注入端（task 9.4）。`spawn_seriko` が返す `SerikoSink` の clone で、
    /// `inject_seriko_tick`（`send_tick` 直接注入・loop ticker 不起動・R7.2/7.3）に使う。surface_sink 本体は
    /// boot が sinks 第 1 要素として値消費するため、この clone をハーネスに保持する。全 clone は単一 seriko
    /// inbox への送信端で配送意味は同一。shutdown 時に drop して inbox 切断→worker 自然終了させる。
    tick_sink: SerikoSink,
}

impl SpineHarness {
    /// 標準台本（boot 系列＋`\s[0]\e` OnBoot＋ForceQuit close 系列）で spine ハーネスを起動する。
    ///
    /// `on_boot` は OnBoot GET が返す応答スクリプト。6.1 は最小 talk（`\s[0]\e`）を渡す。
    /// SERIKO ループは **不活性**（`LoopDriver::Inert`）で駆動する——既存 spine 全テストの非退行経路
    /// （設計 Testing Strategy E2E-3）。ループを実表で活性化するまばたき e2e は [`Self::boot_live`]。
    fn boot(on_boot: &str) -> SpineHarness {
        let (backend, shiori_handle) = Self::standard_backend(on_boot);
        Self::boot_with(backend, shiori_handle, LoopDriver::Inert)
    }

    /// 実 emo2 表＋固定注入乱数列で SERIKO ループを**活性化**して spine ハーネスを起動する（task 9.4）。
    ///
    /// `rng` は 1/N 抽選の固定注入列（決定論・実 entropy 非依存・R7.1/7.2）。tick は loop ticker を起動せず
    /// [`Self::inject_seriko_tick`]（`send_tick` 直接注入）で制御する（sleep 不使用・R7.3）。まばたき e2e が
    /// 使う（実 kero/sakura まばたき 1 周の full golden は task 10.2）。
    fn boot_live(on_boot: &str, rng: LoopRng) -> SpineHarness {
        let (backend, shiori_handle) = Self::standard_backend(on_boot);
        Self::boot_with(backend, shiori_handle, LoopDriver::Live(rng))
    }

    /// 標準 scripted backend（boot 系列＋OnBoot talk＋ForceQuit close 系列）を組む。
    ///
    /// boot 系列（OnInitialize→OnFirstBoot→OnBoot→basewareversion）＋ shutdown（`GhostRuntime::shutdown`
    /// は常に ForceQuit 経路＝OnClose NOTIFY→Unload・ghost spine S1 と同旨）を台本化する。OnSecondChange は
    /// kanade へ Tick を送らないため不要。
    fn standard_backend(on_boot: &str) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(on_boot.to_string())))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
            .build()
    }

    /// 任意の scripted backend＋ループ駆動モードで spine ハーネスを起動する（6.2/6.3 が独自台本を注入する口）。
    fn boot_with(
        backend: ScriptedShioriBackend,
        shiori_handle: ScriptedShioriHandle,
        driver: LoopDriver,
    ) -> SpineHarness {
        // ── headless GPU World（MTA COM＋WARP 可・R8.4）＋合成 GhostWindows（scope [0,1]） ──
        let mut world = make_world_with_gpu();
        spawn_ghost_windows(&mut world, &two_scope_placements(), &titles());

        // ── 構築入力（実 emo2 fixture・COM は make_world_with_gpu で初期化済み） ──
        let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1])
            .expect("emo2 fixture の BootAssets 組立は成功する");

        // ── presenter／文字層ランタイム／実 EmoTextSink（テストスレッド＝UI pump スレッド） ──
        let presenter = EmoPresenter::new();
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
        let (emo_text_sink, text_pump) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump (test) thread");

        // ── talk 時刻源（本番同様 dola clock 注入）＋実 ClockedTextSink<EmoTextSink> ──
        let clock_fn: Arc<dyn Fn() -> f64 + Send + Sync> = Arc::new(dola::runtime::clock::now);
        let clock = TalkClock::new(clock_fn);
        let clocked_text_sink = ClockedTextSink::new(emo_text_sink, clock.clone());

        // ── mpsc＋PresentBridge→spawn_seriko（実 SurfaceOutput 経路）。SurfaceResolver は
        //    非 Clone ゆえ spawn_seriko が値消費し、Emo2Wiring 側は空 alias のプレースホルダで埋める
        //    （attach は resolver を読まない・Task 4.1 申し送り・wire_emo2_boot 手順4 と同型）。 ──
        let (tx, rx) = mpsc::channel::<PresentCommand>();
        let bridge = PresentBridge::new(tx);
        let BootAssets {
            shells,
            balloons,
            balloon_model,
            resolver,
            static_binds,
            bind_resolver,
            loop_tables,
        } = assets;
        // SERIKO ループ構成（task 9.4・design「結線・資産・実機経路（spine.rs）」）: 実 emo2 表
        // （`BootAssets.loop_tables`＝task 9.1 が `EmoWorld` スナップショットから `from_world` で構築）＋
        // 固定注入乱数列（`driver`）で `loop_config` を組む。既存 spine テストは Inert（`disabled()` 相当＝
        // 空表・ダミー乱数）でループ完全不活性＝従来観測どおり非退行（設計 Testing Strategy E2E-3・
        // Implementation Notes）。まばたき e2e（`boot_live`）のみ実表＋固定 rng で駆動する（本番 mod.rs は
        // 実 entropy・spine は固定注入列で決定論・R7.1/7.2/7.3）。
        let LoopTables {
            shell: shell_table,
            balloon: balloon_table,
        } = loop_tables;
        let loop_config = match driver {
            LoopDriver::Inert => SerikoLoopConfig::disabled(),
            LoopDriver::Live(rng) => SerikoLoopConfig {
                shell_table,
                balloon_table,
                rng,
            },
        };
        // 実名前解決表（BootAssets.bind_resolver・task 7.1 が emo2 fixture の MountModel から構築）＋
        // loop_config を seriko の起動へ値渡しで配線する（production wire_emo2_boot と同型・task 7.2/9.4）。
        let (surface_sink, seriko) = spawn_seriko(
            resolver,
            static_binds.clone(),
            bind_resolver,
            loop_config,
            bridge,
        );
        // loop tick 直接注入端（task 9.4）: SerikoSink を 1 本 clone してハーネスへ保持する。surface_sink 本体は
        // 下の boot_options が sinks 第 1 要素として値消費するため、この clone を `inject_seriko_tick`
        // （send_tick 直接注入・loop ticker 不起動・R7.2/7.3）に使う。全 clone は単一 seriko inbox への送信端で
        // 配送意味は同一。shutdown 時に drop して inbox 切断→worker 自然終了させる（下記 shutdown_bounded）。
        let tick_sink = surface_sink.clone();
        let wiring_assets = BootAssets {
            shells,
            balloons,
            balloon_model,
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds,
            // 実 bind_resolver は seriko が値消費済み（attach は bind_resolver を読まない）ため空表プレースホルダ。
            bind_resolver: BindResolver::empty(),
            // 実 loop_tables は loop_config へ移送済み（attach は loop_tables を読まない）ため空表プレースホルダ。
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: AnimationTable::empty(),
            },
        };

        // ── move channel＋実 MoveCueSink（wire_emo2_boot 手順4 と同型・S-3 形＝task 9.3） ──
        // talk スレッドの MoveCueSink が送出端、UI スレッドの Emo2Wiring が受信端 move_rx（frame 相
        // drain＝run_move_drain_phase・task 9.2）を保持する。9.1 の throwaway 送出端を実 MoveCueSink へ
        // 差し替え、production `wire_emo2_boot` の 3-sink 構成を spine でも忠実に再現する。
        let (move_tx, move_rx) = mpsc::channel::<MoveDirective>();
        let move_sink = MoveCueSink::new(move_tx);

        // ── scripted boot（実 sink 注入・TickerMode::Disabled＝Tick 注入で駆動・R8.3） ──
        // sinks は broadcast 登録先で surface（seriko）／text（ClockedTextSink）／move（MoveCueSink）の
        // 3 sink を第 1〜3 要素として渡す（production mod.rs と同順・S-3 形）。
        let options = GhostBootOptions {
            ghost_root: emo2_root(),
            default_encoding: DefaultEncoding::Ansi,
            shiori: ShioriWiring::Custom(Box::new(move || Ok(Box::new(backend) as Box<dyn ShioriBackend>))),
            sinks: vec![
                Box::new(surface_sink),
                Box::new(clocked_text_sink),
                Box::new(move_sink),
            ],
            system_vars: areka_ghost::default_system_vars(),
            ticker: TickerMode::Disabled,
        };
        let ghost = boot(options).expect("scripted boot は解決可能な emo2 ghost_root で成功する");

        // ── frame 三相結線状態（wire_emo2_boot 手順6 相当・System 登録はせず直接駆動する） ──
        // Emo2Wiring は move の受信端 move_rx を保持し frame 相 drain（run_move_drain_phase・task 9.2）に
        // 備える。move の spine e2e（task 9.3）は上の実 MoveCueSink 経由で cue→channel→drain を通す。
        let wiring =
            Emo2Wiring::new(presenter, rx, move_rx, Rc::clone(&runtime), clock, wiring_assets);

        SpineHarness { world, wiring, runtime, ghost, seriko, shiori_handle, text_pump, tick_sink }
    }

    /// 文字層 UI アクターの pending メッセージを headless に drain する（実 ClockedTextSink 経路）。
    fn pump_text(&self) {
        pump_until_idle();
    }

    /// seriko ループへ tick を 1 発直接注入する（loop ticker 不起動・`SerikoSink::send_tick`・R7.2/7.3）。
    ///
    /// 本番（mod.rs）は `spawn_loop_ticker` の worker スレッドが実時刻で `send_tick` するが、spine は
    /// 決定論のため ticker を起動せず、テストスレッドから注入時刻のみで `send_tick` を直接呼ぶ（sleep 不使用）。
    /// dispatcher 経路（`inject_dispatcher_tick`）は talk/cue clock を進めるのみで seriko ループには届かない
    /// （ghost 側にループ結線なし）——ループ tick はこの直接注入だけが供給する。
    fn inject_seriko_tick(&self, now_ms: u64) {
        self.tick_sink.send_tick(now_ms);
    }

    /// dispatcher へ Tick を 1 発注入する（sleep 不使用・注入時刻のみで進める・R8.3）。
    fn inject_dispatcher_tick(&self, now: u64) {
        self.ghost
            .dispatcher()
            .send(DispatcherMsg::Tick { now: MonotonicMs(now) })
            .expect("dispatcher actor should still be alive to accept an injected Tick");
    }

    /// 正規終了＋全ハンドル有界 join でハーネスを畳む（hang させない）。
    ///
    /// `GhostRuntime::shutdown(User)` は ForceQuit 経路（OnClose NOTIFY→Unload）で ghost 一式を
    /// join し、dispatcher が保持する `SerikoSink` クローンを drop する→seriko worker の inbox 切断→
    /// 自然終了。続けて seriko を有界 join する（ghost spine S1/S2 の後片付け技法）。
    fn shutdown_bounded(self) {
        let SpineHarness { world, wiring, runtime, ghost, seriko, shiori_handle, text_pump, tick_sink } =
            self;

        run_bounded("spine ghost shutdown", Duration::from_secs(10), move || {
            // 正規 close（DD-10 と同じ User）。ForceQuit ゆえ OnClose は NOTIFY で消化される。
            let _ = ghost.shutdown(CloseReason::User);
        });
        // loop tick 直接注入端の clone を明示 drop（task 9.4）: ghost.shutdown が dispatcher 保持の
        // SerikoSink クローンを drop しても、ハーネス保持の tick_sink clone が生きていると seriko inbox が
        // 切断されず worker が終了しない。全 Sender drop で自然終了させるため seriko join の前に drop する。
        drop(tick_sink);
        join_bounded("spine seriko join", Duration::from_secs(10), seriko)
            .expect("seriko worker should terminate once all SerikoSink clones drop after shutdown");

        // 残り（!Send・テストスレッド常駐）を明示 drop（UI アクター/presenter/Rc runtime/World）。
        drop(wiring);
        drop(world);
        drop(runtime);
        drop(text_pump);
        let _ = shiori_handle;
    }
}

// ===========================================================================
// task 6.1 スモークテスト
// ===========================================================================

/// 観測可能な完了条件（tasks.md task 6.1）: ハーネスが scripted ghost を boot させ、Tick 注入に
/// より attach 準備状態まで **panic なく** 到達することをスモークレベルで固定する（R8.1/8.3/8.4/8.6）。
///
/// 檻に入れる判断分岐:
/// - **scripted boot 発火**: boot 系列（OnInitialize→OnFirstBoot→OnBoot→basewareversion）が
///   scripted backend へ (method,id) 順で届く（＝「scripted ghost を boot させた」直接証跡）。
/// - **Tick 注入の疎通**: `dispatcher()` への Tick 送出が Ok（ghost スタック生存・sleep 不使用）。
/// - **attach 到達**: headless GPU World（WARP・MTA）＋合成 `GhostWindows` 上で `run_attach_phase`
///   が **panic せず** 完走し、DD-12 の縮退がバグを隠さない檻＝`planned==attached==2`（全 scope の
///   シェル装着成功）を装着サマリ `info!` で観測でき、ERROR は 0 件。
/// - **ハンドル生存**: attach 後も seriko worker は稼働中・dispatcher は再度 Tick を受理する。
///
/// 豊富な観測（S1 ピクセル readback・S2 typewriter・S3 `\b`・S5 close 握手）は 6.2/6.3 の担当。
#[test]
fn spine_harness_boots_scripted_ghost_and_reaches_attach_ready() {
    // 最小 talk（1 サーフェス cue・テキストなし）で決定論を単純に保つ。
    let mut harness = SpineHarness::boot(r"\s[0]\e");

    // ── (1) scripted boot 発火: boot 系列が backend へ (method,id) 順で届く ──
    // boot 系列は kanade スレッド上の同期往復のみで完走する（Tick 不要）。実スレッド境界を跨ぐため
    // 有界スピン待機（sleep なし・yield_now のみ）で 4 呼出の到達を待ってから照合する。
    let mut boot_calls = Vec::new();
    for _ in 0..100_000u32 {
        boot_calls = harness.shiori_handle.non_status_calls();
        if boot_calls.len() >= 4 {
            break;
        }
        std::thread::yield_now();
    }
    let projected: Vec<(&str, &str)> = boot_calls
        .iter()
        .map(|c| match c {
            RecordedCall::Notify { id, .. } => ("notify", id.as_str()),
            RecordedCall::Get { id, .. } => ("get", id.as_str()),
            RecordedCall::Unload => ("unload", ""),
            RecordedCall::Status => ("status", ""),
        })
        .collect();
    assert!(
        projected.len() >= 4,
        "scripted boot 系列が有界内に発火しない（scripted ghost を boot できていない）: {boot_calls:?}"
    );
    assert_eq!(
        &projected[..4],
        &[
            ("notify", "OnInitialize"),
            ("get", "OnFirstBoot"),
            ("get", "OnBoot"),
            ("notify", "basewareversion"),
        ],
        "boot 系列が正典順序（OnInitialize→OnFirstBoot→OnBoot→basewareversion）で発火していない"
    );

    // ── (2) Tick 注入の疎通（ghost スタック生存・sleep 不使用・R8.3） ──
    harness.inject_dispatcher_tick(1);

    // ── 実 ClockedTextSink<EmoTextSink> の UI ドレインが headless に pump できることを裏付ける
    //    （pending なし／ありに関わらず panic しない・R8 の実 sink 経路の疎通確認）。 ──
    harness.pump_text();

    // ── (3) attach 到達: run_attach_phase を GPU World＋合成 GhostWindows 上で駆動し、DD-12 の
    //    「計画件数＝実装着件数」を装着サマリで観測する（縮退がバグを隠さない檻・R8.1）。 ──
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "attach 到達（planned=2 attached=2＝全 scope のシェル装着成功）が観測できない: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach フェーズで ERROR が発火した（装着失敗・log-first）: {logs:?}"
    );

    // ── (4) ハンドル生存: seriko worker 稼働中・dispatcher は再度 Tick を受理する ──
    assert!(
        !harness.seriko.is_finished(),
        "attach 到達時点で seriko worker は稼働中であるべき（実 sink 経路が生きている）"
    );
    harness.inject_dispatcher_tick(2);

    // ── 後片付け: 正規終了＋全ハンドル有界 join（hang させない・R8.3 の観測点＝有界 join のみ） ──
    harness.shutdown_bounded();
}

// ===========================================================================
// task 6.2 spine 観測ケース（S1 boot→表示／S3 `\b` 配送／S4 `\b` なし完走）
//
// 6.1 の `SpineHarness` の上に構築する。実 sink 経路の末端（`EmoPresenter::apply` の実描画→
// `read_back`）まで観測境界を延ばす（R8.2）。sleep 不使用・注入 Tick と有界 drain のみ（R8.3）・
// headless GPU（WARP・MTA・R8.4）・x64 完結（R8.6）。
// ===========================================================================

/// BGRA 密配列（`stride=width*4`）のうち α バイト（各 4 バイト画素の index 3）が非 0 の画素数を数える。
///
/// 「非全透明（初期面が実描画された）」の R8.5 述語＝`opaque_count > 0`。`read_back` は
/// premultiplied B8G8R8A8 を密（RowPitch 除去済み）で返すため、単純な 4 バイト刻みで α を見る。
fn opaque_count(bgra: &[u8]) -> usize {
    bgra.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// `PresentCommand`（`#[non_exhaustive]`）の表示対象 `TargetId` を取り出す（未知 variant は `None`）。
fn present_command_target(cmd: &PresentCommand) -> Option<TargetId> {
    match cmd {
        PresentCommand::ShowSurface { target, .. } => Some(*target),
        PresentCommand::Hide { target, .. } => Some(*target),
        PresentCommand::InvalidateCache { target, .. } => Some(*target),
        _ => None,
    }
}

/// `PresentCommand` の variant 名（`PresentCommand` は `reply` を含み `Debug` 非実装ゆえ診断表示用）。
fn variant_name(cmd: &PresentCommand) -> &'static str {
    match cmd {
        PresentCommand::ShowSurface { .. } => "ShowSurface",
        PresentCommand::Hide { .. } => "Hide",
        PresentCommand::InvalidateCache { .. } => "InvalidateCache",
        _ => "<unknown>",
    }
}

/// 注入 Tick を増分しながら rx を有界に drain し、`want` 件の `PresentCommand` を集めて返す。
///
/// scripted OnBoot talk（`ghost`→`sakura`→`seriko`→`PresentBridge`→rx）は別スレッド群を跨いで
/// 非同期に流れるため、Tick 注入（`DispatcherMsg::Tick`）と `try_iter` drain を有界ループで交互に
/// 回し、必要件数が揃うか反復上限に達するまで進める（sleep 不使用・`yield_now` のみ・R8.3）。全 cue
/// が `at=0.0`（`\w` なし）ゆえ最初の有効 Tick で発火し切るが、talk 起動・スレッド伝播の遅延を有界
/// スピンで吸収する。揃わなければ短い Vec を返す（呼び手が件数を assert＝hang しない）。
fn tick_and_collect(harness: &mut SpineHarness, want: usize) -> Vec<PresentCommand> {
    let mut received: Vec<PresentCommand> = Vec::new();
    for now in 1u64..=200_000 {
        harness.inject_dispatcher_tick(now);
        received.extend(harness.wiring.drain_received());
        if received.len() >= want {
            break;
        }
        std::thread::yield_now();
    }
    received
}

/// spine S1（boot→表示・DD-12・R1.1/1.2/1.3/1.4/8.1/8.2/8.5）: `\b` を含まない OnBoot 台本
/// （`\s[0]`）で boot→attach フェーズ→初回 `\s` 駆動を走らせ、(a) 装着サマリ `info!` が
/// planned==attached==2（DD-12 の縮退が scope 導出バグを隠さない檻＝期待 scope 数の全 target 完了）・
/// ERROR 0 件、(b) **シェルは attach 直後は非表示**（`read_back` Err＝供給面未生成・defect #5・
/// 2026-07-13 実機#5）で**バルーンは attach 初回表示済み**（`opaque_count>0`＝面0 の実描画＋文字層
/// スロット取得）、(c) 最初のさくらスクリプト `\s[0]` cue が seriko→PresentBridge→drain 経路で運ぶ
/// `ShowSurface{shell_target(0),0}` を apply するとシェルが**非表示→surface0 の実描画**へ遷移する
/// （`opaque_count>0`）ことを固定する。観測境界を実描画→readback まで延ばす（R8.2）。
///
/// # defect #5 の檻（シェルは初回 `\s` まで非表示）
///
/// 旧 DD-9 は attach 時にシェル初期面（scope0=surface0／scope>=1=surface10）を焼き込んでいたが、実機#5
/// で「起動時に規定面が一瞬ちらつく」欠陥が判明した。SSP 互換の既定は「シェル表示なし（-1）」であり、
/// 初回シェル表示は最初の `\s` cue が駆動する。本ケースは attach 直後の shell `read_back` が Err
/// （供給面未生成＝合成面なし＝透過）であること、`\s[0]` 適用でのみシェルが非表示→実描画へ遷移する
/// ことを檻に入れて回帰を防ぐ。バルーンは文字層スロット取得のため attach 初回表示（面0）を保つ。
#[test]
fn spine_s1_boot_to_display_attaches_all_targets_with_opaque_readback() {
    let mut harness = SpineHarness::boot(r"\s[0]\e"); // \b-free（シェル面 cue \s[0] のみ）

    // attach フェーズ: DD-12 の planned==attached==2 を装着サマリで観測（縮退がバグを隠さない檻）。
    // attach は Tick 非依存（GPU 資源＋GhostWindows ゲートのみ）ゆえ boot 直後に直接駆動する。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "DD-12: 計画件数＝実装着件数（planned=2 attached=2・期待 scope 数の全 target 完了）が観測できない: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach で ERROR が発火（装着失敗・log-first）: {logs:?}"
    );

    // (b-1) シェルは初回 `\s` cue まで非表示（defect #5）: attach 直後の shell target は供給面未生成
    //       ＝`read_back` Err（合成面なし＝透過）。attach で surface0/surface10 を焼き付けない。
    for (label, target) in [
        ("shell scope0", shell_target(0)),
        ("shell scope1", shell_target(1)),
    ] {
        assert!(
            harness.wiring.read_back_target(target).is_err(),
            "{label} は初回 \\s cue 前は非表示であるべき（供給面未生成・read_back Err・defect #5）"
        );
    }

    // (b-2) バルーンは attach 初回表示（面0・文字層スロット取得のため保持）＝非全透明（R8.1/8.2/8.5）。
    for (label, target) in [
        ("balloon scope0", balloon_target(0)),
        ("balloon scope1", balloon_target(1)),
    ] {
        let px = harness.wiring.read_back_target(target).unwrap_or_else(|e| {
            panic!("{label} の read_back 失敗（バルーン初回面表示で供給面生成済みのはず）: {e:?}")
        });
        assert!(
            opaque_count(&px) > 0,
            "{label} の readback が全透明（バルーン初期面が表示されていない・R8.1/8.2/8.5）: len={}",
            px.len()
        );
    }

    // (c) 初回 `\s[0]` cue を実 sink 経路（ghost→sakura→seriko→PresentBridge→rx）で駆動し、shell 表示
    //     対象（偶数 TargetId）へ ShowSurface{shell_target(0),0,static_binds} が届くことを確認する（R8.2）。
    let mut received = tick_and_collect(&mut harness, 1);
    let show_idx = received
        .iter()
        .position(|c| matches!(c, PresentCommand::ShowSurface { target, .. } if *target == shell_target(0)))
        .unwrap_or_else(|| {
            panic!(
                "S1: 初回 \\s[0] のシェル ShowSurface{{shell_target(0)}} が受信列に無い: variants={:?}",
                received.iter().map(variant_name).collect::<Vec<_>>()
            )
        });
    let show = received.remove(show_idx);
    match &show {
        PresentCommand::ShowSurface { target, surface_id, .. } => {
            assert_eq!(*target, shell_target(0), "初回 \\s[0] は shell 表示対象（偶数 TargetId・DD-3）");
            assert_eq!(*surface_id, 0, "surface_id は 0（\\s[0]・seriko 数値解決の透過）");
        }
        _ => unreachable!("position で ShowSurface を選別済み"),
    }

    // 形状記録後に実 presenter へ apply（実描画→readback・R8.2）。初回 `\s` 適用で shell が非表示から
    // surface0 の実描画へ遷移する（hidden→shown・defect #5 の正しい表示駆動）。
    harness.wiring.apply_present(&mut harness.world, show);
    let after_show = harness
        .wiring
        .read_back_target(shell_target(0))
        .expect("初回 \\s[0] 適用後は shell scope0 の供給面が生成され read_back 可能");
    assert!(
        opaque_count(&after_show) > 0,
        "初回 \\s[0] 適用で shell scope0 が surface0 の実描画へ遷移（非表示→非全透明・R8.1/8.2/8.5）"
    );

    harness.shutdown_bounded();
}

/// spine S3（`\b` 配送・R5.4/DD-5・R8.2）: `\b[-1]`→`\b[0]` を含む scripted OnBoot 台本を実 sink 経路
/// （`ghost`→`sakura`→`seriko`→`PresentBridge`→rx）で流し、受信 `PresentCommand` 列に
/// `Hide{balloon_target(0)}`→`ShowSurface{balloon_target(0), surface_id:0, binds:default}` が
/// **順序どおり**現れることをアサートする（受信列順序＝本ケースの観測完了条件）。続けて記録済み指令を
/// 実 presenter へ apply し、balloon target の `read_back` が非全透明（surface0 の実描画・R8.2）で
/// attach 初期面と同一バイトを再駆動することを確認する。
///
/// # readback 遷移の観測境界（実装事実の申し送り・CONCERNS 相当）
///
/// `EmoPresenter::apply_hide` は WUC visual の可視フラグを落とすのみで swap chain の供給面
/// （`source_tex`）は破棄しない。`read_back` はその供給面を直読みするため **Hide は readback の
/// バイトを変えない**（emo-present の `empty_composition_degrades_to_hidden_and_replies_ok` が
/// 同事実を固定＝Hide 縮退後も `read_back` は旧供給面長のまま成立）。加えて balloon fixture は
/// surface0 のみ・attach 初回表示も surface0 ゆえ、`\b[-1]`→`\b[0]` の前後で readback バイトは不変
/// （両方 surface0）。よって「Hide→全透明」型のピクセル遷移は本経路では観測不能である。本テストは
/// (1) 受信列順序（R5.4 の本質・観測完了条件）と (2) apply 後の balloon readback が非全透明かつ attach
/// 初期面と同一（surface_id/binds が正しく貫通し実描画された証跡・R8.2）で `\b` 配送の貫通を檻に入れる。
#[test]
fn spine_s3_balloon_face_cue_delivers_hide_then_show_in_order() {
    let mut harness = SpineHarness::boot(r"\b[-1]\b[0]\e");

    // 先に attach（balloon target を生成・初期面 surface0 を表示）。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("attached=2")),
        "S3 前提: attach 完了（balloon target 生成）が観測できない: {logs:?}"
    );

    // attach 初期面（surface0）の balloon readback を基準として捕捉（非全透明）。
    let baseline = harness
        .wiring
        .read_back_target(balloon_target(0))
        .expect("attach 後の balloon read_back（初期面 surface0）");
    assert!(
        opaque_count(&baseline) > 0,
        "前提: attach 初期面（balloon surface0）は非全透明"
    );

    // scripted OnBoot talk を Tick 注入で駆動し、受信 PresentCommand を 2 件（Hide→ShowSurface）集める。
    let received = tick_and_collect(&mut harness, 2);
    assert_eq!(
        received.len(),
        2,
        "\\b[-1]\\b[0] は Hide→ShowSurface のちょうど 2 件を配送するはず（受信 {} 件・variants={:?}）",
        received.len(),
        received.iter().map(variant_name).collect::<Vec<_>>()
    );

    // 受信列順序（R5.4/DD-5）: 1 件目 Hide{balloon(0)}・2 件目 ShowSurface{balloon(0),0,default}。
    match &received[0] {
        PresentCommand::Hide { target, reply } => {
            assert_eq!(*target, balloon_target(0), "1 件目は balloon 表示対象の Hide（\\b[-1]）");
            assert!(reply.is_none(), "reply は None（撃ちっぱなし）");
        }
        other => panic!("1 件目は Hide{{balloon}} のはず: {}", variant_name(other)),
    }
    match &received[1] {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            binds,
            pattern,
            reply,
        } => {
            assert_eq!(*target, balloon_target(0), "2 件目は balloon 表示対象の ShowSurface（\\b[0]）");
            assert_eq!(*surface_id, 0, "surface_id は 0（\\b[0]・seriko 解決済み数値の透過）");
            assert_eq!(
                *binds,
                BindSet::default(),
                "binds は既定（空集合＝バルーン着せ替えなし・DD-5/R5.1）"
            );
            // 非退行（task 9.4・R5.4）: loop 不活性（Inert）経路の cue 由来 ShowSurface は pattern 寄与なし＝空
            // （PatternState 拡張前と観測等価）。ループを活性化する boot_live 経路でのみ pattern が載る。
            assert!(
                pattern.is_empty(),
                "loop 不活性経路の cue 由来 ShowSurface は pattern 空（従来と観測等価・R5.4）"
            );
            assert!(reply.is_none(), "reply は None（撃ちっぱなし）");
        }
        other => panic!("2 件目は ShowSurface{{balloon,0,default}} のはず: {}", variant_name(other)),
    }

    // 実 presenter へ apply（実描画→readback まで観測境界を延ばす・R8.2）。形状記録後に move で流す。
    let mut cmds = received.into_iter();
    let hide = cmds.next().expect("Hide");
    harness.wiring.apply_present(&mut harness.world, hide);
    // apply_hide は供給面を破棄しない → read_back は基準（surface0）のまま（可視フラグは readback に映らない）。
    let after_hide = harness
        .wiring
        .read_back_target(balloon_target(0))
        .expect("Hide 後も balloon 供給面は保持され read_back 可能");
    assert_eq!(
        after_hide, baseline,
        "apply_hide は swap chain 供給面を破棄しない（read_back は供給面直読み・可視フラグ非反映）"
    );

    let show = cmds.next().expect("ShowSurface");
    harness.wiring.apply_present(&mut harness.world, show);
    let after_show = harness
        .wiring
        .read_back_target(balloon_target(0))
        .expect("ShowSurface{0} 後の balloon read_back");
    assert!(
        opaque_count(&after_show) > 0,
        "ShowSurface{{0}} 適用後の balloon は非全透明（surface0 の実描画・R8.2）"
    );
    assert_eq!(
        after_show, baseline,
        "\\b[0]→ShowSurface{{balloon,0,default}} は attach 初期面（surface0）と同一バイトを再駆動する（surface_id/binds の貫通証跡）"
    );

    harness.shutdown_bounded();
}

/// spine S4（`\b` なし完走・R5.5・R1 系）: `\b` を含まない OnBoot 台本が S1 経路（boot→表示）を完走し、
/// かつ受信 `PresentCommand` 列に **balloon 表示対象（奇数 TargetId）宛の指令が一切現れない**
/// （＝`\b` 由来の面切替が無い）ことを固定する。emo2 fixture は balloons0.png のみで OnBoot デモは
/// バルーン面切替なしで完走する（R5.5）。`\s[0]` はシェル面指令（偶数 TargetId）を 1 件生むため、
/// 「talk が実際に流れたが balloon 面切替は無い」を受信列で決定論的に区別できる。
#[test]
fn spine_s4_balloon_free_onboot_completes_without_balloon_face_switch() {
    let mut harness = SpineHarness::boot(r"\s[0]\e"); // \b-free（シェル面 cue のみ）

    // S1 経路: attach 完走（planned==attached==2）＋ shell/balloon readback 非全透明。
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "S4: boot→表示（attach 完走・planned=2 attached=2）が観測できない: {logs:?}"
    );
    assert_eq!(count_level(&logs, "ERROR"), 0, "attach で ERROR なし: {logs:?}");
    // シェルは初回 `\s` cue まで非表示（defect #5・2026-07-13 実機#5）: attach 直後は供給面未生成
    // ＝`read_back` Err（合成面なし＝透過）。attach で surface0 を焼き付けない。
    assert!(
        harness.wiring.read_back_target(shell_target(0)).is_err(),
        "shell scope0 は初回 \\s cue 前は非表示であるべき（供給面未生成・read_back Err・defect #5）"
    );
    // バルーンは attach 初回表示（面0・文字層スロット取得のため保持）＝非全透明。
    let balloon_px = harness
        .wiring
        .read_back_target(balloon_target(0))
        .unwrap_or_else(|e| panic!("balloon scope0 の read_back 失敗: {e:?}"));
    assert!(
        opaque_count(&balloon_px) > 0,
        "balloon scope0 の readback が全透明（バルーン初期面が表示されていない）"
    );

    // OnBoot talk（\s[0]）を駆動し、少なくとも 1 件（シェル面指令）を受信＝talk が実際に流れたことを担保。
    let mut received = tick_and_collect(&mut harness, 1);
    assert!(
        !received.is_empty(),
        "\\b なし OnBoot 台本の talk が有界内に発火しない（boot→talk 経路が通っていない）"
    );
    // settle: さらに有界に Tick/drain して残余（万一の balloon 指令）を漏れなく回収する（sleep 不使用）。
    for now in 1_000_000u64..1_000_000 + 5_000 {
        harness.inject_dispatcher_tick(now);
        received.extend(harness.wiring.drain_received());
        std::thread::yield_now();
    }

    // R5.5: 受信列に balloon 表示対象（奇数 TargetId）宛は一切現れない（`\b` 由来面切替なしで完走）。
    for cmd in &received {
        if let Some(t) = present_command_target(cmd) {
            assert_eq!(
                t.0 % 2,
                0,
                "\\b なし台本で balloon 表示対象（奇数 TargetId）宛の指令が現れた（面切替 leak・R5.5 違反）: {:?} / variant={}",
                t,
                variant_name(cmd)
            );
        }
    }

    harness.shutdown_bounded();
}

// ===========================================================================
// task 6.3 spine 観測ケース（S2 talk→typewriter／S5 close 握手）
//
// 6.1 の `SpineHarness`＋6.2 の観測ヘルパの上に構築する。S2 は実 sink 経路の末端
// （seriko→shell 面切替の実描画 readback／emo-text typewriter の `present_frame` 駆動→
// text 供給面 readback）まで観測境界を延ばす（R8.2/R8.5）。sleep 不使用・注入 Tick と
// 注入 talk_time のみ（R8.3）・headless GPU（WARP・MTA・R8.4）・x64 完結（R8.6）。
// ===========================================================================

/// 装着済み balloon text actor の供給面（emo-text `TextLayerRuntime::surface`）の非透明画素数。
///
/// 未装着（供給面なし）は 0。S2 の typewriter リビール観測（`present_frame` 駆動後の text 供給面
/// readback）に使う。`harness.runtime` は `wiring.runtime` と同一 `Rc<RefCell<..>>`（clone）ゆえ、
/// `run_text_phase`（`present_frame`）が更新した供給面をそのまま読み戻せる（借用は逐次・非重複）。
fn text_surface_opaque(harness: &SpineHarness, actor: &ActorKey) -> usize {
    let rt = harness.runtime.borrow();
    match rt.surface(actor) {
        Some(surface) => surface
            .read_back()
            .map(|bytes| opaque_count(&bytes))
            .unwrap_or(0),
        None => 0,
    }
}

/// spine S2（talk→typewriter・R2.1/2.2/2.3/2.4・R3.1・R8.2/R8.5）: `\s[2100]`（シェル面切替）＋
/// テキスト＋`\c`（Clear）を含む scripted OnBoot 台本を実 sink 経路で流し、
/// (1) 受信 `PresentCommand` 列に `ShowSurface{shell_target(0), surface_id:2100}` が現れ、apply 後の
/// shell readback が**非表示から surface2100 の実描画へ遷移**すること（初回面表示の実描画・R2.4/R3.1・
/// defect #5 ゆえ attach 時の初期 surface0 baseline は無い＝シェルは初回 `\s` まで非表示）、
/// (2) テキスト cue の typewriter リビールを注入 `talk_time` の階段値で駆動し、text 供給面の
/// `opaque_count` が**単一 talk 内で単調非減少**・pre-reveal（t=0.0）全透明・`Clear`（at=0.95）後の
/// 全域透明（R8.5・R2 系の檻）を檻に入れる。
///
/// # 二段配送で単一 talk 内のリビール→Clear を分離（talk_clock 既知制約に整合）
///
/// emo-text の cue は**到着即時適用**（state.rs `apply_cue`）: `Text` は追記＋per-glyph リビール時刻
/// `r_i=max(r_{i-1}+interval, at)`（`interval = cue.duration / N`・配送 duration 由来＝
/// areka-P0-cue-playback-duration で `char_wait` を撤去）確定、`Clear` は**配送即時にバッファ全消去**（時刻ゲートではない）。
/// リビールの時刻ゲートは `visible(t)=|{i:r_i≤t}|` のみ。よって単調非減少の階段は「Text 配送済み・
/// Clear 未配送」のバッファに注入 `talk_time`（clock 非経由・R8.3）を振って観測し（Phase 1）、その後
/// dispatcher の elapsed を Clear（`\w[20]`＝at=1.05）超へ進めて Clear を配送し全消去を観測する
/// （Phase 2）。台本の `\w[1]`（Text at=0.05）により t=0.0 は先頭グリフ r_0=0.05 未達で全透明。単調
/// 述語の適用範囲を単一 talk 内（Clear 配送前のリビール区間）に限定することで、talk 跨ぎの epoch
/// リベース逆行（talk_clock 既知制約）を対象外にする（設計 Testing Strategy S2）。`present_frame` は
/// 各 t で全域再描画（残渣なし・決定論・R7.3）ゆえ、注入 t に対し `opaque_count` は `visible(t)` の
/// 単調性をそのまま反映する。
///
/// # validrect 外非透明なし（best-effort・CONCERNS 相当）
///
/// text 供給面は validrect 寸ちょうどのクリップ面（draw_readback_test が「readback は validrect 寸の
/// BGRA 密配列＝validrect 外の画素は供給面に存在しない」を単体で固定済み）であり、非透明画素は構造上
/// validrect 内に閉じる。本 spine は実 balloon fixture 由来の validrect 寸を再導出せず、単調非減少＋
/// Clear 後全透明（R2 系の本質）を主檻とし、validrect 外非透明なしは供給面クリップの構造的帰結として
/// draw_readback_test の単体檻に委ねる（parent 指示の best-effort）。
#[test]
fn spine_s2_talk_drives_surface_switch_and_typewriter_reveal() {
    // \s[2100]（シェル面切替・actor "0"）→ \w[1] 後にテキスト（typewriter・at=0.05）→ \w[20] 後に
    // \c（Clear・at=1.05）。Text と Clear の間に大きな待ちを置き、二段配送（Text のみ→Clear）で
    // 「単一 talk 内のリビール」→「Clear の全消去」を分離できるようにする。
    let mut harness = SpineHarness::boot(r"\s[2100]\w[1]アヒルやアヒル\w[20]\c\e");
    let actor = ActorKey::from("0");

    // ── attach（shell/balloon 装着・text actor 登録）: S1 と同じ planned==attached==2 前提 ──
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "S2 前提: attach 完了（planned=2 attached=2）が観測できない: {logs:?}"
    );
    assert_eq!(count_level(&logs, "ERROR"), 0, "attach で ERROR なし: {logs:?}");

    // ── シェルは初回 `\s` cue まで非表示（defect #5・2026-07-13 実機#5）: `\s[2100]` 適用前の shell
    //    scope0 は供給面未生成＝`read_back` Err（合成面なし＝透過）。attach で surface0 を焼き付けない。 ──
    assert!(
        harness.wiring.read_back_target(shell_target(0)).is_err(),
        "S2 前提: shell scope0 は初回 \\s cue 前は非表示（供給面未生成・read_back Err・defect #5）"
    );

    // ── Phase 1: シェル面指令（\s[2100]）＋テキスト cue **のみ**を配送する。dispatcher は now→
    //    elapsed=(now-base)/1000 換算で start_time 順に cue を解放する（Emote@0.0→Text@0.05→
    //    Clear@1.05）。テキスト（at=0.05）到達直後で打ち切り、Clear（at=1.05）を**まだ解放させない**
    //    （Clear は配送即時にバッファ全消去＝時刻ゲートではない・state.rs apply_cue）。elapsed を小刻み
    //    （5ms/反復）に進め、テキスト到達で即 break することで Clear 域（1.05s）へ到達しない。 ──
    let mut show_cmds: Vec<PresentCommand> = Vec::new();
    let mut now = 0u64;
    let mut text_reached = false;
    for _ in 0..100_000u32 {
        now += 5; // 小刻みで進め、テキスト到達（at=0.05）直後に打ち切る（Clear=at1.05 へ到達しない）
        harness.inject_dispatcher_tick(now);
        show_cmds.extend(harness.wiring.drain_received());
        harness.pump_text();
        // テキスト cue 到達確認: 完全リビール域 t=0.30 で非透明になれば runtime へ流入済み。
        run_text_phase(&mut harness.wiring, &mut harness.world, Some(0.30));
        if !show_cmds.is_empty() && text_surface_opaque(&harness, &actor) > 0 {
            text_reached = true;
            break;
        }
    }
    assert!(
        text_reached,
        "S2: シェル面指令＋テキスト cue が有界内に runtime へ到達しない（boot→talk→sink 経路不通）"
    );

    // ── (1) シェル面切替（R2.4/R3.1）: 受信列に ShowSurface{shell_target(0),2100} が現れる ──
    let idx = match show_cmds.iter().position(|c| {
        matches!(c, PresentCommand::ShowSurface { target, surface_id, .. }
            if *target == shell_target(0) && *surface_id == 2100)
    }) {
        Some(i) => i,
        None => panic!(
            "S2: \\s[2100] のシェル面切替 ShowSurface{{shell_target(0),2100}} が受信列に無い: variants={:?}",
            show_cmds.iter().map(variant_name).collect::<Vec<_>>()
        ),
    };
    let show = show_cmds.remove(idx);
    match &show {
        PresentCommand::ShowSurface {
            target,
            surface_id,
            reply,
            ..
        } => {
            assert_eq!(*target, shell_target(0), "シェル表示対象（偶数 TargetId・DD-3）");
            assert_eq!(*surface_id, 2100, "surface_id は 2100（\\s[2100]・seriko 数値解決の透過）");
            assert!(reply.is_none(), "reply は None（撃ちっぱなし）");
        }
        _ => unreachable!("position で ShowSurface を選別済み"),
    }

    // apply 後、shell が非表示から surface2100 の実描画へ遷移する（初回面表示の実描画まで観測・R8.2）。
    // defect #5 ゆえ attach 時の初期 surface0 baseline は存在せず、hidden（read_back Err）→shown
    // （opaque_count>0）が `\s[2100]` の実描画の証跡になる。
    harness.wiring.apply_present(&mut harness.world, show);
    let after_switch = harness
        .wiring
        .read_back_target(shell_target(0))
        .expect("\\s[2100] 適用後は shell scope0 の供給面が生成され read_back 可能");
    assert!(
        opaque_count(&after_switch) > 0,
        "S2: \\s[2100] 適用で shell scope0 が surface2100 の実描画へ遷移（非表示→非全透明・R3.1/R8.2）"
    );

    // ── (2) typewriter（R2.2/2.3/R8.5）: Clear 未配送（テキストのみの単一 talk バッファ）に対し注入
    //    talk_time 階段で present_frame を駆動し、text 供給面の opaque_count が単調非減少であること・
    //    pre-reveal（t=0.0＜先頭グリフ r_0=0.05）が全透明であることを固定する。 ──
    let staircase: Vec<f64> = (0..=18).map(|i| i as f64 * 0.05).collect(); // 0.00,0.05,...,0.90
    let mut counts: Vec<usize> = Vec::with_capacity(staircase.len());
    for &t in &staircase {
        run_text_phase(&mut harness.wiring, &mut harness.world, Some(t));
        counts.push(text_surface_opaque(&harness, &actor));
    }
    assert_eq!(
        counts[0], 0,
        "S2: pre-reveal（t=0.0＜先頭グリフ r_0=0.05）は text 供給面が全透明（opaque_count==0）: {counts:?}"
    );
    for i in 1..counts.len() {
        assert!(
            counts[i] >= counts[i - 1],
            "S2: typewriter の opaque_count は単一 talk 内で単調非減少（注入 talk_time 階段）: {counts:?}"
        );
    }
    assert!(
        *counts.last().expect("staircase は非空") > 0,
        "S2: リビールが実際に進行し text が描画される（末尾 t=0.90 で非全透明・非空虚な檻）: {counts:?}"
    );

    // ── Phase 2＋Clear 後全域透明（R8.5）: dispatcher elapsed を Clear（at=1.05）超へ進めて Clear を
    //    配送する。Clear は配送即時にバッファを全消去する（state.rs apply_cue）ため、Clear 配送後は
    //    どの注入 talk_time でも text 供給面が全域透明（premultiplied 全 0）へ戻る。配送前は
    //    present_frame(2.0)＝全リビール（非透明）ゆえ、0 への遷移が Clear 到達の観測点になる。 ──
    let mut clear_reached = false;
    for _ in 0..100_000u32 {
        now += 50; // 大きめに進め Clear（at=1.05s）を配送させる
        harness.inject_dispatcher_tick(now);
        harness.pump_text();
        run_text_phase(&mut harness.wiring, &mut harness.world, Some(2.0));
        if text_surface_opaque(&harness, &actor) == 0 {
            clear_reached = true;
            break;
        }
    }
    assert!(
        clear_reached,
        "S2: Clear cue が有界内に runtime へ到達しない（\\w[20]\\c の配送が完了しない）"
    );
    // Clear 配送後は「リビール済みだった」区間の talk_time（t=0.30）でも全域透明（Clear の全消去・R8.5）。
    run_text_phase(&mut harness.wiring, &mut harness.world, Some(0.30));
    assert_eq!(
        text_surface_opaque(&harness, &actor),
        0,
        "S2: Clear 配送後は t=0.30（リビール域）でも text 供給面が全域透明（Clear の全消去・R8.5）"
    );

    harness.shutdown_bounded();
}

/// spine S5（close 握手・R6.1/6.2/6.3・R8.3）: `shutdown(CloseReason::User)`（ForceQuit 経路＝
/// OnClose NOTIFY→Unload）で ghost 一式を畳み、(a) OnClose 台本が消化され（`ScriptedShioriHandle` に
/// `Notify{OnClose}`→`Unload` が順に記録される）、(b) `shutdown` が有界時間で `Ok` を返し、(c) seriko
/// worker（＋ghost 内部の全ハンドルは `shutdown` が内部 join）が有界 join で完了する（timeout=panic
/// ゆえ hang すれば test FAIL）ことを固定する（設計 Testing Strategy S5・ghost spine S 系の手法）。
///
/// # ForceQuit 経路の OnClose は NOTIFY（GET でない）
///
/// `GhostRuntime::shutdown` は常に ForceQuit 横断遷移で終了する（close talk を発行しない）。ゆえに
/// OnClose は片道 NOTIFY として消化され（標準台本 `SpineHarness::boot` が `notify("OnClose")` を
/// 台本化済み）、続けて正規 clean shutdown の `Unload`（`Ok(ExitKind::Clean)`）が呼ばれる。close talk
/// 駆動の Quit 経路（OnClose GET→`\-`）は ghost spine S4 の担当領域であり、本 spine の主眼は
/// 「areka 側の実 sink 結線（seriko 含む）が shutdown で hang せず有界 join する」ことの檻に置く。
#[test]
fn spine_s5_close_handshake_consumes_onclose_and_joins_all_handles_bounded() {
    // 標準台本（OnClose NOTIFY＋Unload(Clean)）で boot。最小 OnBoot talk（\s[0]\e）。
    let harness = SpineHarness::boot(r"\s[0]\e");

    // boot 系列（非 Status 4 呼出）が届くまで有界スピン（OnClose を boot ノイズと分離・sleep 不使用）。
    let mut boot_calls = Vec::new();
    for _ in 0..100_000u32 {
        boot_calls = harness.shiori_handle.non_status_calls();
        if boot_calls.len() >= 4 {
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        boot_calls.len() >= 4,
        "S5 前提: boot 系列 4 呼出が有界内に発火する: {boot_calls:?}"
    );

    // 分解して所有ハンドルを得る（shutdown_bounded と同型・shiori_handle は照合のため保持）。
    let SpineHarness {
        world,
        wiring,
        runtime,
        ghost,
        seriko,
        shiori_handle,
        text_pump,
        tick_sink,
    } = harness;

    // (b) shutdown(User) が有界時間で Ok を返す（hang しない・ForceQuit→OnClose NOTIFY→Unload）。
    run_bounded("spine s5 ghost shutdown", Duration::from_secs(10), move || {
        let result = ghost.shutdown(CloseReason::User);
        assert!(
            result.is_ok(),
            "S5: shutdown は close 握手後 Ok を返す（正規 clean shutdown）: {result:?}"
        );
    });

    // loop tick 直接注入端の clone を明示 drop（task 9.4）: seriko の全 SerikoSink Sender（dispatcher 保持分は
    // shutdown が drop 済み）を落とし切って inbox を切断し worker を自然終了させる（join 前・shutdown_bounded と同旨）。
    drop(tick_sink);

    // (c) seriko worker が有界 join で完了する（timeout=panic ゆえ hang すれば test FAIL・R8.3）。
    // shutdown が ghost 一式を join→dispatcher 保持の SerikoSink クローンを drop→seriko inbox 切断→
    // 自然終了、という連鎖の末端をここで有界 join して観測する。
    join_bounded("spine s5 seriko join", Duration::from_secs(10), seriko).expect(
        "S5: seriko worker は shutdown 後、SerikoSink クローン全 drop で有界時間内に終了する",
    );

    // (a) OnClose 台本消化: Notify{OnClose}→Unload が順に記録される（ForceQuit close 握手→clean unload）。
    let calls = shiori_handle.non_status_calls();
    let onclose_idx = calls
        .iter()
        .position(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "OnClose"));
    let unload_idx = calls.iter().position(|c| matches!(c, RecordedCall::Unload));
    assert!(
        onclose_idx.is_some(),
        "S5: OnClose NOTIFY が消化される（ForceQuit close 握手）: {calls:?}"
    );
    assert!(
        unload_idx.is_some(),
        "S5: Unload が呼ばれる（正規 clean shutdown）: {calls:?}"
    );
    assert!(
        onclose_idx < unload_idx,
        "S5: OnClose→Unload の順（close 握手→unload）: {calls:?}"
    );

    // 残り（!Send・テストスレッド常駐）を明示 drop（presenter/World/Rc runtime/UI アクター）。
    drop(wiring);
    drop(world);
    drop(runtime);
    drop(text_pump);
}

// ===========================================================================
// task 9.3 — move cue の決定論 spine e2e（cue→CueSheet→dispatch→broadcast→実 MoveCueSink→
// move channel→frame 相 drain→apply→実窓移動）。
//
// 9.1 が置いた throwaway `(_move_tx, move_rx)` を実 `MoveCueSink`（`SpineHarness::boot_with` の
// sinks 第 3 要素）へ差し替えた S-3 形（production `wire_emo2_boot` の 3-sink 構成）の上で、
// `\1\![move,...]` を含む OnBoot talk を実 sink 経路で流し、`MoveDirective` が move channel へ届き
// frame 相 drain（`run_move_drain_phase`＝task 9.2）で対象窓が fixture 検算位置へ即時移動することを
// 固定する。9.2 の frame 相配線が spine で end-to-end に生きていることの自動檻（headless・sleep 不使用・
// 注入 Tick のみ・手動実機確認は Task 11 に一本化）。
// ===========================================================================

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム・follow.rs/frame.rs の fake_handle 相当）。
fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// spine World の各キャラ／バルーン窓へ偽 WindowHandle を付与する（`enqueue_window_set_pos` が
/// WindowPos を書ける条件＝WindowHandle 実在。`spawn_ghost_windows` は実窓生成前ゆえ handle 未付与で、
/// これが無いと `move_window_to` は warn＋no-op に縮退し窓が動かない）。
fn attach_fake_window_handles(world: &mut World, gw: &GhostWindows) {
    let mut raw = 0x100usize;
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).unwrap(),
            gw.balloon_window(scope).unwrap(),
        ] {
            world.entity_mut(e).insert(fake_handle(raw));
            raw += 0x10;
        }
    }
}

/// entity の WindowPos.position を読む（未設定は panic で検出）。
fn window_position(world: &World, e: Entity) -> Point {
    world
        .get::<WindowPos>(e)
        .expect("WindowPos があるはず")
        .position
        .expect("position があるはず")
}

/// spine move e2e（R5.1/R8.1・DD・task 9.3）: fixture 形の move script を含む OnBoot talk を実 sink 経路
/// （ghost→sakura compile→CueSheet→dispatch→broadcast→**実 MoveCueSink**→move channel）で流し、frame 相
/// drain（`run_move_drain_phase`＝task 9.2）が `MoveDirective` を drain して対象キャラ窓を検算位置へ即時
/// 移動させることを固定する。9.1 が置いた throwaway 送出端を実 MoveCueSink（sinks 第 3 要素・S-3 形）へ
/// 差し替えた配線が spine で end-to-end に生きていることの自動檻（headless・sleep 不使用・注入 Tick のみ・
/// R8.3/8.4/8.6）。窓が実際に動く＝`MoveDirective` が channel へ届き drain→apply された唯一の経路ゆえ、
/// 移動観測が「channel 到達＋frame 相 drain の live」を同時に証跡する。
///
/// # `\1` は正典どおり scope1（エモ＝相方）へ切替（観測 scope は 1・R4.4）
///
/// fixture は `\1\![move,-353,,,0,base,base]`（kero=scope1 を sakura=scope0 基準で動かす意図）で、
/// **bare `\1` は正典どおり sakura compile で `SpeakerScope{1}` へ写像される**（Task 12.1 で
/// `decode.rs`／lexer が `\0`/`\1` を SpeakerScope へ正規化・以前の `Raw` passthrough 縮退は解消済み）。
/// ゆえに move cue の scope は切替後の 1 として発火し（`cue.actor == "1"` → `MoveDirective.scope == 1`）、
/// base は `0`＝**scope0（むらさき＝話者）を基準にした scope1 の移動**として反映される（対象＝scope1 char 窓・
/// 基準＝scope0 char 窓）。実 channel 到達 directive は
/// `MoveDirective{ scope:1, x:Px(-353), y:Fix, base:Scope(0) }`。この e2e が `\1` の正典スコープ切替を
/// parse→compile→cue.actor→MoveDirective.scope→対象窓解決まで end-to-end に固定する。
///
/// # 検算（`two_scope_placements`・全て物理 px・R-6）
///
/// 対象＝scope1 pos(1049,1063) size(278,357)・基準＝scope0 pos(1483,733) size(434,687)・x=Px(-353)・y=Fix。
/// `CanonDefaultBasepos`（x=幅÷2）で
/// x' = base_pos.x + basepos(base窓).x + dx − basepos(対象窓).x
///    = 1483 + 434/2 − 353 − 278/2 = 1483 + 217 − 353 − 139 = 1208・
/// y は Fix ゆえ対象窓（scope1）の現状維持 1063。移動先 (1208,1063) は move cue が channel→drain→apply を
/// 通ったことの非空虚な証跡（RED では窓不動）。
///
/// # RED（実 MoveCueSink 未配線時）
///
/// 9.1 の throwaway `(_move_tx, move_rx)`（送出端即 drop・sinks に MoveCueSink なし）では move cue は
/// seriko/text sink へのみ broadcast され両者が良性スキップ→move channel は空のまま→窓は不動（moved=false
/// で FAIL・実測済み）。実 MoveCueSink を 3rd sink へ配線して初めて窓が動く（GREEN）。
#[test]
fn spine_move_cue_drives_window_move_end_to_end() {
    // fixture 形の move script（`\1` は正典どおり scope1 へ切替・doc 参照）。bare `\1` は Task 12.1 で
    // SpeakerScope へ写像されるため実 SHIORI 由来の現実的入力＝正典スコープ切替を e2e 検証する。
    let mut harness = SpineHarness::boot(r"\1\![move,-353,,,0,base,base]\e");

    // GhostWindows（`boot_with` が spawn_ghost_windows で資源挿入）から対象 char 窓を引き、実窓生成前ゆえ
    // 未付与の WindowHandle を偽装付与する（move_window_to の反映口 enqueue_window_set_pos の成立条件）。
    let gw = harness
        .world
        .get_resource::<GhostWindows>()
        .expect("spine World には GhostWindows が挿入済み")
        .clone();
    attach_fake_window_handles(&mut harness.world, &gw);
    // 観測 scope は 1（`\1` が正典どおり scope1 へ切替・doc 参照）。対象＝scope1（エモ）char 窓・基準＝scope0。
    let target = gw.char_window(1).expect("scope1（エモ＝相方）の char 窓");

    // 移動前の初期位置（two_scope_placements の scope1 char_pos）。
    let baseline = window_position(&harness.world, target);
    assert_eq!(
        baseline,
        Point { x: 1049, y: 1063 },
        "前提: 移動前の scope1 初期位置（two_scope_placements）"
    );

    // OnBoot talk を Tick 注入で駆動し、各反復で実 frame 相 move drain を回す。move cue は at=0.0 ゆえ talk
    // 起動後の最初の有効 Tick で発火するが、boot→compile→dispatch→broadcast はスレッド群を跨いで非同期に
    // 流れるため、窓が動く（channel→drain→apply 完了）まで有界スピン（sleep 不使用・yield_now のみ）で待つ。
    let mut moved = false;
    for now in 1u64..=200_000 {
        harness.inject_dispatcher_tick(now);
        // 実 frame 相 drain（task 9.2）: move channel を try_iter し apply_move_directive で即時反映。
        run_move_drain_phase(&harness.wiring, &mut harness.world);
        if window_position(&harness.world, target) != baseline {
            moved = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        moved,
        "move cue が有界内に channel→frame drain→apply を通って対象窓を動かさない（実 MoveCueSink 配線が死んでいる？）"
    );

    // 検算位置（scope0 基準・CanonDefaultBasepos）へ即時移動＝MoveDirective が channel へ届き drain→apply された
    // 非空虚な証跡（R5.1・9.2 の frame 相配線が spine で生きている）。
    assert_eq!(
        window_position(&harness.world, target),
        Point { x: 1208, y: 1063 },
        "x'=1483+217−353−139=1208（base=scope0 基準・CanonDefaultBasepos）・y=Fix は現状維持 1063（cue→channel→frame drain→apply→実窓移動）"
    );

    // 二重適用なし: move cue は 1 発ゆえ、追加 drain で窓はさらに動かない（channel は drain 済みで空）。
    run_move_drain_phase(&harness.wiring, &mut harness.world);
    assert_eq!(
        window_position(&harness.world, target),
        Point { x: 1208, y: 1063 },
        "move channel は drain 済みで空（二重適用なし・FIFO 全件消費）"
    );

    harness.shutdown_bounded();
}

// ===========================================================================
// task 9.4 — まばたきスモーク（direct send_tick 注入 → SERIKO ループ → pattern 搬送指令）
//
// 本 task の主眼はハーネスの ripple 修正（spawn_seriko arity／loop_tables／ShowSurface.pattern）＋
// 直接 tick 注入配線（`SerikoSink::send_tick`）＋既存テスト非退行（loop 不活性経路）だが、direct
// send_tick が実 emo2 表のループへ届き pattern を載せた表示指令を生む配線を最小スモークで裏付ける。
// 実 kero/sakura まばたき 1 周の PresentCommand 列 golden（2106→2110→-1→ベース復帰）と R3.4 default-OFF
// 対照は task 10.2 が本スモークの上に構築する（本 task では作らない）。
// ===========================================================================

/// 常時発火 rng（1/N 抽選で必ず 0 を返す＝毎境界で抽選通過・actor.rs／looper.rs `always_fire` と同旨）。
///
/// 「固定注入乱数列」の最小形（発見的 entropy 非依存・R7.1/7.2）。まばたきスモークは「発火が起きること」
/// のみを檻に入れるため定数 0 で足りる（発火順序・回数を厳密に固定する full golden は task 10.2）。
fn always_fire_rng() -> LoopRng {
    Box::new(|_bound: u32| 0)
}

/// spine まばたきスモーク（R7.1/7.2/7.3・DD・task 9.4）: 実 emo2 shell 表＋固定注入乱数（常時発火）で
/// `boot_live` し、kero まばたきの `interval,random,4` アニメ（pattern0=2106／pattern1=2110／pattern3=-1）を
/// 持つ surface 2100 を `\s[2100]` で表示させたのち、`SerikoSink::send_tick` を**直接注入**（loop ticker
/// 不起動・sleep 不使用）して 1000ms 絶対グリッド境界を跨がせ、seriko ループが **pattern を載せた**
/// ShowSurface{shell_target(0),2100} を PresentBridge→rx へ発行することを固定する。direct send_tick →
/// LoopRuntime → emit_display → adapter → rx の end-to-end 配線が spine で生きていることの最小自動檻。
///
/// # 表示中ゲート（loop は Show 済み slot のみ評価・R6.1/2.1）
///
/// ループは表示中の slot に対してのみアニメ評価する。まず `\s[2100]` cue を dispatcher tick で駆動し
/// rx に ShowSurface{2100} が現れる（＝seriko が ScopeStates に scope0 shell=surface2100 を記録済み）まで
/// 待ってから send_tick を注入する。dispatcher tick は talk/cue clock を進めるのみで seriko ループには
/// 届かない（ghost 側にループ結線なし）ため、ループ発火はこの直接注入 send_tick だけが供給する。
///
/// # 決定論（sleep 不使用・注入時刻＋注入乱数のみ・R7.2/7.3）
///
/// 起動 tick（now=0・境界初期化・非跨ぎ・無発行）→ 40ms 刻みで進め、境界跨ぎ（1000/2000/…）で常時発火 rng が
/// 抽選通過→pattern 進行。boot→talk→sink やスレッド伝播の非同期遅延は有界スピン（`yield_now` のみ）で吸収する。
#[test]
fn spine_blink_smoke_send_tick_drives_loop_pattern_command() {
    // 実表＋常時発火固定 rng でループ活性化（既存テストは Inert＝非退行・本テストのみ Live）。
    let mut harness = SpineHarness::boot_live(r"\s[2100]\e", always_fire_rng());

    // surface2100（kero まばたき random,4）を表示させる: \s[2100] cue を dispatcher tick で駆動し、
    // rx に shell ShowSurface{2100} が現れる（＝seriko が表示中 slot を記録済み）まで有界スピン。
    let mut shown = false;
    for now in 1u64..=200_000 {
        harness.inject_dispatcher_tick(now);
        for cmd in harness.wiring.drain_received() {
            if matches!(&cmd, PresentCommand::ShowSurface { target, surface_id, .. }
                if *target == shell_target(0) && *surface_id == 2100)
            {
                shown = true;
            }
        }
        if shown {
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        shown,
        "\\s[2100]（kero まばたき surface）の初回シェル表示が有界内に rx へ現れない（表示中ゲート前提不成立）"
    );

    // send_tick を直接注入して 1000ms 絶対グリッド境界を跨がせる（loop ticker 不起動・sleep 不使用）。
    // pattern を載せた ShowSurface{shell_target(0),2100} が現れるまで有界スピン（sub 秒進行＋境界跨ぎの双方を送る）。
    let mut pattern_carrying = false;
    let mut now = 0u64;
    harness.inject_seriko_tick(now); // 起動 tick（境界初期化・非跨ぎ・無発行）
    for _ in 0..100_000u32 {
        now += 40; // 小刻みに進め境界跨ぎ（1000ms グリッド）と pattern 進行（sub 秒）の双方を供給
        harness.inject_seriko_tick(now);
        for cmd in harness.wiring.drain_received() {
            if let PresentCommand::ShowSurface {
                target,
                surface_id,
                pattern,
                ..
            } = &cmd
            {
                if *target == shell_target(0) && *surface_id == 2100 && !pattern.is_empty() {
                    pattern_carrying = true;
                }
            }
        }
        if pattern_carrying {
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        pattern_carrying,
        "direct send_tick が実 emo2 ループを駆動して pattern 搬送 ShowSurface{{shell_target(0),2100}} を発行しない（send_tick→LoopRuntime→emit→adapter→rx 配線が死んでいる？）"
    );

    harness.shutdown_bounded();
}
