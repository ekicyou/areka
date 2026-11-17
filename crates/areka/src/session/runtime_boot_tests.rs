//! 実行系の側の起動（`boot_runtime`）の決定論テスト（areka-P0-mcp-reload 要件 4.2・13.4）。
//!
//! 確かめること: 束から 13 本の sink が今の並びで組まれ、12 本目が読み直しの受け口・13 本目が
//! 常に手前の受け口（線へ届く）であること・
//! 起動に失敗したとき World が変わらないこと・成功したときだけ受信端 5 つが World に据わること。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use areka_emo_present::PresentCommand;
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootError, GhostRuntime, ShioriWiring, TickerMode};
use areka_kanade::{BootOrigin, CloseReason, MonotonicMs, ShioriBackend};
use areka_seriko::{BindResolver, SerikoLoopConfig, SurfaceResolver, spawn_seriko};
use bevy_ecs::world::World;
use dola::cue::{ActorKey, CueCommand, TalkCue};

use super::*;
use crate::input_events::user_break::UserBreakWiring;
use crate::readme::ReadmeWiring;
use crate::sample_test_support::acquire_emo2;
use crate::session::ghost_switch::ChangeRx;
use crate::session::shell_balloon_switch::SwitchRx;
use areka_runtime::RuntimeInputs;
use areka_runtime::adapter::PresentBridge;
use areka_runtime::reload_cue::ReloadRx;
use areka_runtime::spine::{
    Progress, ScriptedShioriBackend, ScriptedShioriHandle, run_bounded_watching, wait_until,
};
use areka_runtime::talk_clock::TalkClock;
use areka_runtime::user_break_cue::NoUserBreakSignal;
use sample_ghost_kit::SampleRoot;

/// 窓の側の束の持ち物（seriko と文字の線を生かしておく）。
struct Rig {
    sources: CueSinkSources,
    _present_rx: Receiver<PresentCommand>,
    _move_rx: Receiver<areka_runtime::move_cue::PositionCue>,
    _lifecycle_rx: Receiver<areka_runtime::talk_lifecycle::TalkLifecycleSignal>,
    _zorder_rx: Receiver<areka_runtime::zorder_cue::ZOrderDirective>,
    /// 常に手前の線の受信端（窓の側が 1 度だけ作る・読み直しで組み直した受け口も同じ線へ送る）。
    window_state_rx: Receiver<areka_runtime::window_state_cue::WindowStateDirective>,
    _text: Box<dyn std::any::Any>,
    _runtime: Rc<RefCell<TextLayerRuntime>>,
}

