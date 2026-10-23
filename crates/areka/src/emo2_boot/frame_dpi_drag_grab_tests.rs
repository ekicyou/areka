//! 拡大率の違う画面をまたぐドラッグで、掴んだ点を窓の大きさの比で置き直す檻
//! （`dpi-realign-remembered-chain` 要件 4.1〜4.3・8.5・設計 DragGrab）。
//!
//! # ここが押さえるもの
//!
//! - 実測の形（件 9: 1108×1368 → 831×1026・掴んだ点 (225,806) → (169,605)）で、落とした点
//!   (−1500,1000) から出る左上が (−1669,395) になり、落とした点と掴んだ点の差が縦横 1 px 以内
//!   （要件 4.1・4.2・8.5）。
//! - ドラッグ中でない窓・同じ大きさの書き込み・前の大きさが 0 以下・バルーン窓では置き直さない
//!   （要件 4.3）。
//!
//! 窓の位置の式は wndproc と `drag_follow` の「始めの左上（整数）＋（今のカーソル − 始めの
//! カーソル）」をここで写して計算する。

use wintf::ecs::drag::DraggingState;
use wintf::ecs::{Point, SizeI};

use super::dpi::{GhostWindowKind, reconcile_window_size, rescale_drag_grab};
use super::test_support::{FrameHarness, pos_of};
use crate::placement::diag::PlacementRoute;

/// 実測の形（件 9）の前の大きさ（拡大率 200% の画面）。
const BEFORE: (u32, u32) = (1108, 1368);
/// 実測の形の後の大きさ（拡大率 150% の画面・0.75 倍）。
const AFTER: (u32, u32) = (831, 1026);
/// 窓の中の掴んだ点（前の大きさで）。
const GRAB: (i32, i32) = (225, 806);
/// 落とした点。
const DROP: Point = Point { x: -1500, y: 1000 };

/// 窓の位置の式（wndproc・`drag_follow` と同じ形）: 始めの左上を整数で読み、カーソルの差を足す。
fn top_left_at(drag: &DraggingState, cursor: Point) -> Point {
    Point {
        x: drag.initial_inset.0 as i32 + (cursor.x - drag.drag_start_pos.x),
        y: drag.initial_inset.1 as i32 + (cursor.y - drag.drag_start_pos.y),
    }
}

/// 当該窓へ大きさを書く（拡大率の相と同じ共通末端）。
fn write_size(
    harness: &mut FrameHarness,
    window: bevy_ecs::entity::Entity,
    size: (u32, u32),
) -> bool {
    reconcile_window_size(
        &mut harness.world,
        window,
        GhostWindowKind::Char,
        size,
        PlacementRoute::DpiReproject,
    )
}

/// キャラ窓 0 を前の大きさにし、今の左上から GRAB の点で掴んだドラッグの最中にする。
fn dragging_char0() -> (FrameHarness, bevy_ecs::entity::Entity, DraggingState) {
    let mut harness = FrameHarness::new();
    let window = harness.char_window(0);
    write_size(&mut harness, window, BEFORE);
    let top_left = pos_of(&harness.world, window).expect("前提: キャラ窓に位置がある");
    let drag = DraggingState {
        drag_start_pos: Point::new(top_left.x + GRAB.0, top_left.y + GRAB.1),
        initial_inset: (top_left.x as f32, top_left.y as f32),
    };
    harness.world.entity_mut(window).insert(drag);
    (harness, window, drag)
}

fn drag_of(harness: &FrameHarness, window: bevy_ecs::entity::Entity) -> DraggingState {
    harness
        .world
        .get::<DraggingState>(window)
        .copied()
        .expect("前提: DraggingState がある")
}

/// DraggingState は比べる形を持たないので、2 つの欄を組で比べる。
fn fields(drag: DraggingState) -> (Point, (f32, f32)) {
    (drag.drag_start_pos, drag.initial_inset)
}

/// 純関数に実測の形を入れると、掴んだ点が (169,605) へ縮み、始めの左上が整数の値になる。
#[test]
fn rescale_drag_grab_shrinks_the_measured_grab_point() {
    let start = Point::new(-2000, 300);
    let inset = ((start.x - GRAB.0) as f32, (start.y - GRAB.1) as f32);
    let got = rescale_drag_grab(
        start,
        inset,
        SizeI::new(BEFORE.0 as i32, BEFORE.1 as i32),
        SizeI::new(AFTER.0 as i32, AFTER.1 as i32),
    );
    assert_eq!(got, ((start.x - 169) as f32, (start.y - 605) as f32));
    assert_eq!(got.0.fract(), 0.0, "始めの左上が整数でない");
    assert_eq!(got.1.fract(), 0.0, "始めの左上が整数でない");
}

