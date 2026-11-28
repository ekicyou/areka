//! 実行系の側の起動（`boot_runtime`）の決定論テスト。
//!
//! 確かめること: 束から sink が `runtime_sinks` の並びで組まれ、最後から 2 本目が読み直しの
//! 受け口・最後が常に手前の受け口（線へ届く）であること・
//! 起動に失敗したとき World が変わらないこと・成功したときだけ受信端 5 つが World に据わること。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use areka_emo_present::PresentCommand;
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootError, GhostRuntime, ShioriWiring, TickerMode};
use areka_kanade::{BootOrigin, CloseReason, ShioriBackend};
use areka_seriko::{BindResolver, SerikoLoopConfig, SurfaceResolver, spawn_seriko};
use bevy_ecs::world::World;

use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::ghost_switch::ChangeRx;
use crate::ghost_switch_test_support::{TICK_EVERY, TICK_STEP_MS};
use crate::readme::ReadmeWiring;
use crate::shell_balloon_switch::SwitchRx;
use areka_input::user_break::UserBreakWiring;
use areka_runtime::RuntimeInputs;
use areka_runtime::adapter::PresentBridge;
use areka_runtime::consumer_ledger::test_support::carrier_cue;
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
            clock: TalkClock::new(Arc::new(dola::now)),
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

/// 束から組まれる受け口の列は、最後から 2 本目が読み直しの受け口・最後が常に手前の受け口（どちらも
/// 受信端へ届く）である。
#[test]
fn sinks_are_thirteen_with_reload_then_window_state_last() {
    let rig = rig();
    let world = World::new();
    let (mut sinks, lines) = runtime_sinks(&world, &rig.sources);
    assert_eq!(
        sinks.len(),
        13,
        "11 本の並びの後ろに読み直しの受け口、その後ろに常に手前の受け口"
    );

    sinks[11].emit(carrier_cue("reload", &["ghost"]));
    assert_eq!(
        lines.reload_rx.try_iter().count(),
        1,
        "最後から 2 本目の受け口は読み直しの受信端へ送る"
    );

    sinks
        .last_mut()
        .expect("最後の受け口")
        .emit(carrier_cue("set", &["windowstate", "stayontop"]));
    assert_eq!(
        rig.window_state_rx.try_iter().count(),
        1,
        "最後の受け口は常に手前の受信端へ送る"
    );
    assert_eq!(
        lines.reload_rx.try_iter().count(),
        0,
        "常に手前の指令は読み直しの線へ行かない"
    );
}

/// SHIORI だけの読み直しで受け口を組み直しても、常に手前の指令は窓の側の同じ線に届く
/// （送出端は束が持つ）。
#[test]
fn rebuilt_window_state_sink_reaches_the_same_line() {
    let rig = rig();
    let world = World::new();
    let on = || carrier_cue("set", &["windowstate", "stayontop"]);

    let (mut first, _) = runtime_sinks(&world, &rig.sources);
    first.last_mut().expect("最後の受け口").emit(on());
    drop(first);
    let (mut rebuilt, _) = runtime_sinks(&world, &rig.sources);
    rebuilt.last_mut().expect("最後の受け口").emit(on());

    assert_eq!(
        rig.window_state_rx.try_iter().count(),
        2,
        "組み直す前と後の受け口が同じ受信端へ送る"
    );
}

/// 窓の在るゴーストの起動の選択肢（本番と spine が共に使う）には strict の判定が入る
/// （無いと strict の記録が無言で 0 件になる）。
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

    assert!(matches!(result, Err(GhostBootError::Mount(_))));
    assert_eq!(installed(&world), 0, "失敗では受信端を据えない");
}

