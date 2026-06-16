#![allow(unused_variables)]

use crate::api::get_window_long_ptr;
use ambassador::*;
use windows::Win32::{Foundation::*, UI::HiDpi::*, UI::WindowsAndMessaging::*};
use windows::core::*;

#[delegatable_trait]
pub trait WinState {
    /// Get the HWND value.
    fn hwnd(&self) -> HWND;

    /// Set the HWND value.
    fn set_hwnd(&mut self, hwnd: HWND);

    /// Get whether mouse tracking is enabled.
    fn mouse_tracking(&self) -> bool {
        true
    }

    /// Set whether mouse tracking is enabled.
    fn set_mouse_tracking(&mut self, tracking: bool) {}

    /// Get the stored DPI value.
    fn dpi(&self) -> f32;

    /// Set the stored DPI value.
    fn set_dpi(&mut self, dpi: f32);

    /// Handle a WM_DPICHANGED message by extracting the new DPI and updating the stored DPI value.
    fn set_dpi_change_message(&mut self, wparam: WPARAM, lparam: LPARAM) {
        let dpi = (wparam.0 as u16) as f32;
        self.set_dpi(dpi);
    }

    /// Calculate the effective window size (including borders, title bar, etc) for a given client area size.
    /// This is useful when creating a window with a specific client area size.
    fn effective_window_size(
        &self,
        client_size: windows_numerics::Vector2,
    ) -> Result<windows_numerics::Vector2> {
        let dpi = self.dpi();
        let scale = dpi / 96.0;
        let client_size_px = windows_numerics::Vector2 {
            X: client_size.X * scale,
            Y: client_size.Y * scale,
        };

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: client_size_px.X.ceil() as i32,
            bottom: client_size_px.Y.ceil() as i32,
        };
        let hwnd = self.hwnd();
        let style = WINDOW_STYLE(get_window_long_ptr(hwnd, GWL_STYLE)? as u32);
        let ex_style = WINDOW_EX_STYLE(get_window_long_ptr(hwnd, GWL_EXSTYLE)? as u32);
        let dpi_value = dpi as u32;
        unsafe { AdjustWindowRectExForDpi(&mut rect, style, false, ex_style, dpi_value)? }
        Ok(windows_numerics::Vector2 {
            X: (rect.right - rect.left) as f32,
            Y: (rect.bottom - rect.top) as f32,
        })
    }
}

#[derive(Debug)]
pub struct SimpleWinState {
    hwnd: HWND,
    mouse_tracking: bool,
    dpi: f32,
}

impl Default for SimpleWinState {
    fn default() -> Self {
        Self {
            hwnd: HWND::default(),
            mouse_tracking: false,
            dpi: 96.0,
        }
    }
}

impl WinState for SimpleWinState {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn set_hwnd(&mut self, hwnd: HWND) {
        self.hwnd = hwnd;
    }

    fn mouse_tracking(&self) -> bool {
        self.mouse_tracking
    }

    fn set_mouse_tracking(&mut self, tracking: bool) {
        self.mouse_tracking = tracking;
    }

    fn dpi(&self) -> f32 {
        self.dpi
    }

    fn set_dpi(&mut self, dpi: f32) {
        self.dpi = dpi;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_win_state_default_values() {
        let s = SimpleWinState::default();
        assert_eq!(s.hwnd(), HWND::default());
        assert!(!s.mouse_tracking());
        assert_eq!(s.dpi(), 96.0);
    }

    #[test]
    fn simple_win_state_setters_roundtrip() {
        let mut s = SimpleWinState::default();
        let hwnd = HWND(0x1234 as *mut _);
        s.set_hwnd(hwnd);
        s.set_mouse_tracking(true);
        s.set_dpi(144.0);
        assert_eq!(s.hwnd(), hwnd);
        assert!(s.mouse_tracking());
        assert_eq!(s.dpi(), 144.0);
    }

    /// 必須メソッドのみ実装した最小実装（トレイトのデフォルト実装の検証用）
    struct MinimalState {
        hwnd: HWND,
        dpi: f32,
    }

    impl WinState for MinimalState {
        fn hwnd(&self) -> HWND {
            self.hwnd
        }
        fn set_hwnd(&mut self, hwnd: HWND) {
            self.hwnd = hwnd;
        }
        fn dpi(&self) -> f32 {
            self.dpi
        }
        fn set_dpi(&mut self, dpi: f32) {
            self.dpi = dpi;
        }
    }

    #[test]
    fn trait_default_mouse_tracking_is_true_and_setter_is_noop() {
        let mut s = MinimalState {
            hwnd: HWND::default(),
            dpi: 96.0,
        };
        // デフォルト実装: mouse_tracking() は常に true、set_mouse_tracking は no-op
        assert!(s.mouse_tracking());
        s.set_mouse_tracking(false);
        assert!(s.mouse_tracking());
    }

    #[test]
    fn set_dpi_change_message_extracts_low_word_as_dpi() {
        // WM_DPICHANGED: wparam の下位 16bit（X 軸 DPI）のみ採用し、
        // 上位 16bit（Y 軸 DPI）と lparam（推奨 RECT）は無視される
        let mut s = SimpleWinState::default();
        s.set_dpi_change_message(WPARAM(96usize | (144usize << 16)), LPARAM(0));
        assert_eq!(s.dpi(), 96.0);
    }

    #[test]
    fn set_dpi_change_message_with_equal_xy_dpi() {
        let mut s = SimpleWinState::default();
        s.set_dpi_change_message(WPARAM(120usize | (120usize << 16)), LPARAM(0));
        assert_eq!(s.dpi(), 120.0);
    }

    #[test]
    fn effective_window_size_with_null_hwnd_returns_err() {
        // HWND 未設定（null）ではスタイル取得（GetWindowLongPtrW）が失敗し
        // Err が伝播する（ウィンドウ生成不要の GUI 非依存エラー経路）
        let s = SimpleWinState::default();
        let r = s.effective_window_size(windows_numerics::Vector2 { X: 100.0, Y: 100.0 });
        assert!(r.is_err());
    }
}
