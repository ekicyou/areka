//! `WM_MOUSEMOVE`／`WM_MOUSELEAVE` の受け手（`mouse_move.rs`）を配送の経路で確かめる決定論テスト。
//!
//! 道具窓（`DISPATCH_TARGET`・`create_test_window`）は兄弟の
//! `window_pos_transition_tests.rs` のものを借りる（写さない）。

use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, SendMessageW, WM_MOUSEMOVE};

use super::super::window_pos::window_pos_transition_tests::{DISPATCH_TARGET, create_test_window};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::WindowHandle;
use crate::ecs::world::EcsWorld;

/// 同じ窓へ `WM_MOUSEMOVE` を 2 回配ると、離脱の見張り（`TME_LEAVE`）を 2 回掛ける——
/// 見張りの行（成功の `trace` か失敗の `debug`）が 2 本残る（6 番）。
#[test]
fn every_mouse_move_arms_the_leave_watch() {
    let (hwnd, instance) = create_test_window();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world
        .borrow_mut()
        .world_mut()
        .spawn(WindowHandle { hwnd, instance })
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
