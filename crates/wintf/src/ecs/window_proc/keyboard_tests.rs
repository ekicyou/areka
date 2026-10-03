//! `WM_ACTIVATE` の非活性化がドラッグを中断する範囲と、閾値前（準備中）の取り消しで
//! 終了の知らせを配らないことの決定論テスト。
//!
//! 知らせの件数は `dispatch_drag_events` を 1 回呼んだ後の `Messages<DragEndEvent>` を
//! `drain` して数える。累積器の返す形（`FlushResult`）には触らない。

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use bevy_ecs::message::Messages;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_ACTIVATE, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KEYDOWN,
};

use crate::ecs::drag::{
    DragAccumulatorResource, DragEndEvent, DragStateSnapshot, DragTransition, cancel_dragging,
    dispatch_drag_events, snapshot_drag_state, start_dragging, start_preparing,
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
    // mouse_move.rs が閾値越えで積む開始の種の代わりに手で積み、配った後の遷移だけを見る。
    accumulator.set_transition(DragTransition::Started {
        entity,
        start_pos: PhysicalPoint::new(10, 20),
        timestamp: Instant::now(),
    });
    accumulator.flush();

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
    let transitions = accumulator
        .flush()
        .map(|flushed| flushed.transitions)
        .unwrap_or_default();
    assert!(
        matches!(
            transitions.as_slice(),
            [DragTransition::Ended {
                cancelled: true,
                ..
            }]
        ),
        "中断の遷移が積まれている: {transitions:?}"
    );
}

/// 準備に入った（閾値前の）窓を 1 つ用意する。
fn setup_preparing() -> (Rc<RefCell<EcsWorld>>, Entity) {
    // DRAG_STATE は thread_local。前のテストの残りを JustEnded へ落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        w.insert_resource(DragAccumulatorResource::new());
        w.spawn_empty().id()
    };
    start_preparing(entity, PhysicalPoint::new(10, 20), HWND::default());
    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::Preparing { .. }
    ));
    (world, entity)
}

/// メッセージを wndproc の経路へ配る。
fn send(world: &Rc<RefCell<EcsWorld>>, entity: Entity, msg: u32, wparam: usize) {
    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg,
        wparam: WPARAM(wparam),
        lparam: LPARAM(0),
    };
    crate::ecs::dispatch_window_message(world, entity, &message);
}

/// 1 回の画面更新として配り、届いた終了の知らせを読み出す。
fn dispatch_once(world: &Rc<RefCell<EcsWorld>>) -> Vec<DragEndEvent> {
    let mut w = world.borrow_mut();
    let w = w.world_mut();
    dispatch_drag_events(w);
    w.resource_mut::<Messages<DragEndEvent>>().drain().collect()
}

/// 準備中の取り消しは状態を取り消しの印つきの休む状態へ落とし、終了の知らせを配らない。
fn assert_cancel_before_threshold_dispatches_no_drag_end(msg: u32, wparam: usize, what: &str) {
    let (world, entity) = setup_preparing();
    send(&world, entity, msg, wparam);

    assert!(
        matches!(
            snapshot_drag_state(),
            DragStateSnapshot::JustEnded {
                cancelled: true,
                ..
            }
        ),
        "閾値前の{what}で状態は JustEnded{{cancelled:true}} になる: {:?}",
        snapshot_drag_state()
    );
    let ends = dispatch_once(&world);
    assert_eq!(
        ends.len(),
        0,
        "閾値前の{what}で終了の知らせは 0 件のはず: {ends:?}"
    );
}

#[test]
fn esc_before_threshold_dispatches_no_drag_end() {
    assert_cancel_before_threshold_dispatches_no_drag_end(WM_KEYDOWN, VK_ESCAPE.0 as usize, "ESC");
}

#[test]
fn cancelmode_before_threshold_dispatches_no_drag_end() {
    assert_cancel_before_threshold_dispatches_no_drag_end(
        WM_CANCELMODE,
        0,
        "メニューやダイアログの割り込み（WM_CANCELMODE）",
    );
}

/// `WM_CAPTURECHANGED` は `mark_released()` を先に呼ぶので `ReleaseCapture` は走らない。
#[test]
fn capturechanged_before_threshold_dispatches_no_drag_end() {
    assert_cancel_before_threshold_dispatches_no_drag_end(
        WM_CAPTURECHANGED,
        0,
        "捕捉の喪失（WM_CAPTURECHANGED）",
    );
}

/// 準備中の非活性化は今も終了の知らせを配らない（修正の前後とも緑）。
#[test]
fn deactivation_before_threshold_dispatches_no_drag_end() {
    let (world, entity) = setup_preparing();
    send(&world, entity, WM_ACTIVATE, 0); // WA_INACTIVE

    let ends = dispatch_once(&world);
    assert_eq!(
        ends.len(),
        0,
        "閾値前の非活性化で終了の知らせは 0 件: {ends:?}"
    );
}
