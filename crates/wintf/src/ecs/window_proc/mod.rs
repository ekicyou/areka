//! ウィンドウプロシージャモジュール
//!
//! Windowsメッセージのディスパッチとハンドラ管理

mod keyboard;
mod lifecycle;
mod mouse_button;
mod mouse_move;
mod window_pos;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::*;

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::OnceLock;

// SAFETY: EcsWorldはメインスレッドでのみアクセスされる
// wndprocもメインスレッドから呼ばれるため安全
struct SendWeak(Weak<RefCell<crate::ecs::world::EcsWorld>>);
unsafe impl Send for SendWeak {}
unsafe impl Sync for SendWeak {}

static ECS_WORLD: OnceLock<SendWeak> = OnceLock::new();

/// EcsWorldへの弱参照を登録（WinThreadMgr初期化時に呼ばれる）
#[inline]
pub(crate) fn set_ecs_world(world: Weak<RefCell<crate::ecs::world::EcsWorld>>) {
    let _ = ECS_WORLD.set(SendWeak(world));
}

/// EcsWorldへの参照を取得（try_borrow_mut可能な状態で）
pub(super) fn try_get_ecs_world() -> Option<Rc<RefCell<crate::ecs::world::EcsWorld>>> {
    ECS_WORLD.get().and_then(|weak| weak.0.upgrade())
}

/// ECS専用のウィンドウプロシージャ
pub(crate) extern "system" fn ecs_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let result = match message {
        WM_NCCREATE => lifecycle::WM_NCCREATE(hwnd, message, wparam, lparam),
        WM_NCDESTROY => lifecycle::WM_NCDESTROY(hwnd, message, wparam, lparam),
        WM_ERASEBKGND => lifecycle::WM_ERASEBKGND(hwnd, message, wparam, lparam),
        WM_PAINT => lifecycle::WM_PAINT(hwnd, message, wparam, lparam),
        WM_CLOSE => lifecycle::WM_CLOSE(hwnd, message, wparam, lparam),
        WM_WINDOWPOSCHANGED => window_pos::WM_WINDOWPOSCHANGED(hwnd, message, wparam, lparam),
        WM_DISPLAYCHANGE => lifecycle::WM_DISPLAYCHANGE(hwnd, message, wparam, lparam),
        WM_DPICHANGED => window_pos::WM_DPICHANGED(hwnd, message, wparam, lparam),
        // マウスメッセージ
        WM_NCHITTEST => mouse_move::WM_NCHITTEST(hwnd, message, wparam, lparam),
        WM_MOUSEMOVE => mouse_move::WM_MOUSEMOVE(hwnd, message, wparam, lparam),
        WM_MOUSELEAVE => mouse_move::WM_MOUSELEAVE(hwnd, message, wparam, lparam),
        WM_LBUTTONDOWN => mouse_button::WM_LBUTTONDOWN(hwnd, message, wparam, lparam),
        WM_LBUTTONUP => mouse_button::WM_LBUTTONUP(hwnd, message, wparam, lparam),
        WM_RBUTTONDOWN => mouse_button::WM_RBUTTONDOWN(hwnd, message, wparam, lparam),
        WM_RBUTTONUP => mouse_button::WM_RBUTTONUP(hwnd, message, wparam, lparam),
        WM_MBUTTONDOWN => mouse_button::WM_MBUTTONDOWN(hwnd, message, wparam, lparam),
        WM_MBUTTONUP => mouse_button::WM_MBUTTONUP(hwnd, message, wparam, lparam),
        WM_XBUTTONDOWN => mouse_button::WM_XBUTTONDOWN(hwnd, message, wparam, lparam),
        WM_XBUTTONUP => mouse_button::WM_XBUTTONUP(hwnd, message, wparam, lparam),
        WM_LBUTTONDBLCLK => mouse_button::WM_LBUTTONDBLCLK(hwnd, message, wparam, lparam),
        WM_RBUTTONDBLCLK => mouse_button::WM_RBUTTONDBLCLK(hwnd, message, wparam, lparam),
        WM_MBUTTONDBLCLK => mouse_button::WM_MBUTTONDBLCLK(hwnd, message, wparam, lparam),
        WM_XBUTTONDBLCLK => mouse_button::WM_XBUTTONDBLCLK(hwnd, message, wparam, lparam),
        WM_MOUSEWHEEL => mouse_button::WM_MOUSEWHEEL(hwnd, message, wparam, lparam),
        WM_MOUSEHWHEEL => mouse_button::WM_MOUSEHWHEEL(hwnd, message, wparam, lparam),
        WM_KEYDOWN => keyboard::WM_KEYDOWN(hwnd, message, wparam, lparam),
        WM_CANCELMODE => keyboard::WM_CANCELMODE(hwnd, message, wparam, lparam),
        WM_ACTIVATE => keyboard::WM_ACTIVATE(hwnd, message, wparam, lparam),
        _ => None,
    };

    result.unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, message, wparam, lparam) })
}

/// hwndからEntity IDを取得するヘルパー関数
#[inline]
pub(crate) fn get_entity_from_hwnd(hwnd: HWND) -> Option<Entity> {
    unsafe {
        let entity_bits = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        Entity::try_from_bits(entity_bits as u64)
    }
}
