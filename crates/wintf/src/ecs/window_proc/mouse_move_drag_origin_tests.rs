//! wndproc のドラッグの式が始めの値として何を読むかの決定論テスト（`dragging_window_origin`）。
//!
//! ドラッグ対象の `DraggingState` があればその値（左上は整数にしたもの）、無ければ控え
//! （スレッドの持ち物の `initial_window_pos`／`start_pos`）を読む。値が同じなら結果も同じ。

use super::dragging_window_origin;
use crate::ecs::Point;
use crate::ecs::drag::DraggingState;
use crate::ecs::pointer::PhysicalPoint;

fn live(start: (i32, i32), inset: (f32, f32)) -> DraggingState {
    DraggingState {
        drag_start_pos: PhysicalPoint::new(start.0, start.1),
        initial_inset: inset,
    }
}

#[test]
fn without_live_state_the_snapshot_is_used() {
    let got = dragging_window_origin(Point { x: 300, y: 400 }, PhysicalPoint::new(10, 20), None);
    assert_eq!(got, (Point { x: 300, y: 400 }, PhysicalPoint::new(10, 20)));
}

#[test]
fn live_state_wins_over_the_snapshot() {
    // 掴んだ点を置き直した後の形（左上も始めのカーソルも控えと違う）。
    let state = live((55, 66), (250.0, 380.0));
    let got = dragging_window_origin(
        Point { x: 300, y: 400 },
        PhysicalPoint::new(10, 20),
        Some(&state),
    );
    assert_eq!(got, (Point { x: 250, y: 380 }, PhysicalPoint::new(55, 66)));
}

#[test]
fn live_inset_is_rounded_to_the_nearest_integer() {
    let state = live((0, 0), (100.6, -20.6));
    let (origin, _) =
        dragging_window_origin(Point { x: 0, y: 0 }, PhysicalPoint::new(0, 0), Some(&state));
    assert_eq!(origin, Point { x: 101, y: -21 });
}

#[test]
fn same_values_give_the_same_result_as_the_snapshot() {
    // dispatch.rs が控えと同じ値で入れたまま誰も置き直していない形（4.3）。
    let snap_origin = Point { x: -1200, y: 340 };
    let snap_start = PhysicalPoint::new(-1100, 500);
    let state = live((-1100, 500), (-1200.0, 340.0));
    assert_eq!(
        dragging_window_origin(snap_origin, snap_start, Some(&state)),
        dragging_window_origin(snap_origin, snap_start, None),
    );
}
