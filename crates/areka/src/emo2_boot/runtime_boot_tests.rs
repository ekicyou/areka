//! 実行系の側の起動（`boot_runtime`）の決定論テスト（areka-P0-mcp-reload 要件 4.2・13.4）。
//!
//! 確かめること: 束から 12 本の sink が今の並びで組まれ、12 本目が読み直しの受け口であること・
//! 起動に失敗したとき World が変わらないこと・成功したときだけ受信端 5 つが World に据わること。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use areka_emo_present::PresentCommand;
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::{ShioriWiring, TickerMode};
use areka_kanade::{BootOrigin, CloseReason, ShioriBackend};
use areka_seriko::{BindResolver, SerikoLoopConfig, SurfaceResolver, spawn_seriko};
use bevy_ecs::world::World;
use dola::cue::{ActorKey, CueCommand, TalkCue};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::*;
use crate::emo2_boot::Emo2BootInputs;
use crate::emo2_boot::adapter::PresentBridge;
use crate::emo2_boot::ghost_switch::ChangeRx;
use crate::emo2_boot::reload_cue::ReloadRx;
use crate::emo2_boot::sample_test_support::acquire_emo2;
use crate::emo2_boot::shell_balloon_switch::SwitchRx;
use crate::emo2_boot::spine::{Progress, ScriptedShioriBackend, run_bounded_watching};
use crate::emo2_boot::talk_clock::TalkClock;
use crate::input_events::user_break::UserBreakWiring;
use crate::readme::ReadmeWiring;

/// 窓の側の束の持ち物（seriko と文字の線を生かしておく）。
struct Rig {
    sources: CueSinkSources,
    _present_rx: Receiver<PresentCommand>,
    _move_rx: Receiver<crate::emo2_boot::move_cue::PositionCue>,
    _lifecycle_rx: Receiver<crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal>,
    _zorder_rx: Receiver<crate::emo2_boot::zorder_cue::ZOrderDirective>,
    _text: Box<dyn std::any::Any>,
    _runtime: Rc<RefCell<TextLayerRuntime>>,
}

/// 本物の seriko と文字の線から束を組む（窓は要らない）。
fn rig() -> Rig {
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let (text, text_pump) =
        spawn_emo_text(Rc::clone(&runtime)).expect("このスレッドで文字の線を起こせる");
    let (present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    let (surface, _seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        areka_emo_compose::BindSet::default(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        PresentBridge::new(present_tx),
    );
    let (move_tx, move_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
    let (zorder_tx, zorder_rx) = mpsc::channel();
    Rig {
        sources: CueSinkSources {
            surface,
            text,
            clock: TalkClock::new(Arc::new(dola::runtime::clock::now)),
            move_tx,
            lifecycle_tx,
            zorder_tx,
        },
        _present_rx: present_rx,
        _move_rx: move_rx,
        _lifecycle_rx: lifecycle_rx,
        _zorder_rx: zorder_rx,
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

fn com() {
    // SAFETY: 起動の資産の読み込みに要る COM 初期化（既初期化の S_FALSE 等は無視・assets.rs 同流儀）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// 束から 12 本が組まれ、12 本目が読み直しの受け口（受信端へ届く）である。
#[test]
fn sinks_are_twelve_with_reload_last() {
    let rig = rig();
    let world = World::new();
    let (mut sinks, lines) = runtime_sinks(&world, &rig.sources);
    assert_eq!(sinks.len(), 12, "11 本の今の並びの後ろに読み直しの受け口");

    let cue = TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier("reload", vec!["ghost".to_string()]),
        duration: 0.0,
    };
    sinks.last_mut().expect("12 本目").emit(cue);
    assert_eq!(
        lines.reload_rx.try_iter().count(),
        1,
        "12 本目の受け口は読み直しの受信端へ送る"
    );
}

/// 起動に失敗したとき World は変わらない（受信端は 1 つも据わらない）。
#[test]
fn failed_boot_leaves_world_unchanged() {
    let rig = rig();
    let mut world = World::new();
    let inputs = Emo2BootInputs {
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

    assert!(matches!(result, Err(RuntimeBootError::Boot(_))));
    assert_eq!(installed(&world), 0, "失敗では受信端を据えない");
}

/// 起動に成功したときだけ受信端 5 つが World に据わる。
#[test]
fn successful_boot_installs_receivers() {
    com();
    let rig = rig();
    let sample = acquire_emo2();
    let (backend, handle) = ScriptedShioriBackend::builder().build();
    let mut world = World::new();
    let inputs = Emo2BootInputs {
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
    };
    assert_eq!(installed(&world), 0);

    let runtime = boot_runtime(&mut world, inputs, &rig.sources, mpsc::channel().0, None)
        .expect("偽の SHIORI と emo2 検体で実行系が起きる");
    assert_eq!(installed(&world), 5, "成功で受信端 5 つが据わる");

    let calls = move || handle.call_count();
    run_bounded_watching(
        "runtime_boot ghost shutdown",
        Progress::Count(&calls),
        move || {
            let _ = runtime.shutdown(CloseReason::User { scope: 0 });
        },
    );
    drop(sample);
}

// 台本の出どころを起動の道で通すテスト（areka-P0-script-security-level・同じ組み方を借りる）。
#[path = "script_security_e2e_tests.rs"]
mod script_security_e2e;
