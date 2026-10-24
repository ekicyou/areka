//! 動かせない指定の帳簿と書き手の決定論の檻（task 5.1・要件 4.1・4.3・4.4・5.1〜5.4・5.8・
//! 6.2・7.3・design「BalloonLock」）。
//!
//! 窓は偽の部品（`BalloonWindowMarker`＋`DragConfig`）だけで組み、記録は
//! `log-capture-kit`（`test_support::capture_logs`）で取る。

use std::collections::BTreeSet;

use wintf::ecs::drag::DragConfig;

use super::super::spawn::BalloonWindowMarker;
use super::super::test_support::{LogEvent, capture_logs};
use super::*;

fn spawn_balloon(world: &mut World, scope: usize) -> Entity {
    world
        .spawn((BalloonWindowMarker { scope }, DragConfig::default()))
        .id()
}

/// バルーンの窓 2 つ（スコープ 0・1）の World。
fn world_with_balloons() -> (World, [Entity; 2]) {
    let mut world = World::new();
    let e = [spawn_balloon(&mut world, 0), spawn_balloon(&mut world, 1)];
    (world, e)
}

fn movable(world: &World, e: Entity) -> bool {
    world.get::<DragConfig>(e).unwrap().move_window
}

fn scopes(raw: &[u32]) -> BTreeSet<u32> {
    raw.iter().copied().collect()
}

fn reason(world: &World, scope: u32) -> Option<HoldReason> {
    world.resource::<BalloonLockLedger>().reason(scope)
}

fn count(events: &[LogEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// descript の段だけ: 書かれたスコープだけ動かせず、ほかは動かせる（4.1・4.3）。
#[test]
fn seed_holds_only_descript_scopes() {
    let (mut world, e) = world_with_balloons();
    seed_balloon_lock(&mut world, &scopes(&[1]));

    assert!(movable(&world, e[0]));
    assert!(!movable(&world, e[1]));
    assert_eq!(
        reason(&world, 1),
        Some(HoldReason {
            descript: true,
            script: false
        })
    );
    assert_eq!(reason(&world, 0), None);
}

/// 帳簿が空なら今と同じく全部動かせる（4.3）。
#[test]
fn empty_seed_keeps_every_balloon_movable() {
    let (mut world, e) = world_with_balloons();
    world.get_mut::<DragConfig>(e[0]).unwrap().move_window = false;
    seed_balloon_lock(&mut world, &BTreeSet::new());

    assert!(movable(&world, e[0]));
    assert!(movable(&world, e[1]));
}

/// `\0` の lock で `\0` だけ止まり `\1` は動かせる・記録 1 件（5.1・5.8）。
#[test]
fn script_lock_applies_only_to_writing_scope() {
    let (mut world, e) = world_with_balloons();
    seed_balloon_lock(&mut world, &BTreeSet::new());

    let ((), events) = capture_logs(|| apply_script_balloon_lock(&mut world, 0, true));

    assert!(!movable(&world, e[0]));
    assert!(movable(&world, e[1]));
    assert_eq!(count(&events, "balloon_move_locked"), 1);
    assert_eq!(
        reason(&world, 0),
        Some(HoldReason {
            descript: false,
            script: true
        })
    );
}

/// unlock は書いた所のスコープの台本の分だけ解き、ほかのスコープの lock は残す（5.2・5.8）。
#[test]
fn script_unlock_releases_only_its_scope() {
    let (mut world, e) = world_with_balloons();
    seed_balloon_lock(&mut world, &BTreeSet::new());
    apply_script_balloon_lock(&mut world, 0, true);
    apply_script_balloon_lock(&mut world, 1, true);

    let ((), events) = capture_logs(|| apply_script_balloon_lock(&mut world, 0, false));

    assert!(movable(&world, e[0]));
    assert!(!movable(&world, e[1]));
    assert_eq!(count(&events, "balloon_move_unlocked"), 1);
    assert_eq!(count(&events, "balloon_move_unlock_held"), 0);
}

/// descript の段の間は unlock で解けず、`balloon_move_unlock_held` が 1 件（5.3・5.8）。
#[test]
fn unlock_cannot_release_descript_tier() {
    let (mut world, e) = world_with_balloons();
    seed_balloon_lock(&mut world, &scopes(&[0]));
    apply_script_balloon_lock(&mut world, 0, true);

    let ((), events) = capture_logs(|| apply_script_balloon_lock(&mut world, 0, false));

    assert!(!movable(&world, e[0]));
    assert_eq!(count(&events, "balloon_move_unlocked"), 1);
    assert_eq!(count(&events, "balloon_move_unlock_held"), 1);
    assert_eq!(
        reason(&world, 0),
        Some(HoldReason {
            descript: true,
            script: false
        })
    );
}

/// 取り替えは descript の段だけを替え、台本の lock を保つ（4.4・5.4）。
#[test]
fn replace_descript_keeps_script_lock() {
    let (mut world, e) = world_with_balloons();
    seed_balloon_lock(&mut world, &scopes(&[0]));
    apply_script_balloon_lock(&mut world, 1, true);

    replace_balloon_lock_descript(&mut world, &BTreeSet::new());

    assert!(movable(&world, e[0]), "descript の段は新しい値（空）になる");
    assert!(!movable(&world, e[1]), "台本の lock は残る");
}

/// 帳簿が無い所への lock は警告 1 件の上で空の帳簿を据えて当てる（design の失敗表）。
#[test]
fn lock_without_ledger_warns_and_seeds_empty() {
    let (mut world, e) = world_with_balloons();

    let ((), events) = capture_logs(|| apply_script_balloon_lock(&mut world, 1, true));

    assert_eq!(count(&events, "balloon_lock_ledger_missing"), 1);
    assert!(movable(&world, e[0]));
    assert!(!movable(&world, e[1]));
}

/// 書き手は `move_window` だけを書き、有効の印と閾値は変えない（6.2）。
#[test]
fn writer_touches_only_move_window() {
    let mut world = World::new();
    let e = world
        .spawn((
            BalloonWindowMarker { scope: 0 },
            DragConfig {
                enabled: false,
                threshold: 17,
                ..Default::default()
            },
        ))
        .id();

    seed_balloon_lock(&mut world, &scopes(&[0]));
    let drag = world.get::<DragConfig>(e).unwrap();
    assert!(!drag.move_window);
    assert!(!drag.enabled);
    assert_eq!(drag.threshold, 17);

    let mut world2 = World::new();
    let e2 = world2
        .spawn((
            BalloonWindowMarker { scope: 0 },
            DragConfig {
                enabled: true,
                threshold: 9,
                move_window: false,
                ..Default::default()
            },
        ))
        .id();
    seed_balloon_lock(&mut world2, &BTreeSet::new());
    let drag = world2.get::<DragConfig>(e2).unwrap();
    assert!(drag.move_window);
    assert!(drag.enabled);
    assert_eq!(drag.threshold, 9);
}
