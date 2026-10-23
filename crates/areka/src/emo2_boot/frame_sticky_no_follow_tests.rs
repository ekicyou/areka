//! 組の相手を連れ動かさない道の檻（areka-P0-window-link-directives task 4・要件 4.7・
//! 設計の決め 6「連れ動きはドラッグと `\![move]` だけ」）。
//!
//! 組 {0,1} を 2.1 の関数で据え、スコープ 0（下吸着）だけを吸着の当て直し・表情の切り替えに
//! よる大きさの変化・作業領域の変化で動かす。相手のスコープ 1 は吸着なしにして、自分の道では
//! 動かないようにしておく——動けば連れ動きが紛れ込んだことになる。どの檻にも「スコープ 0 は
//! 実際に動いた」という陽性の対を置く（相手が動かないのが無操作のおかげで通らないように）。
//! 並べ直し（`resetwindowpos`）の檻は `placement/reset_pos_tests.rs` に置く。

use crate::placement::follow::{Anchored, MonitorSnapshot};
use crate::placement::resolver::{Anchor, RectPx, SizePx};
use crate::placement::script_alignment::{AlignmentWord, apply_script_alignment};
use crate::placement::spawn::GhostWindows;
use crate::placement::sticky_group_ledger::{apply_set_sticky, seed_sticky_descript};
use wintf::ecs::window::drain_window_pos_commands;

use super::test_support::{pos_of, resnap_world};
use super::work_area_sync::{SnapshotChange, resnap_for_work_area_change};
use super::*;

/// 組 {0,1} の World（スコープ 1 は吸着なし）。
fn grouped_world() -> (World, GhostWindows) {
    let (mut world, gw) = resnap_world();
    let partner = gw.char_window(1).unwrap();
    world.entity_mut(partner).insert(Anchored(Anchor::Free));
    seed_sticky_descript(&mut world, None);
    assert!(apply_set_sticky(
        &mut world,
        &["0".to_string(), "1".to_string()]
    ));
    (world, gw)
}

/// 相手が動かず、スコープ 0 は動いたことを確かめて片付ける。
fn assert_only_scope0_moved(
    world: World,
    gw: &GhostWindows,
    before: [Option<wintf::ecs::Point>; 2],
) {
    let (c0, c1) = (gw.char_window(0).unwrap(), gw.char_window(1).unwrap());
    assert_ne!(pos_of(&world, c0), before[0], "前提: スコープ 0 が動いた");
    assert_eq!(pos_of(&world, c1), before[1], "組の相手は動かない");
    drop(world);
    let _residue = drain_window_pos_commands();
}

fn positions(world: &World, gw: &GhostWindows) -> [Option<wintf::ecs::Point>; 2] {
    [0, 1].map(|s| pos_of(world, gw.char_window(s).unwrap()))
}

/// 吸着の当て直し（台本の吸着の指令）では相手を動かさない。
#[test]
fn reanchoring_does_not_move_the_partner() {
    let (mut world, gw) = grouped_world();
    let before = positions(&world, &gw);
    assert!(apply_script_alignment(
        &mut world,
        0,
        &AlignmentWord::Anchor(Anchor::Top)
    ));
    assert_only_scope0_moved(world, &gw, before);
}

/// 表情の切り替えによる大きさの変化（再スナップ）では相手を動かさない。
#[test]
fn surface_size_change_does_not_move_the_partner() {
    let (mut world, gw) = grouped_world();
    let before = positions(&world, &gw);
    resnap_from_sizes(
        &mut world,
        [(0usize, SizePx { w: 434, h: 700 })].into_iter(),
    );
    assert_only_scope0_moved(world, &gw, before);
}

/// 作業領域の変化（寄せ直し）では相手を動かさない。
#[test]
fn work_area_change_does_not_move_the_partner() {
    let (mut world, gw) = grouped_world();
    let before = positions(&world, &gw);
    let previous = world.resource::<MonitorSnapshot>().clone();
    let raised = RectPx {
        bottom: previous.work_areas[0].bottom - 100,
        ..previous.work_areas[0]
    };
    let current = MonitorSnapshot {
        work_areas: vec![raised],
    };
    world.insert_resource(current.clone());
    resnap_for_work_area_change(&mut world, &SnapshotChange { previous, current });
    assert_only_scope0_moved(world, &gw, before);
}
