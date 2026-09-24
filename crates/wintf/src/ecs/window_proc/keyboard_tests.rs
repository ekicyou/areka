//! `WM_ACTIVATE` の非活性化がドラッグを中断する範囲の決定論テスト。

use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::WM_ACTIVATE;

use crate::ecs::drag::{
    DragAccumulatorResource, DragStateSnapshot, DragTransition, cancel_dragging,
    snapshot_drag_state, start_dragging, start_preparing,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;

/// 閾値到達の直後（`JustStarted`＝次の tick の配送前）に窓が非アクティブになっても、
/// `Dragging` と同じく `Ended { cancelled: true }` を積んでからドラッグを中断する。
#[test]
fn deactivation_right_after_the_threshold_cancels_the_drag() {
    // DRAG_STATE は thread_local。前のテストの残りを JustEnded へ落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (entity, accumulator) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        let accumulator = DragAccumulatorResource::new();
        w.insert_resource(accumulator.clone());
        (w.spawn_empty().id(), accumulator)
    };

    start_preparing(entity, PhysicalPoint::new(10, 20), HWND::default());
    start_dragging(PhysicalPoint::new(40, 50));
    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustStarted { .. }
    ));
    accumulator.flush(); // start_dragging が積んだ Started を捨て、以降の遷移だけを見る

    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg: WM_ACTIVATE,
        wparam: WPARAM(0), // WA_INACTIVE
        lparam: LPARAM(0),
    };
    crate::ecs::dispatch_window_message(&world, entity, &message);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: true,
                ..
            }
        ),
        "JustStarted のまま非活性化してもドラッグは中断される"
    );
    let transition = accumulator.flush().and_then(|flushed| flushed.transition);
    assert!(
        matches!(
            transition,
            Some(DragTransition::Ended {
                cancelled: true,
                ..
            })
        ),
        "中断の遷移が積まれている: {transition:?}"
    );
}
