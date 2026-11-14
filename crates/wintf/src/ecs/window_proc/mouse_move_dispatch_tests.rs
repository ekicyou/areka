//! `WM_MOUSEMOVE`／`WM_MOUSELEAVE` の受け手（`mouse_move.rs`）を配送の経路で確かめる決定論テスト。
//!
//! 道具窓（`DISPATCH_TARGET`・`create_test_window`）は兄弟の
//! `window_pos_transition_tests.rs` のものを借りる（写さない）。

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, SendMessageW, WM_MOUSEMOVE};

use super::super::window_pos::window_pos_transition_tests::{DISPATCH_TARGET, create_test_window};
use crate::ecs::drag::{
    DragStateSnapshot, WindowDragContext, WindowDragContextResource, cancel_dragging,
    snapshot_drag_state, start_dragging, start_preparing, update_dragging,
};
use crate::ecs::pointer::{PhysicalPoint, PointerLeave, PointerState};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{Window, WindowHandle};
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;

/// 同じ窓へ `WM_MOUSEMOVE` を 2 回配ると、離脱の見張り（`TME_LEAVE`）を 2 回掛ける——
/// 見張りの行（成功の `trace` か失敗の `debug`）が 2 本残る（6 番）。
#[test]
fn every_mouse_move_arms_the_leave_watch() {
    let hwnd = create_test_window();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world
        .borrow_mut()
        .world_mut()
        .spawn(WindowHandle { hwnd })
        .id();
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = Some((world.clone(), entity)));

    let log = capture_under_filter("wintf::ecs::window_proc::mouse_move=trace", || {
        for _ in 0..2 {
            // SAFETY: Win32 境界。同じスレッドの自分の窓へ同期で送る（クライアント座標 (5, 5)）。
            unsafe {
                SendMessageW(
                    hwnd,
                    WM_MOUSEMOVE,
                    Some(WPARAM(0)),
                    Some(LPARAM((5 << 16) | 5)),
                );
            }
        }
    });

    // 配送を止めてから窓を壊す（`DestroyWindow` のメッセージを拾わない）。
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = None);
    // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }

    let armed = log
        .lines()
        .filter(|l| l.contains("TrackMouseEvent(TME_LEAVE)"))
        .count();
    assert_eq!(armed, 2, "移動のたびに見張りを掛ける。ログ:\n{log}");
}

/// `WM_MOUSELEAVE` は届いた窓の部品だけを離脱にする（48 番）——窓 A へ配ると A の子は
/// `PointerState` を外して `PointerLeave` が付き、B の子の `PointerState` は残る。続けて B へ
/// 配ると B の子も離脱になる。
#[test]
fn mouse_leave_leaves_only_the_window_it_reached() {
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (window_a, child_a, window_b, child_b) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        let window_a = w.spawn(Window::default()).id();
        let window_b = w.spawn(Window::default()).id();
        let child_a = w.spawn((ChildOf(window_a), PointerState::default())).id();
        let child_b = w.spawn((ChildOf(window_b), PointerState::default())).id();
        (window_a, child_a, window_b, child_b)
    };
    let leave = |target: Entity| {
        let m = WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_MOUSELEAVE,
            wparam: WPARAM(0),
            lparam: LPARAM(0),
        };
        crate::ecs::dispatch_window_message(&world, target, &m);
    };
    // (PointerState がある, PointerLeave がある)
    let state = |e: Entity| {
        let w = world.borrow();
        let w = w.world();
        (
            w.get::<PointerState>(e).is_some(),
            w.get::<PointerLeave>(e).is_some(),
        )
    };

    leave(window_a);
    assert_eq!(state(child_a), (false, true), "A の子は離脱になる");
    assert_eq!(
        state(child_b),
        (true, false),
        "B の子の PointerState は残る"
    );

    leave(window_b);
    assert_eq!(
        state(child_b),
        (false, true),
        "B へ配ると B の子も離脱になる"
    );
}

/// 取っ手が空の窓をドラッグで動かす形（`move_window` が真）で `WM_MOUSEMOVE` を配ると、
/// 窓の移動の書き込みが失敗し、`drag_move_window_failed` の `error` が 1 本残る（44 番）。
#[test]
fn failed_drag_window_move_is_logged_as_error() {
    // DRAG_STATE は thread_local。前のテストの残りを JustEnded へ落としてから始める。
    cancel_dragging();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world.borrow_mut().world_mut().spawn_empty().id();

    // Dragging まで進める。窓の文脈は取っ手が無く（空の取っ手になる）、窓を動かす形。
    start_preparing(entity, PhysicalPoint::new(10, 20), HWND::default());
    start_dragging(PhysicalPoint::new(40, 50));
    let ctx = WindowDragContextResource::new();
    ctx.set(WindowDragContext {
        move_window: true,
        ..Default::default()
    });
    update_dragging(PhysicalPoint::new(40, 50), Some(&ctx));
    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::Dragging {
            move_window: true,
            ..
        }
    ));

    let log = capture_under_filter("wintf::ecs::window_proc::mouse_move=error", || {
        let m = WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_MOUSEMOVE,
            wparam: WPARAM(0),
            lparam: LPARAM((60 << 16) | 50),
        };
        crate::ecs::dispatch_window_message(&world, entity, &m);
    });
    cancel_dragging();

    let failed = log
        .lines()
        .filter(|l| l.contains("ERROR") && l.contains("drag_move_window_failed"))
        .count();
    assert_eq!(
        failed, 1,
        "窓の移動の失敗を error で 1 本残す。ログ:\n{log}"
    );
}
