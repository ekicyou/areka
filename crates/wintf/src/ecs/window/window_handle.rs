//! WindowHandle コンポーネントとライフサイクルフック
//!
//! 作成済みウィンドウのハンドル情報とDPI関連の処理を提供する。

use bevy_ecs::prelude::*;
use tracing::debug;
use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::api::*;

use super::dpi::DPI;

// ============================================================================
// WindowHandle
// ============================================================================

/// 作成済みウィンドウのハンドル情報（システムが自動的に設定）
#[derive(Component, Debug, Copy, Clone)]
#[component(storage = "SparseSet", on_add = on_window_handle_add, on_remove = on_window_handle_remove)]
pub struct WindowHandle {
    pub hwnd: HWND,
    pub instance: HINSTANCE,
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

impl WindowHandle {
    /// ウィンドウのDPI値を取得
    pub fn get_dpi(&self) -> u32 {
        unsafe { GetDpiForWindow(self.hwnd) }
    }

    /// ウィンドウスタイルと拡張スタイルを取得
    pub fn get_style(&self) -> Result<(WINDOW_STYLE, WINDOW_EX_STYLE), String> {
        let style = match get_window_long_ptr(self.hwnd, GWL_STYLE) {
            Ok(v) => WINDOW_STYLE(v as u32),
            Err(e) => {
                return Err(format!(
                    "GetWindowLongPtrW(GWL_STYLE) failed for HWND {:?}: {:?}",
                    self.hwnd, e
                ));
            }
        };

        let ex_style = match get_window_long_ptr(self.hwnd, GWL_EXSTYLE) {
            Ok(v) => WINDOW_EX_STYLE(v as u32),
            Err(e) => {
                return Err(format!(
                    "GetWindowLongPtrW(GWL_EXSTYLE) failed for HWND {:?}: {:?}",
                    self.hwnd, e
                ));
            }
        };

        Ok((style, ex_style))
    }

    /// クライアント領域RECTをウィンドウ全体RECTに変換
    ///
    /// # Arguments
    /// * `client_rect` - クライアント領域の矩形
    ///
    /// # Returns
    /// * `Ok(RECT)` - ウィンドウ全体の矩形
    /// * `Err(String)` - 変換失敗時のエラーメッセージ
    pub fn client_to_window_rect(&self, client_rect: RECT) -> Result<RECT, String> {
        let (style, ex_style) = self.get_style()?;
        let dpi = self.get_dpi();
        if dpi == 0 {
            return Err(format!(
                "GetDpiForWindow returned 0 for HWND {:?}",
                self.hwnd
            ));
        }

        let mut rect = client_rect;
        let result = unsafe { AdjustWindowRectExForDpi(&mut rect, style, false, ex_style, dpi) };

        if result.is_err() {
            return Err(format!(
                "AdjustWindowRectExForDpi failed for HWND {:?}: {:?}",
                self.hwnd, result
            ));
        }

        Ok(rect)
    }

    /// ウィンドウ全体RECTをクライアント領域RECTに変換（逆変換）
    ///
    /// AdjustWindowRectExForDpiの逆変換を行う。
    /// 原点(0,0)での差分を計算し、その差分を使って変換する。
    ///
    /// # Arguments
    /// * `window_rect` - ウィンドウ全体の矩形
    ///
    /// # Returns
    /// * `Ok(RECT)` - クライアント領域の矩形
    /// * `Err(String)` - 変換失敗時のエラーメッセージ
    pub fn window_to_client_rect(&self, window_rect: RECT) -> Result<RECT, String> {
        // 原点でクライアント→ウィンドウ変換を行い、差分を計算
        let origin_client = RECT {
            left: 0,
            top: 0,
            right: 100, // サイズは差分計算には影響しない
            bottom: 100,
        };
        let origin_window = self.client_to_window_rect(origin_client)?;

        // 差分: ウィンドウ座標 - クライアント座標
        let left_diff = origin_window.left - origin_client.left;
        let top_diff = origin_window.top - origin_client.top;
        let right_diff = origin_window.right - origin_client.right;
        let bottom_diff = origin_window.bottom - origin_client.bottom;

        // 逆変換: クライアント座標 = ウィンドウ座標 - 差分
        Ok(RECT {
            left: window_rect.left - left_diff,
            top: window_rect.top - top_diff,
            right: window_rect.right - right_diff,
            bottom: window_rect.bottom - bottom_diff,
        })
    }

