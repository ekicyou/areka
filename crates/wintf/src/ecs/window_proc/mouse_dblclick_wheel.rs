//! ダブルクリック・ホイールメッセージハンドラ
//!
//! WM_*BUTTONDBLCLK, WM_MOUSEWHEEL, WM_MOUSEHWHEEL の処理を担当する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::debug;
use windows::Win32::Foundation::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// ダブルクリックメッセージハンドラ共通処理
#[inline]
/// WM_*BUTTONDBLCLK ハンドラ共通処理
///
/// ダブルクリックイベントを処理し、hit_testでターゲットエンティティを特定して
/// PointerStateを付与する。WM_LBUTTONDOWNの代わりにWM_LBUTTONDBLCLKが来るため、
/// ボタン押下記録も同時に行う。
///
/// # Arguments
/// - `hwnd`: ウィンドウハンドル
/// - `wparam`: 修飾キー状態とXBUTTON情報
/// - `lparam`: クリック座標（クライアント座標）
/// - `double_click`: ダブルクリック種別
fn handle_double_click_message(
    world: &Rc<RefCell<EcsWorld>>,
    window_entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
    double_click: crate::ecs::pointer::DoubleClick,
) -> HandlerResult {
    use crate::ecs::layout::hit_test::{PhysicalPoint as HitTestPoint, hit_test_in_window};
    use crate::ecs::pointer::{PhysicalPoint, PointerState};

    // クリック位置を取得
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

    // 修飾キー状態を抽出
    let wparam_val = wparam.0 as u32;
    let shift = (wparam_val & 0x04) != 0;
    let ctrl = (wparam_val & 0x08) != 0;

    // ダブルクリックに対応するボタンを取得
    let button = match double_click {
        crate::ecs::pointer::DoubleClick::Left => crate::ecs::pointer::PointerButton::Left,
        crate::ecs::pointer::DoubleClick::Right => crate::ecs::pointer::PointerButton::Right,
        crate::ecs::pointer::DoubleClick::Middle => crate::ecs::pointer::PointerButton::Middle,
        crate::ecs::pointer::DoubleClick::XButton1 => crate::ecs::pointer::PointerButton::XButton1,
        crate::ecs::pointer::DoubleClick::XButton2 => crate::ecs::pointer::PointerButton::XButton2,
        crate::ecs::pointer::DoubleClick::None => return Some(LRESULT(0)),
    };

    // hit_test でターゲットエンティティを特定し、PointerState を確保
    {
        if let Ok(mut world_borrow) = world.try_borrow_mut() {
            if let Some(target_entity) = hit_test_in_window(
                world_borrow.world(),
                window_entity,
                HitTestPoint::new(x as f32, y as f32),
            ) {
                tracing::info!(
                    window_entity = ?window_entity,
                    target_entity = ?target_entity,
                    double_click = ?double_click,
                    x, y,
                    "[handle_double_click_message] Double-click detected"
                );

                // PointerState がない場合は付与
                if world_borrow
                    .world()
                    .get::<PointerState>(target_entity)
                    .is_none()
                {
                    world_borrow
                        .world_mut()
                        .entity_mut(target_entity)
                        .insert(PointerState {
                            client_point: PhysicalPoint::new(x, y),
                            local_point: PhysicalPoint::new(x, y),
                            left_down: button == crate::ecs::pointer::PointerButton::Left,
                            right_down: button == crate::ecs::pointer::PointerButton::Right,
                            middle_down: button == crate::ecs::pointer::PointerButton::Middle,
                            xbutton1_down: button == crate::ecs::pointer::PointerButton::XButton1,
                            xbutton2_down: button == crate::ecs::pointer::PointerButton::XButton2,
                            shift_down: shift,
                            ctrl_down: ctrl,
                            double_click,
                            ..Default::default()
                        });
                    debug!(
                        entity = ?target_entity,
                        button = ?button,
                        double_click = ?double_click,
                        "PointerState inserted on double-click event"
                    );
                } else {
                    // 既存の PointerState に double_click を設定
                    if let Some(mut ps) = world_borrow
                        .world_mut()
                        .get_mut::<PointerState>(target_entity)
                    {
                        ps.double_click = double_click;
                        ps.shift_down = shift;
                        ps.ctrl_down = ctrl;
                    }
                }

                // 修飾キー状態を記録
                crate::ecs::pointer::set_modifier_state(target_entity, shift, ctrl);

                // ボタン状態をバッファに記録
                crate::ecs::pointer::record_button_down(target_entity, button);
            }
        }
    }

    Some(LRESULT(0))
}

/// WM_LBUTTONDBLCLK: 左ボタンダブルクリック
#[inline]
pub(super) fn WM_LBUTTONDBLCLK(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_double_click_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::DoubleClick::Left,
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
    handle_double_click_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::DoubleClick::Right,
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
    handle_double_click_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::DoubleClick::Middle,
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
    let double_click = if xbutton == 1 {
        crate::ecs::pointer::DoubleClick::XButton1
    } else {
        crate::ecs::pointer::DoubleClick::XButton2
    };
    handle_double_click_message(world, entity, hwnd, wparam, lparam, double_click)
}

/// WM_MOUSEWHEEL: 垂直ホイール回転
#[inline]
pub(super) fn WM_MOUSEWHEEL(
    _world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // GET_WHEEL_DELTA_WPARAM: HIWORD of wParam (signed)
    let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
    crate::ecs::pointer::add_wheel_vertical(entity, delta);

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