/// 起動に成功したときだけ受信端 5 つが World に据わる。
#[test]
fn successful_boot_installs_receivers() {
    let rig = rig();
    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
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

/// 起動は渡したバルーンのフォルダの `profile/areka/` をバルーンの層の置き場にし、本体の口の
/// バルーンの差し替えは層を付け替える: 前のバルーンの揮発の値は読めず、覚える値は戻すと読める。
#[test]
fn boot_and_balloon_switch_place_the_balloon_layer() {
    const VOLATILE: &str = "myplugin.state";
    const KEPT: &str = "areka.boot.count";
    let rig = rig();
    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let temp = temp_path_kit::TempPath::under_target("session-balloon-layer");
    let root = areka_ghost::BasewareRoot::new(temp.path().to_path_buf());
    let (first, second) = (root.balloon_dir("first"), root.balloon_dir("second"));
    for dir in [&first, &second] {
        std::fs::create_dir_all(dir).expect("バルーンのフォルダを作る");
    }
    let (backend, handle) = ScriptedShioriBackend::builder().build();
    let mut world = World::new();
    let inputs = RuntimeInputs {
        balloon_root: first.clone(),
        ..emo2_inputs(&sample, backend)
    };
    let runtime = boot_runtime(&mut world, inputs, &rig.sources, mpsc::channel().0, None)
        .expect("偽の SHIORI と emo2 検体で実行系が起きる");
    let kvs = runtime
        .sylphya_reader()
        .kvs(areka_sylphya::AskerId::new("test"));
    let layer = areka_sylphya::PersistScope::Balloon;
    assert_eq!(
        kvs.set(layer, VOLATILE, "v"),
        areka_sylphya::WriteOutcome::Written
    );
    assert_eq!(
        kvs.put(layer, &[(KEPT, "1")]),
        areka_sylphya::WriteOutcome::Written
    );
    let file = |dir: &std::path::Path| dir.join("profile").join("areka").join("sylphya.sqlite");
    assert!(file(&first).is_file());

    world.insert_resource(crate::ghost_switch_test_support::boot_context(
        &root, "emo2",
    ));
    let mut session = GhostSession::for_test(None, sample.folder().to_path_buf());
    session.attach_runtime(runtime);
    world.insert_non_send(GhostSlot(Some(session)));
    crate::hooks::install(&mut world);
    let hooks = *world.resource::<areka_runtime::session_hooks::SessionHooks>();
    let candidate = |dir: &PathBuf| areka_runtime::shell_balloon_resolve::SkinCandidate {
        dir: dir.clone(),
        folder: String::new(),
        name: None,
        hidden: false,
    };

    assert!((hooks.set_current_balloon)(&mut world, &candidate(&second)));
    let after_switch = (kvs.get_in(layer, VOLATILE), kvs.get_in(layer, KEPT));
    assert!((hooks.set_current_balloon)(&mut world, &candidate(&first)));
    let after_return = (kvs.get_in(layer, VOLATILE), kvs.get_in(layer, KEPT));
    drop(kvs);
    let runtime = world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.as_mut().and_then(GhostSession::split_runtime))
        .expect("置き場の実行系");
    shut_down("balloon layer ghost shutdown", runtime, handle);

    assert_eq!(after_switch, (None, None));
    assert_eq!(after_return, (None, Some("1".to_owned())));
    drop(sample);
    drop(temp);
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

/// 中断の線の合図を `Leave` が届くまで読んで返す（同じ読みで後ろの合図まで入ることがある）。
/// 足場は `TickerMode::Disabled` なので、台詞の時計は dispatcher へ合成の Tick を注入して進める
/// （幅と間引きは `SwitchRig` の定数を借りる）。子の通しのテスト 2 本が使う。
fn signals_until_leave(world: &World, runtime: &GhostRuntime) -> Vec<NoUserBreakSignal> {
    let wiring = world
        .get_non_send::<UserBreakWiring>()
        .expect("成功で中断の持ち物が据わる");
    let mut signals = Vec::new();
    let mut clock = dola::TimePoint::ZERO;
    let mut last_tick: Option<Instant> = None;
    let reached = wait_until("OnBoot の Leave", Progress::Unknown, || {
        if last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
            last_tick = Some(Instant::now());
            clock = clock + dola::TimeSpan::from_millis(TICK_STEP_MS as u32);
            let _ = runtime
                .dispatcher()
                .send(DispatcherMsg::Tick { now: clock });
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

// 台本の出どころを起動の道で通すテスト（同じ組み方を借りる）。
#[path = "script_security_e2e_tests.rs"]
mod script_security_e2e;

// 台本の `\![open,readme]` を起動の道で偽の OS まで通すテスト。
#[path = "readme_script_e2e_tests.rs"]
mod readme_script_e2e;
