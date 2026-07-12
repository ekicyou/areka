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
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::{spawn_emo_text, TextLayerRuntime};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{boot, GhostBootOptions, GhostRuntime, ShioriWiring, TickerMode};
use areka_kanade::{CloseReason, MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_seriko::{spawn_seriko, SurfaceResolver};
use bevy_ecs::world::World;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use tracing::field::{Field, Visit};
use tracing_subscriber::prelude::*;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{GraphicsCore, WucGraphicsResource};
use wintf::executor::{FilterResult, JoinHandle, MessageLoop};

use crate::placement::resolver::{PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;

use super::adapter::PresentBridge;
use super::assets::{build_boot_assets, BootAssets};
use super::frame::{run_attach_phase, Emo2Wiring};
use super::talk_clock::{ClockedTextSink, TalkClock};

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
    fn get(&mut self, id: &str, references: &[String]) -> Result<Option<String>, RequestError> {
        self.calls.lock().expect("calls mutex poisoned").push(RecordedCall::Get {
            id: id.to_string(),
            references: references.to_vec(),
        });
        self.get_scripts.get_mut(id).and_then(VecDeque::pop_front).unwrap_or_else(|| {
            panic!("ScriptedShioriBackend::get(\"{id}\"): no scripted response left")
        })
    }

    fn notify(&mut self, id: &str, references: &[String]) -> Result<(), RequestError> {
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
            bottom_snap: true,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1063 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1044 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            bottom_snap: true,
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
}

impl SpineHarness {
    /// 標準台本（boot 系列＋`\s[0]\e` OnBoot＋ForceQuit close 系列）で spine ハーネスを起動する。
    ///
    /// `on_boot` は OnBoot GET が返す応答スクリプト。6.1 は最小 talk（`\s[0]\e`）を渡す。
    fn boot(on_boot: &str) -> SpineHarness {
        // 標準台本: boot 系列（OnInitialize→OnFirstBoot→OnBoot→basewareversion）＋
        // shutdown（`GhostRuntime::shutdown` は常に ForceQuit 経路＝OnClose NOTIFY→Unload・
        // ghost spine S1 と同旨）を台本化する。OnSecondChange は kanade へ Tick を送らないため不要。
        let (backend, shiori_handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(on_boot.to_string())))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
            .build();
        Self::boot_with(backend, shiori_handle)
    }

    /// 任意の scripted backend で spine ハーネスを起動する（6.2/6.3 が独自台本を注入するための口）。
    fn boot_with(backend: ScriptedShioriBackend, shiori_handle: ScriptedShioriHandle) -> SpineHarness {
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
        let BootAssets { shells, balloons, balloon_model, resolver, static_binds } = assets;
        let (surface_sink, seriko) = spawn_seriko(resolver, static_binds.clone(), bridge);
        let wiring_assets = BootAssets {
            shells,
            balloons,
            balloon_model,
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds,
        };

        // ── scripted boot（実 sink 注入・TickerMode::Disabled＝Tick 注入で駆動・R8.3） ──
        let options = GhostBootOptions {
            ghost_root: emo2_root(),
            default_encoding: DefaultEncoding::Ansi,
            shiori: ShioriWiring::Custom(Box::new(move || Ok(Box::new(backend) as Box<dyn ShioriBackend>))),
            surface_sink,
            text_sink: clocked_text_sink,
            ticker: TickerMode::Disabled,
        };
        let ghost = boot(options).expect("scripted boot は解決可能な emo2 ghost_root で成功する");

        // ── frame 三相結線状態（wire_emo2_boot 手順6 相当・System 登録はせず直接駆動する） ──
        let wiring = Emo2Wiring::new(presenter, rx, Rc::clone(&runtime), clock, wiring_assets);

        SpineHarness { world, wiring, runtime, ghost, seriko, shiori_handle, text_pump }
    }

    /// 文字層 UI アクターの pending メッセージを headless に drain する（実 ClockedTextSink 経路）。
    fn pump_text(&self) {
        pump_until_idle();
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
        let SpineHarness { world, wiring, runtime, ghost, seriko, shiori_handle, text_pump } = self;

        run_bounded("spine ghost shutdown", Duration::from_secs(10), move || {
            // 正規 close（DD-10 と同じ User）。ForceQuit ゆえ OnClose は NOTIFY で消化される。
            let _ = ghost.shutdown(CloseReason::User);
        });
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
