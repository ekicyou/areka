//! 窓の組の連れ動きの純関数とドラッグの書き手（task 1.3・要件 2.4／4.2／5.2・設計の決め 12）。

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use wintf::ecs::drag::DragConfig;

use super::sticky_follow::{partner_target, sync_drag_writer, wintf_moves_window};
use super::test_support::single_monitor_snapshot;
use super::{Anchored, CharWindowMarker};
use crate::placement::resolver::{Anchor, PointPx, SizePx};
use crate::placement::sticky_group_ledger::StickyGroupLedger;

/// 相手の窓の大きさ（物理 px）。
const SIZE: SizePx = SizePx { w: 400, h: 600 };
/// 相手の始めの位置（どの辺にも付いていない所）。
const START: PointPx = PointPx { x: 300, y: 200 };
/// ドラッグした窓の量。
const DELTA: PointPx = PointPx { x: 50, y: -30 };

/// 吸着なしの相手は量をそのまま足した所へ動く。
#[test]
fn free_partner_moves_by_whole_delta() {
    let snapshot = single_monitor_snapshot();
    assert_eq!(
        partner_target(START, DELTA, Anchor::Free, SIZE, Some(&snapshot)),
        PointPx { x: 350, y: 170 }
    );
}

/// 下・上・左・右の相手は、吸着した軸に量が乗らず辺に付いたまま、もう一方の軸だけ動く（4.2・5.2）。
#[test]
fn anchored_partner_ignores_delta_on_anchored_axis() {
    let snapshot = single_monitor_snapshot(); // wa = (0,0)-(1920,1043)
    let cases = [
        (
            Anchor::Bottom,
            PointPx {
                x: 350,
                y: 1043 - 600,
            },
        ),
        (Anchor::Top, PointPx { x: 350, y: 0 }),
        (Anchor::Left, PointPx { x: 0, y: 170 }),
        (
            Anchor::Right,
            PointPx {
                x: 1920 - 400,
                y: 170,
            },
        ),
    ];
    for (anchor, want) in cases {
        assert_eq!(
            partner_target(START, DELTA, anchor, SIZE, Some(&snapshot)),
            want,
            "{anchor:?}"
        );
    }
}

/// モニタの写しが無くても、吸着した軸には量が乗らない（始めの位置のまま）。
#[test]
fn anchored_axis_stays_without_snapshot() {
    let cases = [
        (Anchor::Bottom, PointPx { x: 350, y: 200 }),
        (Anchor::Top, PointPx { x: 350, y: 200 }),
        (Anchor::Left, PointPx { x: 300, y: 170 }),
        (Anchor::Right, PointPx { x: 300, y: 170 }),
        (Anchor::Free, PointPx { x: 350, y: 170 }),
    ];
    for (anchor, want) in cases {
        assert_eq!(
            partner_target(START, DELTA, anchor, SIZE, None),
            want,
            "{anchor:?}"
        );
    }
}

/// wintf に窓を動かさせるのは、吸着なしで組に入っていない窓だけ（設計の決め 12）。
#[test]
fn wintf_moves_only_free_ungrouped_window() {
    assert!(wintf_moves_window(Anchor::Free, false));
    assert!(!wintf_moves_window(Anchor::Free, true));
    assert!(!wintf_moves_window(Anchor::Bottom, false));
    assert!(!wintf_moves_window(Anchor::Bottom, true));
}

fn spawn_char(world: &mut World, scope: usize, anchor: Anchor, move_window: bool) -> Entity {
    world
        .spawn((
            CharWindowMarker { scope },
            Anchored(anchor),
            DragConfig {
                move_window,
                ..Default::default()
            },
        ))
        .id()
}

fn move_window(world: &World, e: Entity) -> bool {
    world.get::<DragConfig>(e).unwrap().move_window
}

/// 組を据えると両方の `move_window` が偽になり、組が無ければ向きどおりに戻る（2.4・設計の決め 12）。
#[test]
fn sync_drag_writer_follows_groups_and_anchors() {
    let mut world = World::new();
    let free = spawn_char(&mut world, 0, Anchor::Free, true);
    let bottom = spawn_char(&mut world, 1, Anchor::Bottom, false);
    let other = spawn_char(&mut world, 2, Anchor::Free, true);

    let mut ledger = StickyGroupLedger::default();
    ledger.try_add_script_group(BTreeSet::from([0, 1])).unwrap();
    world.insert_resource(ledger);
    sync_drag_writer(&mut world);
    assert!(!move_window(&world, free));
    assert!(!move_window(&world, bottom));
    // 組に入っていない吸着なしの窓は wintf が動かすまま。
    assert!(move_window(&world, other));

    // 組を解くと向きどおりに戻る。
    world
        .resource_mut::<StickyGroupLedger>()
        .reset_to_descript();
    sync_drag_writer(&mut world);
    assert!(move_window(&world, free));
    assert!(!move_window(&world, bottom));
    assert!(move_window(&world, other));
}

/// 帳簿が無ければ組なしとして向きどおりに付ける。
#[test]
fn sync_drag_writer_without_ledger_uses_anchor_only() {
    let mut world = World::new();
    let free = spawn_char(&mut world, 0, Anchor::Free, false);
    let bottom = spawn_char(&mut world, 1, Anchor::Bottom, true);
    sync_drag_writer(&mut world);
    assert!(move_window(&world, free));
    assert!(!move_window(&world, bottom));
}