/// 本物の seriko と文字の線から束を組む（窓は要らない）。
fn rig() -> Rig {
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let (text, text_pump) = spawn_emo_text(Rc::clone(&runtime));
    let (present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    let (surface, _seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        areka_seriko::DefaultBinds::uniform(areka_emo_compose::BindSet::default()),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        PresentBridge::new(present_tx),
    );
    let (move_tx, move_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
    let (zorder_tx, zorder_rx) = mpsc::channel();
    let (window_state_tx, window_state_rx) = mpsc::channel();
    Rig {
        sources: CueSinkSources {
            surface,
            text,
            clock: TalkClock::new(Arc::new(dola::runtime::clock::now)),
            move_tx,
            lifecycle_tx,
            zorder_tx,
            window_state_tx,
        },
        _present_rx: present_rx,
        _move_rx: move_rx,
        _lifecycle_rx: lifecycle_rx,
        _zorder_rx: zorder_rx,
        window_state_rx,
        _text: Box::new(text_pump),
        _runtime: runtime,
    }
}

/// 受信端 5 つ（readme・中断・ghost の切替・shell/balloon の切替・読み直し）のうち World に在る数。
fn installed(world: &World) -> usize {
    [
        world.get_non_send::<ReadmeWiring>().is_some(),
        world.get_non_send::<UserBreakWiring>().is_some(),
        world.get_non_send::<ChangeRx>().is_some(),
        world.get_non_send::<SwitchRx>().is_some(),
        world.get_non_send::<ReloadRx>().is_some(),
    ]
    .into_iter()
    .filter(|b| *b)
    .count()
}

/// 束から 13 本が組まれ、12 本目が読み直しの受け口・13 本目が常に手前の受け口（どちらも
/// 受信端へ届く）である。
#[test]
fn sinks_are_thirteen_with_reload_then_window_state_last() {
    let rig = rig();
    let world = World::new();
    let (mut sinks, lines) = runtime_sinks(&world, &rig.sources);
    assert_eq!(
        sinks.len(),
        13,
        "11 本の今の並びの後ろに読み直しの受け口、その後ろに常に手前の受け口"
    );

    let cue = TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier("reload", vec!["ghost".to_string()]),
        duration: 0.0,
    };
    sinks[11].emit(cue);
    assert_eq!(
        lines.reload_rx.try_iter().count(),
        1,
        "12 本目の受け口は読み直しの受信端へ送る"
    );

    let cue = TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(
            "set",
            vec!["windowstate".to_string(), "stayontop".to_string()],
        ),
        duration: 0.0,
    };
    sinks.last_mut().expect("13 本目").emit(cue);
    assert_eq!(
        rig.window_state_rx.try_iter().count(),
        1,
        "13 本目の受け口は常に手前の受信端へ送る"
    );
    assert_eq!(
        lines.reload_rx.try_iter().count(),
        0,
        "常に手前の指令は読み直しの線へ行かない"
    );
}

/// SHIORI だけの読み直しで受け口を組み直しても、常に手前の指令は窓の側の同じ線に届く
/// （送出端は束が持つ・stay-on-top-directives 要件 4.3）。
#[test]
fn rebuilt_window_state_sink_reaches_the_same_line() {
    let rig = rig();
    let world = World::new();
    let on = || TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(
            "set",
            vec!["windowstate".to_string(), "stayontop".to_string()],
        ),
        duration: 0.0,
    };

    let (mut first, _) = runtime_sinks(&world, &rig.sources);
    first.last_mut().expect("13 本目").emit(on());
    // 読み直し: 前の受け口を落とし、同じ束から組み直す。
    drop(first);
    let (mut rebuilt, _) = runtime_sinks(&world, &rig.sources);
    rebuilt.last_mut().expect("13 本目").emit(on());

    assert_eq!(
        rig.window_state_rx.try_iter().count(),
        2,
        "組み直す前と後の受け口が同じ受信端へ送る"
    );
}

/// 窓の在るゴーストの起動の選択肢（本番と spine が共に使う）には strict の判定が入る
/// （無いと strict の記録が無言で 0 件になる・spec mcp-strict-errors 要件 2.7）。
#[test]
fn runtime_boot_options_carry_strict_judges() {
    let options = runtime_boot_options(
        PathBuf::from("runtime_boot_tests/使わない/ghost"),
        ShioriWiring::Helper {
            helper_exe: PathBuf::from("runtime_boot_tests/使わない/helper.exe"),
        },
        Vec::new(),
        None,
        TickerMode::Disabled,
    );
    assert!(options.strict_judges.is_some(), "strict の判定が入っている");
}

/// 起動に失敗したとき World は変わらない（受信端は 1 つも据わらない）。
#[test]
fn failed_boot_leaves_world_unchanged() {
    let rig = rig();
    let mut world = World::new();
    let inputs = RuntimeInputs {
        ghost_root: PathBuf::from("runtime_boot_tests/無い/ghost"),
        balloon_root: PathBuf::from("runtime_boot_tests/無い/balloon"),
        shiori: ShioriWiring::Helper {
            helper_exe: PathBuf::from("runtime_boot_tests/使わない/helper.exe"),
        },
        ticker: TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    };

    let result = boot_runtime(&mut world, inputs, &rig.sources, mpsc::channel().0, None);

    // 無いフォルダはマウントで落ちる（失敗の種類まで見る）。
    assert!(matches!(result, Err(GhostBootError::Mount(_))));
    assert_eq!(installed(&world), 0, "失敗では受信端を据えない");
}

