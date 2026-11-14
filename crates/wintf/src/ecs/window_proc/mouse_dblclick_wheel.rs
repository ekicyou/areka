//! ダブルクリック・ホイールメッセージハンドラ
//!
//! WM_*BUTTONDBLCLK, WM_MOUSEWHEEL, WM_MOUSEHWHEEL の処理を担当する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

// 4 つの `WM_*BUTTONDBLCLK` は、ふつうの押下の受け手（`mouse_click::handle_button_message`）を
// ダブルクリックの印つきで呼ぶだけ。当たり判定・`PointerState` の付与・押下の記録・ドラッグ準備・
// 当たらないときの窓の entity への記録は、ふつうの押下と同じ道を通る（2 回目の押下のまま動かせばドラッグになる）。
use super::mouse_click::handle_button_message;

/// WM_LBUTTONDBLCLK: 左ボタンダブルクリック
#[inline]
pub(super) fn WM_LBUTTONDBLCLK(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Left,
        true,
        true,
    )
}

/// WM_RBUTTONDBLCLK: 右ボタンダブルクリック
#[inline]
pub(super) fn WM_RBUTTONDBLCLK(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Right,
        true,
        true,
    )
}

/// WM_MBUTTONDBLCLK: 中ボタンダブルクリック
#[inline]
pub(super) fn WM_MBUTTONDBLCLK(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Middle,
        true,
        true,
    )
}

/// WM_XBUTTONDBLCLK: 拡張ボタンダブルクリック
#[inline]
pub(super) fn WM_XBUTTONDBLCLK(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    let xbutton = ((wparam.0 >> 16) & 0xFFFF) as u16;
    let button = if xbutton == 1 {
        crate::ecs::pointer::PointerButton::XButton1
    } else {
        crate::ecs::pointer::PointerButton::XButton2
    };
    handle_button_message(world, entity, hwnd, wparam, lparam, button, true, true)
}

/// WM_MOUSEWHEEL: 垂直ホイール回転
#[inline]
pub(super) fn WM_MOUSEWHEEL(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // GET_WHEEL_DELTA_WPARAM: HIWORD of wParam (signed)
    let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
    crate::ecs::pointer::add_wheel_vertical(entity, delta);
    crate::ecs::cursor::answer::push_cursor_rc(
        world,
        entity,
        crate::ecs::cursor::CursorTrigger::Wheel,
    );

    Some(LRESULT(0))
}

/// WM_MOUSEHWHEEL: 水平ホイール回転
#[inline]
pub(super) fn WM_MOUSEHWHEEL(
    _world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
    crate::ecs::pointer::add_wheel_horizontal(entity, delta);

    Some(LRESULT(0))
}

#[cfg(test)]
#[path = "mouse_dblclick_wheel_tests.rs"]
mod tests;