    /// クライアント領域の座標・サイズをウィンドウ全体の座標・サイズに変換
    ///
    /// # Arguments
    /// * `position` - クライアント領域の左上座標
    /// * `size` - クライアント領域のサイズ
    ///
    /// # Returns
    /// * `Ok((x, y, width, height))` - ウィンドウ全体座標
    /// * `Err(String)` - 変換失敗時のエラーメッセージ
    pub fn client_to_window_coords(
        &self,
        position: POINT,
        size: SIZE,
    ) -> Result<(i32, i32, i32, i32), String> {
        let client_rect = RECT {
            left: position.x,
            top: position.y,
            right: position.x + size.cx,
            bottom: position.y + size.cy,
        };
        let window_rect = self.client_to_window_rect(client_rect)?;
        Ok((
            window_rect.left,
            window_rect.top,
            window_rect.right - window_rect.left,
            window_rect.bottom - window_rect.top,
        ))
    }

    /// ウィンドウ全体の座標・サイズをクライアント領域の座標・サイズに変換
    ///
    /// # Arguments
    /// * `x` - ウィンドウ左上X座標
    /// * `y` - ウィンドウ左上Y座標
    /// * `width` - ウィンドウ幅
    /// * `height` - ウィンドウ高さ
    ///
    /// # Returns
    /// * `Ok((POINT, SIZE))` - クライアント領域の座標とサイズ
    /// * `Err(String)` - 変換失敗時のエラーメッセージ
    pub fn window_to_client_coords(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(POINT, SIZE), String> {
        let window_rect = RECT {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        };
        let client_rect = self.window_to_client_rect(window_rect)?;
        Ok((
            POINT {
                x: client_rect.left,
                y: client_rect.top,
            },
            SIZE {
                cx: client_rect.right - client_rect.left,
                cy: client_rect.bottom - client_rect.top,
            },
        ))
    }
}

// ============================================================================
// WindowHandle lifecycle hooks
// ============================================================================

/// WindowHandleコンポーネントが追加された直後に呼ばれるフック
fn on_window_handle_add(
    mut world: bevy_ecs::world::DeferredWorld,
    hook: bevy_ecs::lifecycle::HookContext,
) {
    let entity = hook.entity;
    if let Some(handle) = world.get::<WindowHandle>(entity) {
        let hwnd = handle.hwnd;
        debug!(
            entity = ?entity,
            hwnd = ?hwnd,
            "WindowHandle added"
        );

        // DPIコンポーネントを更新（on_window_addでシステムDPIで事前挿入済み）
        // GetDpiForWindow()の実際値が事前値と異なる場合のみ更新（マルチモニタ対応）
        let dpi_value = unsafe { GetDpiForWindow(hwnd) };
        let dpi_component = if dpi_value > 0 {
            DPI::from_dpi(dpi_value as u16, dpi_value as u16)
        } else {
            DPI::default() // 取得失敗時はデフォルト96
        };
        let existing_dpi = world.get::<DPI>(entity).copied();
        if existing_dpi != Some(dpi_component) {
            debug!(
                entity = ?entity,
                old_dpi = ?existing_dpi,
                new_dpi_x = dpi_component.dpi_x,
                new_dpi_y = dpi_component.dpi_y,
                "DPI updated from GetDpiForWindow (differs from system DPI)"
            );
            world.commands().entity(entity).insert(dpi_component);
        } else {
            debug!(
                entity = ?entity,
                dpi_x = dpi_component.dpi_x,
                dpi_y = dpi_component.dpi_y,
                "DPI unchanged (matches pre-initialized system DPI)"
            );
        }

        // Note: WindowPosは on_window_add で挿入済み（CreateWindow前に必要なため）

        // アプリに通知
        if let Some(mut app) = world.get_resource_mut::<crate::ecs::app::App>() {
            app.on_window_created(entity);
        }
    }
}

/// WindowHandleコンポーネントが削除される直前に呼ばれるフック
fn on_window_handle_remove(
    mut world: bevy_ecs::world::DeferredWorld,
    hook: bevy_ecs::lifecycle::HookContext,
) {
    let entity = hook.entity;
    // このタイミングではまだWindowHandleにアクセスできる
    if let Some(handle) = world.get::<WindowHandle>(entity) {
        let hwnd = handle.hwnd;
        debug!(
            entity = ?entity,
            hwnd = ?hwnd,
            "Entity being removed, sending WM_CLOSE"
        );

        // アプリに通知（ウィンドウカウント更新 & 必要ならメッセージ送信）
        if let Some(mut app) = world.get_resource_mut::<crate::ecs::app::App>() {
            app.on_window_destroyed(entity);
        }

        // ウィンドウクローズを非同期で要求
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}