/// 起動に成功したときだけ受信端 5 つが World に据わる。
#[test]
fn successful_boot_installs_receivers() {
    let rig = rig();
    let sample = acquire_emo2();
    let (backend, handle) = ScriptedShioriBackend::builder().build();
    let mut world = World::new();
    let inputs = emo2_inputs(&sample, backend);
    assert_eq!(installed(&world), 0);

    let runtime = boot_runtime(&mut world, inputs, &rig.sources, mpsc::channel().0, None)
        .expect("偽の SHIORI と emo2 検体で実行系が起きる");
    assert_eq!(installed(&world), 5, "成功で受信端 5 つが据わる");

    shut_down("runtime_boot ghost shutdown", runtime, handle);
    drop(sample);
}

/// 偽の SHIORI `backend` で emo2 検体（同梱バルーン `emo2-kakukaku`）を起こす入力（時計は止めたまま）。
/// 子の通しのテスト 2 本も使う。
fn emo2_inputs(sample: &SampleRoot, backend: impl ShioriBackend + Send + 'static) -> RuntimeInputs {
    RuntimeInputs {
        ghost_root: sample.folder().to_path_buf(),
        balloon_root: sample
            .balloon("emo2-kakukaku")
            .expect("emo2 の同梱バルーン")
            .to_path_buf(),
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        ticker: TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    }
}

/// 実行系を降ろし、偽の SHIORI の呼び出しの数を進みの目印に終わるまで待つ。子の通しのテスト 2 本も使う。
fn shut_down(what: &str, runtime: GhostRuntime, handle: ScriptedShioriHandle) {
    let calls = move || handle.call_count();
    run_bounded_watching(what, Progress::Count(&calls), move || {
        let _ = runtime.shutdown(CloseReason::User { scope: 0 });
    });
}

/// 注入する Tick 1 回で台詞の時計を進める幅（合成の ms）。
const TICK_STEP_MS: u64 = 100;
/// Tick を注入する実時間の最小間隔（`SwitchRig` と同じ・spin の反復ごとに投函して受信箱を溢れさせない）。
const TICK_EVERY: Duration = Duration::from_millis(1);

/// 足場は `TickerMode::Disabled` なので、台詞の時計は dispatcher へ合成の Tick を注入して進める。
/// Tick は実時間で [`TICK_EVERY`] に 1 つまでに間引き、中断の線の合図を `Leave` が届くまで読んで返す
/// （同じ読みで後ろの合図まで入ることがある）。子の通しのテスト 2 本が使う。
fn signals_until_leave(world: &World, runtime: &GhostRuntime) -> Vec<NoUserBreakSignal> {
    let wiring = world
        .get_non_send::<UserBreakWiring>()
        .expect("成功で中断の持ち物が据わる");
    let mut signals = Vec::new();
    let mut clock_ms = 0;
    let mut last_tick: Option<Instant> = None;
    let reached = wait_until("OnBoot の Leave", Progress::Unknown, || {
        if last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
            last_tick = Some(Instant::now());
            clock_ms += TICK_STEP_MS;
            let _ = runtime.dispatcher().send(DispatcherMsg::Tick {
                now: MonotonicMs(clock_ms),
            });
        }
        signals.extend(wiring.flag_rx().try_iter());
        signals.contains(&NoUserBreakSignal::Leave)
    });
    assert!(
        reached.is_ok(),
        "Leave まで届かない（届いた分 {signals:?}）: {reached:?}"
    );
    signals
}

// 台本の出どころを起動の道で通すテスト（areka-P0-script-security-level・同じ組み方を借りる）。
#[path = "script_security_e2e_tests.rs"]
mod script_security_e2e;

// 台本の `\![open,readme]` を起動の道で偽の OS まで通すテスト（popup-menu-residue 件 3）。
#[path = "readme_script_e2e_tests.rs"]
mod readme_script_e2e;
