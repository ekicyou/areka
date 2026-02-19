//! ウィンドウライフサイクルおよびディスプレイ変更ハンドラ
//!
//! WM_NCCREATE, WM_NCDESTROY, WM_ERASEBKGND, WM_PAINT, WM_CLOSE, WM_DISPLAYCHANGE

#![allow(non_snake_case)]

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_NCCREATE: ウィンドウ作成時の非クライアント領域初期化
///
/// Entity IDをGWLP_USERDATAに保存する
#[inline]
pub(super) fn WM_NCCREATE(
    hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    let cs = lparam.0 as *const CREATESTRUCTW;
    if !cs.is_null() {
        let entity_bits = unsafe { (*cs).lpCreateParams as isize };
        // Entity IDをGWLP_USERDATAに保存（ID 0も有効）
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, entity_bits) };
    }
    None // DefWindowProcWに委譲
}

/// WM_NCDESTROY: ウィンドウ破棄時のクリーンアップ
///
/// Entity IDを取得してエンティティを削除し、GWLP_USERDATAをクリアする
#[inline]
pub(super) fn WM_NCDESTROY(
    hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // Entity IDを取得してエンティティを削除
    if let Some(entity) = super::get_entity_from_hwnd(hwnd) {
        if let Some(world) = super::try_get_ecs_world() {
            let mut world = world.borrow_mut();

            // エンティティを削除（関連する全コンポーネントも削除される）
            // on_window_handle_removedシステムが自動的にApp通知を行う
            world.world_mut().despawn(entity);
        }
    }

    // GWLP_USERDATAをクリア
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };

    None // DefWindowProcWに委譲
}

/// WM_ERASEBKGND: 背景消去要求
///
/// ULW が全画面を管理するため、背景消去をスキップする
#[inline]
pub(super) fn WM_ERASEBKGND(
    _hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    Some(LRESULT(1)) // 背景消去をスキップ
}

/// WM_PAINT: 再描画要求
///
/// - `CompositionMode::DComp` → `DefWindowProcW` に委譲（DComp は OS 管理）
/// - `CompositionMode::ULW` またはフォールバック → `BeginPaint`/`EndPaint` 最小ペア
#[inline]
pub(super) fn WM_PAINT(
    hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // Entity から CompositionMode を判定
    let is_dcomp = if let Some(entity) = super::get_entity_from_hwnd(hwnd) {
        if let Some(world_rc) = super::try_get_ecs_world() {
            if let Ok(world_borrow) = world_rc.try_borrow() {
                world_borrow
                    .world()
                    .get::<crate::ecs::window::Window>(entity)
                    .map(|w| {
                        w.composition_mode() == crate::ecs::window::CompositionMode::DComp
                    })
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if is_dcomp {
        // DComp モード: DefWindowProcW に委譲
        None
    } else {
        // ULW モード: BeginPaint/EndPaint 最小ペア
        use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
        let mut ps = PAINTSTRUCT::default();
        unsafe {
            let _ = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
        }
        Some(LRESULT(0))
    }
}

/// WM_CLOSE: ウィンドウクローズ要求
///
/// DestroyWindowを呼び出してウィンドウを破棄する
#[inline]
pub(super) fn WM_CLOSE(
    hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    let _ = unsafe { DestroyWindow(hwnd) };
    Some(LRESULT(0))
}

/// WM_DISPLAYCHANGE: ディスプレイ構成変更通知
///
/// Appリソースのmark_display_changeを呼び出す
#[inline]
pub(super) fn WM_DISPLAYCHANGE(
    _hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    if let Some(world) = super::try_get_ecs_world() {
        if let Ok(mut world_borrow) = world.try_borrow_mut() {
            if let Some(mut app) = world_borrow
                .world_mut()
                .get_resource_mut::<crate::ecs::App>()
            {
                app.mark_display_change();
            }
        }
    }
    None // DefWindowProcWに委譲
}
