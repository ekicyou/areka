//! `WM_WINDOWPOSCHANGED` の受け手（`window_pos.rs`）の判断の決定論テスト。
//!
//! 道具窓（`DISPATCH_TARGET`・`create_test_window` ほか）は兄弟の
//! `window_pos_transition_tests.rs` のものを借りる（写さない）。

use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SendMessageW, WINDOWPOS,
    WM_WINDOWPOSCHANGED,
};

use super::window_pos_transition_tests::{DISPATCH_TARGET, create_test_window};
use crate::ecs::window::{DPI, SetWindowPosCommand, WindowHandle, drain_window_pos_commands};
use crate::ecs::world::{EcsWorld, engage_tick_flush_guard};

/// 錠（`engage_tick_flush_guard`）を取ったまま積んだ指令は、`WM_WINDOWPOSCHANGED` を
/// 受けても流れない——受け手の流しは錠を取る手順②だけで、錠を取らない流しを持たない（41 番）。
#[test]
fn windowposchanged_does_not_flush_the_queue_while_the_lock_is_held() {
    let _residue = drain_window_pos_commands();
    let (hwnd, instance) = create_test_window();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world
        .borrow_mut()
        .world_mut()
        .spawn((
            WindowHandle { hwnd, instance },
            crate::ecs::window::WindowPos::default(),
            DPI::from_dpi(96, 96),
        ))
        .id();
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = Some((world.clone(), entity)));

    let remaining = {
        let _lock = engage_tick_flush_guard().expect("錠が既に取られている");
        SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
            hwnd,
            30,
            40,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            None,
        ));
        let wp = WINDOWPOS {
            hwnd,
            x: 10,
            y: 10,
            flags: SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ..Default::default()
        };
        // SAFETY: Win32 境界。同じスレッドの自分の窓へ同期で送る（`wp` は呼び出しの間生きている）。
        unsafe {
            SendMessageW(
                hwnd,
                WM_WINDOWPOSCHANGED,
                Some(WPARAM(0)),
                Some(LPARAM(&wp as *const WINDOWPOS as isize)),
            );
        }
        drain_window_pos_commands().len()
    };

    // 配送を止めてから窓を壊す（`DestroyWindow` のメッセージを拾わない）。
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = None);
    // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }

    assert_eq!(
        remaining, 1,
        "錠を取ったままなのに WM_WINDOWPOSCHANGED の受け手がキューを流した"
    );
}
