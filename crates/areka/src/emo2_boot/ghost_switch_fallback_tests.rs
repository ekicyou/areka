//! 切替先を起こす・既定へ戻す・致命で終える判断の決定論テスト（areka-P0-ghost-shell-balloon-switch task 8.2）。
//!
//! 確かめること: 既定ゴーストが目録に無い根で既定へ戻すと、終了が指示され最初の出所が
//! 「既定ゴーストへ戻せなかった」になり告知（`event="alert"`）は出ず予約が消える（要件 6.4・6.5）／
//! 起動入力の作り口が無い World で切替先を起こすと `error!(ghost_switch_no_context)` 1 件と
//! `error!(ghost_switch_boot_failed)` の上で既定へ戻す経路に入る（切替先が既定でない）か、
//! 切替先が既定なら既定へ戻さず致命で終える（要件 6.1・6.5）。実ゴーストは起こさない。

use areka_ghost::BasewareRoot;
use areka_kanade::{ChangeHandoff, ShioriFaultKind};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::*;
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};

/// 根に `folders` のゴーストを組む（`ghost/<名>/ghost/master/descript.txt` だけ）。
fn root_with(tmp: &TempPath, folders: &[&str]) -> BasewareRoot {
    for folder in folders {
        let master = tmp.child("ghost").join(folder).join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(
            master.join("descript.txt"),
            format!("charset,UTF-8\nname,{folder}\n"),
        )
        .expect("descript");
    }
    BasewareRoot::new(tmp.path().to_path_buf())
}

/// 今のゴーストを `A` とし、`target` への切替の予約（送り出しの段）と置き場（降ろすものの無い中身）と
/// 終了の受け口を据えた World。起動入力の作り口は据えない。
fn switching_world(root: &BasewareRoot, target: &str) -> World {
    let mut world = World::new();
    world.insert_non_send(AppExit::new());
    world.insert_resource(BootContext {
        root: root.clone(),
        app_profile_dir: root.dir().join("profile"),
        helper_exe: root.dir().join("helper.exe"),
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("A"),
                balloon_root: root.balloon_dir("StayseeBalloon"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Memory,
                dir: root.ghost_dir("A"),
                folder: Some("A".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("StayseeBalloon"),
                folder: Some("StayseeBalloon".to_owned()),
            },
        },
    });
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        None,
        root.ghost_dir("A"),
    ))));
    world.insert_non_send(SwitchInFlight {
        target: SwitchTarget {
            dir: root.ghost_dir(target),
            folder: target.to_owned(),
            name: target.to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: root.ghost_dir("A"),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
    });
    world
}

fn count_event(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// 致命で終えた形: 終了の指示・最初の出所が「既定ゴーストへ戻せなかった」（内部の失敗）・予約なし・
/// 告知 0 件・`ghost_switch_fatal` が `error!` で 1 件。
fn assert_fatal(world: &World, events: &[CapturedEvent]) {
    let origin = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    assert!(
        matches!(
            &origin,
            Some(ExitOrigin::GhostFallbackFailed(f)) if f.kind == ShioriFaultKind::Internal
        ),
        "最初の出所: {origin:?}"
    );
    assert_eq!(
        (
            world.non_send::<AppExit>().is_requested(),
            world.get_non_send::<SwitchInFlight>().is_some(),
            count_event(events, "alert"),
        ),
        (true, false, 0),
        "(終了の指示・予約・告知): {events:?}"
    );
    let fatal: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("ghost_switch_fatal"))
        .collect();
    assert_eq!(fatal.len(), 1, "ghost_switch_fatal は 1 件: {events:?}");
    assert_eq!(fatal[0].level, tracing::Level::ERROR);
}

/// 既定ゴーストが目録に無い根で既定へ戻す → 致命（終了の指示・出所・告知なし）。
#[test]
fn switch_to_default_without_default_ghost_is_fatal() {
    let tmp = TempPath::new("ghost-switch-no-default");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");

    let ((), events) = capture(|| switch_to_default(&mut world, "B"));

    assert_fatal(&world, &events);
}

/// 作り口の無い World で切替先（既定でない）を起こす → `no_context`・`boot_failed` の上で既定へ戻す
/// 経路へ入り、既定も同じ理由で起こせず致命。既定へ戻す試みはちょうど 1 回（窓を閉じるのも
/// 切替先と既定の 2 回）で、致命の理由には落ちたゴーストの名前が載る（要件 6.1・6.2・6.5）。
/// 降ろした所要 ms は置き場に居た 1 体の分だけ残る。
#[test]
fn switch_to_without_inputs_source_falls_back_to_default() {
    let tmp = TempPath::new("ghost-switch-no-source");
    let root = root_with(&tmp, &["A", "B", crate::boot_resolve::DEFAULT_GHOST_FOLDER]);
    let mut world = switching_world(&root, "B");

    let ((), events) = capture(|| switch_to(&mut world, ChangeHandoff { script: None }));

    assert_eq!(
        (
            count_event(&events, "ghost_switch_down_ms"),
            count_event(&events, "ghost_switch_no_context"),
            count_event(&events, "ghost_switch_boot_failed"),
            count_event(&events, "windows_closed_for_restart"),
            count_event(&events, "ghost_switch_booted"),
            world
                .get_non_send::<GhostSlot>()
                .is_some_and(|s| s.0.is_none()),
        ),
        (1, 2, 2, 2, 0, true),
        "(降ろした ms・文脈なし・起動の失敗・窓を閉じた・起動・置き場は空): {events:?}"
    );
    assert!(
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("ghost_switch_no_context"))
            .all(|e| e.level == tracing::Level::ERROR),
        "no_context は error!: {events:?}"
    );
    assert_fatal(&world, &events);
    let reason = match &world.resource::<FirstExit>().0 {
        ExitOrigin::GhostFallbackFailed(f) => f.reason.clone(),
        other => panic!("最初の出所: {other:?}"),
    };
    assert!(
        reason.contains("落ちたゴースト: B"),
        "致命の理由に落ちた名前: {reason}"
    );
}

/// 切替先が既定ゴーストで同期の失敗 → 既定へ戻す経路へ入らず（`no_context` は 1 件のまま）致命。
#[test]
fn switch_to_default_ghost_failure_is_fatal_without_retry() {
    let tmp = TempPath::new("ghost-switch-target-default");
    let root = root_with(&tmp, &["A", crate::boot_resolve::DEFAULT_GHOST_FOLDER]);
    let mut world = switching_world(&root, crate::boot_resolve::DEFAULT_GHOST_FOLDER);

    let ((), events) = capture(|| switch_to(&mut world, ChangeHandoff { script: None }));

    assert_eq!(
        count_event(&events, "ghost_switch_no_context"),
        1,
        "既定へ戻す試みをしない: {events:?}"
    );
    assert_fatal(&world, &events);
}
