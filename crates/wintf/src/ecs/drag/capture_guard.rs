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
pub struct CaptureGuard {
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

    /// キャプチャが既に解放済みかどうかを返す。
    #[allow(dead_code)]
    pub fn is_released(&self) -> bool {
        self.released
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

impl std::fmt::Debug for CaptureGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureGuard")
            .field("hwnd", &format_args!("0x{:X}", self.hwnd.0 as usize))
            .field("released", &self.released)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CaptureGuard の基本構造テスト（null HWND で生成可能）
    #[test]
    fn capture_guard_with_null_hwnd() {
        let guard = CaptureGuard::acquire(HWND::default());
        assert!(!guard.is_released());
        // Drop 時に ReleaseCapture が呼ばれるが null HWND なので安全
    }

    /// mark_released 後は is_released が true を返す
    #[test]
    fn mark_released_sets_flag() {
        let mut guard = CaptureGuard::acquire(HWND::default());
        assert!(!guard.is_released());
        guard.mark_released();
        assert!(guard.is_released());
        // Drop 時に ReleaseCapture は呼ばれない
    }

    /// Debug 出力が正しい形式であること
    #[test]
    fn debug_format() {
        let guard = CaptureGuard::acquire(HWND::default());
        let debug_str = format!("{:?}", guard);
        assert!(debug_str.contains("CaptureGuard"));
        assert!(debug_str.contains("released: false"));
    }
}
