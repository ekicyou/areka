//! 利用者のずらしを台本の値の上に重ねること（命令の前のドラッグ・向きを変えたとき・記憶から戻したずらし）と記憶への書き込み。

use std::collections::HashMap;

use areka_sylphya::{Axis, PersistKey};
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::Point;
use wintf::ecs::pointer::Phase;

use super::test_support::{BALLOON_SIZE, ScriptWorld, script_world};
use super::*;
use crate::follow::test_support::{drag_end_event_at, window_pos_sized};
use crate::follow::{BalloonFollow, OffsetBase, on_balloon_drag_end};
use crate::persist::finalize_save_tests::Store;
use crate::reseed::test_support::position_of;
use crate::shared_test_support::one_event_with;

const SHIFT: PointPx = PointPx { x: 30, y: 20 };

fn abs(x: i32, y: i32) -> ScriptOffset {
    ScriptOffset {
        x: OffsetAxis::Absolute(x),
        y: OffsetAxis::Absolute(y),
    }
}

fn add(a: PointPx, b: PointPx) -> PointPx {
    PointPx {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn sub(a: PointPx, b: PointPx) -> PointPx {
    PointPx {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn follow(w: &ScriptWorld) -> PointPx {
    let c = w.windows.char_window(0).unwrap();
    w.world.get::<BalloonFollow>(c).unwrap().offset()
}

fn standard_offset(w: &ScriptWorld) -> PointPx {
    let basis = w.world.resource::<DefaultPlacementBasis>();
    resolve_standard_scopes(&w.world, &w.windows, &basis.source, &basis.balloon)
        .into_iter()
        .find(|s| s.placement.scope == 0)
        .unwrap()
        .placement
        .balloon_offset
}

fn user_moved(w: &ScriptWorld) -> bool {
    let c = w.windows.char_window(0).unwrap();
    w.world.entity(c).contains::<BalloonUserMoved>()
}

/// sakura のバルーンを本体から `rel` の所で離す（wintf が窓を動かした後の DragEnd）。
fn release_at(w: &mut ScriptWorld, rel: PointPx) -> Vec<CapturedEvent> {
    let c = w.windows.char_window(0).unwrap();
    let b = w.windows.balloon_window(0).unwrap();
    let char_pos = position_of(&w.world, c);
    w.world.entity_mut(b).insert(window_pos_sized(
        char_pos.x + rel.x,
        char_pos.y + rel.y,
        BALLOON_SIZE.w,
        BALLOON_SIZE.h,
    ));
    let ev = Phase::Bubble(drag_end_event_at(b, (0, 0)));
    let (_, events) = capture(|| on_balloon_drag_end(&mut w.world, b, b, &ev));
    events
}

/// 利用者が今の位置から `SHIFT` だけバルーンを動かして離す。
fn user_shift(w: &mut ScriptWorld) {
    let rel = add(follow(w), SHIFT);
    release_at(w, rel);
    assert!(user_moved(w));
    assert_eq!(follow(w), rel);
}

fn saved_offset(store: &Store) -> Option<PointPx> {
    let saved: HashMap<PersistKey, String> = store.load();
    let axis = |axis| {
        saved
            .get(&PersistKey::BalloonOffset { scope: 0, axis })
            .map(|v| v.parse::<i32>().unwrap())
    };
    Some(PointPx {
        x: axis(Axis::X)?,
        y: axis(Axis::Y)?,
    })
}

#[test]
fn absolute_offset_after_user_shift_keeps_the_shift_on_the_new_standard() {
    let mut w = script_world(&[]);
    user_shift(&mut w);

    assert!(apply_script_balloon_offset(
        &mut w.world,
        0,
        abs(100, 30),
        true
    ));

    let s_new = standard_offset(&w);
    assert_eq!(follow(&w), add(s_new, SHIFT));
    let c = position_of(&w.world, w.windows.char_window(0).unwrap());
    let b = position_of(&w.world, w.windows.balloon_window(0).unwrap());
    assert_eq!(
        b,
        Point {
            x: c.x + s_new.x + SHIFT.x,
            y: c.y + s_new.y + SHIFT.y,
        }
    );
    assert!(user_moved(&w), "利用者が動かした印を付け直す");
}

#[test]
fn changing_align_carries_the_user_shift_to_the_new_side() {
    let mut w = script_world(&[]);
    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Left, true);
    user_shift(&mut w);

    apply_script_balloon_align(&mut w.world, 0, ScriptAlign::Right, true);

    let right = standard_offset(&w);
    assert_eq!(follow(&w), add(right, SHIFT));
    assert!(user_moved(&w));
}

#[test]
fn command_on_restored_offset_adds_the_share_to_the_persisted_value() {
    let mut w = script_world(&[]);
    let c = w.windows.char_window(0).unwrap();
    let b = w.windows.balloon_window(0).unwrap();
    let persisted = PointPx { x: -260, y: 15 };
    w.world.entity_mut(c).insert((
        BalloonFollow::new(b, OffsetBase::unpinned(persisted)),
        BalloonUserMoved,
    ));

    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);

    let ScriptShare::Share(share) = script_share(&w.world, 0) else {
        panic!("分が解ける");
    };
    assert_ne!(share, PointPx { x: 0, y: 0 });
    assert_eq!(follow(&w), add(persisted, share));
    assert!(user_moved(&w));
}

#[test]
fn release_with_script_value_persists_release_relative_minus_share() {
    let mut w = script_world(&[]);
    let store = Store::new();
    store.wire(&mut w.world);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    let ScriptShare::Share(share) = script_share(&w.world, 0) else {
        panic!("分が解ける");
    };
    let rel = add(follow(&w), SHIFT);

    let events = release_at(&mut w, rel);

    assert_eq!(saved_offset(&store), Some(sub(rel, share)));
    assert_eq!(follow(&w), rel, "追従の基準は離した位置そのもの");
    let save = events
        .iter()
        .find(|e| e.target == "areka::persist::save")
        .expect("保存の記録の行");
    assert_eq!(
        save.field("script_share_x"),
        Some(share.x.to_string().as_str())
    );
    assert_eq!(
        save.field("script_share_y"),
        Some(share.y.to_string().as_str())
    );
    store.finish();
}

#[test]
fn release_without_script_value_persists_release_relative_as_before() {
    let mut w = script_world(&[]);
    let store = Store::new();
    store.wire(&mut w.world);
    let rel = add(follow(&w), SHIFT);

    let events = release_at(&mut w, rel);

    assert_eq!(saved_offset(&store), Some(rel));
    let save = events
        .iter()
        .find(|e| e.target == "areka::persist::save")
        .expect("保存の記録の行");
    assert_eq!(save.field("script_share_x"), Some("0"));
    assert_eq!(save.field("script_share_y"), Some("0"));
    store.finish();
}

#[test]
fn release_with_unresolved_share_skips_persisting_and_warns() {
    let mut w = script_world(&[]);
    let store = Store::new();
    store.wire(&mut w.world);
    apply_script_balloon_offset(&mut w.world, 0, abs(100, 30), true);
    w.world.remove_resource::<DefaultPlacementBasis>();
    let rel = add(follow(&w), SHIFT);

    let events = release_at(&mut w, rel);

    assert_eq!(saved_offset(&store), None);
    let skipped = one_event_with(&events, "balloon_persist_skipped");
    assert_eq!(skipped.level, tracing::Level::WARN);
    assert_eq!(skipped.field_str("reason"), Some("no_default_basis"));
    assert_eq!(follow(&w), rel, "記憶へ書かなくても追従の基準は離した位置");
    store.finish();
}
