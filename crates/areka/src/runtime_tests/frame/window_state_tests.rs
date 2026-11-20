// =============================================================================
// 常に手前の状態と取り出しの相の決定論テスト（記録の側）。
// 正本に居ない窓を飛ばす記録・記録の欄・オンのまま降ろした記録を見る。状態の遷移・窓への
// 印・積む順・台本の再生は `areka-runtime` の `frame/window_state_tests.rs` が持つ。
//
// 帯の指令の列は中身を読む口を持たないので、期待の列を同じ口（`push`）で組み、
// `Debug` の写しを比べる（列の中身＝指令の順と entity の列）。
// =============================================================================

use std::path::PathBuf;
use std::sync::mpsc::channel;

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};

use super::super::RuntimeWiring;
use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};

use super::test_support::{
    OFF, ON, marked, queue_in, queue_text, state_with_channel, world_with_windows,
};
use WindowStateDirective::Raise;
// 私有の欄は渡し口から読み書きする（設計 D3 ⑷）。
use areka_runtime::frame::window_state::test_support::{on, on_mut};
use areka_runtime::frame::wiring::test_support::{window_state, window_state_mut};

/// 今のゴーストのフォルダを `pasta` とする起動の文脈。
fn boot_context() -> BootContext {
    let root = BasewareRoot::new(PathBuf::from("skins"));
    BootContext {
        roots: areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
        roots_origin: crate::roots::RootsOrigin::EnvVar,
        app_profile_dir: PathBuf::from("skins/profile"),
        helper_exe: PathBuf::from("skins/helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("pasta"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("pasta"),
                folder: Some("pasta".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    }
}

fn window_state_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.message().starts_with("[window_state]"))
        .collect()
}

/// 正本に載っているが World に居ない窓は飛ばして記録し、残りの窓には印を付けて積む。
#[test]
fn despawned_registry_window_is_skipped_with_a_debug_record() {
    let (mut state, tx) = state_with_channel();
    let (mut world, all) = world_with_windows();
    world.insert_resource(boot_context());
    crate::session::hooks::install(&mut world);
    // 印の除去の掛け金は正本からスコープごと落とすので、古い写しを据え直して
    // 「正本に載るが World に居ない」形を作る（対の片割れだけが先に消えた間と同じ）。
    let stale = world.resource::<GhostWindows>().clone();
    world.despawn(all[1]);
    world.insert_resource(stale);
    let rest = [all[0], all[2], all[3]];
    tx.send(ON).unwrap();

    let ((), events) = capture(|| run_window_state_phase(&mut state, &mut world));

    let skipped: Vec<_> = window_state_events(&events)
        .into_iter()
        .filter(|e| e.field("dropped").is_some())
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].level, tracing::Level::DEBUG);
    assert_eq!(skipped[0].field_str("ghost"), Some("pasta"));
    assert_eq!(skipped[0].field("dropped"), Some("1"));
    assert!(rest.iter().all(|e| marked(&world, *e)));
    assert_eq!(
        queue_in(&world).unwrap(),
        queue_text(&[(BandOp::Enter, &rest)])
    );
}

// ---------------------------------------------------------------------------
// 記録の欄（要件 6.1・6.2・6.7）
// ---------------------------------------------------------------------------

#[test]
fn log_fields_for_change_unchanged_and_raise() {
    let (mut state, tx) = state_with_channel();
    let (mut world, _) = world_with_windows();
    world.insert_resource(boot_context());
    crate::session::hooks::install(&mut world);
    tx.send(ON).unwrap();
    tx.send(ON).unwrap();
    tx.send(Raise).unwrap();
    tx.send(OFF).unwrap();

    let ((), events) = capture(|| run_window_state_phase(&mut state, &mut world));
    let events = window_state_events(&events);
    assert_eq!(events.len(), 4);

    let on = events[0];
    assert_eq!(on.level, tracing::Level::INFO);
    assert_eq!(on.field_str("ghost"), Some("pasta"));
    assert_eq!(on.field_str("directive"), Some("stayontop"));
    assert_eq!(on.field_str("before"), Some("off"));
    assert_eq!(on.field_str("after"), Some("on"));
    assert_eq!(on.field("windows"), Some("4"));

    let same = events[1];
    assert_eq!(same.level, tracing::Level::DEBUG);
    assert_eq!(same.field_str("ghost"), Some("pasta"));
    assert_eq!(same.field_str("directive"), Some("stayontop"));
    assert_eq!(same.field_str("state"), Some("on"));

    let raise = events[2];
    assert_eq!(raise.level, tracing::Level::DEBUG);
    assert_eq!(raise.field_str("ghost"), Some("pasta"));
    assert_eq!(raise.field_str("band"), Some("topmost"));
    assert_eq!(raise.field("windows"), Some("4"));

    let off = events[3];
    assert_eq!(off.level, tracing::Level::INFO);
    assert_eq!(off.field_str("directive"), Some("!stayontop"));
    assert_eq!(off.field_str("before"), Some("on"));
    assert_eq!(off.field_str("after"), Some("off"));
}

// ---------------------------------------------------------------------------
// オンのまま降ろした記録（task 3.2・要件 4.1／4.2／6.4）
// ---------------------------------------------------------------------------

/// 結線（状態 `on`）と起動の文脈を持つ World。
fn world_with_wiring(on: bool) -> World {
    let mut world = World::new();
    world.insert_resource(boot_context());
    crate::session::hooks::install(&mut world);
    let mut wiring = super::super::test_support::headless_wiring_with(
        channel().1,
        super::super::test_support::zero_clock(),
    );
    *on_mut(window_state_mut(&mut wiring)) = on;
    world.insert_non_send(wiring);
    world
}

/// オンなら理由ごとに 1 行（`info!`・ゴースト・理由）を残し、状態は動かさない。
#[test]
fn discard_log_reports_reason_while_on() {
    for (reason, text) in [
        (DiscardReason::Exit, "exit"),
        (DiscardReason::Switch, "switch"),
        (DiscardReason::Reload, "reload"),
    ] {
        let world = world_with_wiring(true);
        let ((), events) = capture(|| log_stay_on_top_discard(&world, reason));
        let events = window_state_events(&events);
        assert_eq!(events.len(), 1, "{reason:?}");
        assert_eq!(events[0].level, tracing::Level::INFO);
        assert_eq!(events[0].field_str("ghost"), Some("pasta"));
        assert_eq!(events[0].field_str("reason"), Some(text));
        assert!(on(window_state(
            world.get_non_send::<RuntimeWiring>().unwrap()
        )));
    }
}

/// オフのとき・結線が無いときは何も書かない。
#[test]
fn discard_log_is_silent_while_off_or_unwired() {
    let off = world_with_wiring(false);
    let ((), events) = capture(|| log_stay_on_top_discard(&off, DiscardReason::Exit));
    assert!(window_state_events(&events).is_empty());

    let unwired = World::new();
    let ((), events) = capture(|| log_stay_on_top_discard(&unwired, DiscardReason::Switch));
    assert!(window_state_events(&events).is_empty());
}
