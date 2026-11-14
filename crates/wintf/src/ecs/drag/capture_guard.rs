//! マウスキャプチャの RAII ガード
//!
//! `SetCapture` / `ReleaseCapture` のライフサイクルを RAII パターンで管理する。
//! パニック時にもスタック巻き戻しで確実に `ReleaseCapture` が呼ばれることを保証する。

use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};

/// マウスキャプチャの RAII ガード
///
/// `acquire()` で `SetCapture` を呼び出し、`Drop` で `ReleaseCapture` を呼び出す。
/// `WM_CAPTURECHANGED` 受信時は `mark_released()` を呼ぶことで、
/// Drop 時の `ReleaseCapture` をスキップする（OS が既に解放済みのため）。
pub(crate) struct CaptureGuard {
    hwnd: HWND,
    released: bool,
}

impl CaptureGuard {
    /// キャプチャを取得して新しいガードを生成する。
    ///
    /// Win32 `SetCapture` は常に成功し、前の capture owner の HWND を返す。
    pub fn acquire(hwnd: HWND) -> Self {
        unsafe {
            SetCapture(hwnd);
        }
        debug!(
            hwnd = format!("0x{:X}", hwnd.0 as usize),
            "[CaptureGuard] Capture acquired"
        );
        Self {
            hwnd,
            released: false,
        }
    }

    /// WM_CAPTURECHANGED 受信時に呼ぶ。
    ///
    /// キャプチャは OS により既に解放されているため、
    /// Drop 時の `ReleaseCapture` 呼び出しをスキップするよう内部フラグをセットする。
    pub fn mark_released(&mut self) {
        self.released = true;
    }

    /// キャプチャを取った窓の HWND を返す（左ボタンを離した窓との一致に使う）。
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        if !self.released {
            let _ = unsafe { ReleaseCapture() };
            debug!(
                hwnd = format!("0x{:X}", self.hwnd.0 as usize),
                "[CaptureGuard] Capture released via Drop"
            );
        }
    }
}