/// 前の大きさの縦か横が 0 以下なら比が決まらないので置き直さない。
#[test]
fn rescale_drag_grab_keeps_the_inset_when_the_before_size_is_not_positive() {
    let start = Point::new(100, 200);
    let inset = (-125.5, -606.0);
    let after = SizeI::new(831, 1026);
    for before in [
        SizeI::new(0, 1368),
        SizeI::new(1108, 0),
        SizeI::new(-1, 1368),
        SizeI::new(1108, -5),
    ] {
        assert_eq!(
            rescale_drag_grab(start, inset, before, after),
            inset,
            "{before:?}"
        );
    }
}

/// ドラッグ中のキャラ窓へ縮んだ大きさを書くと掴んだ点が置き直り、落とした点 (−1500,1000) で
/// 左上が (−1669,395)・落とした点と掴んだ点の差が縦横 1 px 以内（要件 4.1・4.2・8.5）。
#[test]
fn a_dragging_char_window_rescales_its_grab_point_with_the_measured_shape() {
    let (mut harness, window, before_drag) = dragging_char0();

    // 陽性の対: 置き直す前の値で落とすと、縮んだ窓の掴んだ点は落とした点から大きく外れる。
    let unscaled = top_left_at(&before_drag, DROP);
    assert_eq!(unscaled, Point::new(DROP.x - GRAB.0, DROP.y - GRAB.1));

    assert!(
        write_size(&mut harness, window, AFTER),
        "前提: 大きさの書き込みが起きる"
    );
    let drag = drag_of(&harness, window);
    assert_eq!(
        drag.drag_start_pos, before_drag.drag_start_pos,
        "始めのカーソルが変わった"
    );
    let grab_after = (
        drag.drag_start_pos.x - drag.initial_inset.0 as i32,
        drag.drag_start_pos.y - drag.initial_inset.1 as i32,
    );
    assert_eq!(grab_after, (169, 605), "掴んだ点が比で置き直っていない");

    let top_left = top_left_at(&drag, DROP);
    assert_eq!(top_left, Point::new(-1669, 395), "落とした点から出る左上");

    // 掴んだ点（窓の中の位置を大きさの比で縮めた正確な値）が落とした点から 1 px 以内。
    let exact = (
        f64::from(GRAB.0) * f64::from(AFTER.0) / f64::from(BEFORE.0),
        f64::from(GRAB.1) * f64::from(AFTER.1) / f64::from(BEFORE.1),
    );
    let dx = (f64::from(top_left.x) + exact.0 - f64::from(DROP.x)).abs();
    let dy = (f64::from(top_left.y) + exact.1 - f64::from(DROP.y)).abs();
    assert!(dx <= 1.0 && dy <= 1.0, "差が 1 px を超えた: ({dx}, {dy})");
}

/// 同じ大きさの書き込みでは置き直さない（要件 4.3）。
#[test]
fn a_same_size_write_keeps_the_grab_point() {
    let (mut harness, window, before_drag) = dragging_char0();
    write_size(&mut harness, window, BEFORE);
    assert_eq!(fields(drag_of(&harness, window)), fields(before_drag));
}

/// ドラッグ中でない窓の大きさを書いても、ドラッグ中の窓の掴んだ点は変わらない（要件 4.3）。
#[test]
fn resizing_a_window_that_is_not_dragged_keeps_the_grab_point() {
    let (mut harness, window, before_drag) = dragging_char0();
    let other = harness.char_window(1);
    assert!(
        write_size(&mut harness, other, AFTER),
        "前提: 大きさの書き込みが起きる"
    );
    assert!(
        harness.world.get::<DraggingState>(other).is_none(),
        "ドラッグ中でない窓に印が付いた"
    );
    assert_eq!(fields(drag_of(&harness, window)), fields(before_drag));
}

/// バルーン窓の大きさの書き込みでは置き直さない。
#[test]
fn a_balloon_window_size_write_keeps_the_grab_point() {
    let mut harness = FrameHarness::new();
    let balloon = harness.balloon_window(0);
    let top_left = pos_of(&harness.world, balloon).expect("前提: バルーン窓に位置がある");
    let drag = DraggingState {
        drag_start_pos: Point::new(top_left.x + 40, top_left.y + 30),
        initial_inset: (top_left.x as f32, top_left.y as f32),
    };
    harness.world.entity_mut(balloon).insert(drag);
    assert!(
        reconcile_window_size(
            &mut harness.world,
            balloon,
            GhostWindowKind::Balloon,
            (446, 316),
            PlacementRoute::DpiReproject,
        ),
        "前提: 大きさの書き込みが起きる"
    );
    assert_eq!(
        harness
            .world
            .get::<DraggingState>(balloon)
            .copied()
            .map(fields),
        Some(fields(drag))
    );
}
