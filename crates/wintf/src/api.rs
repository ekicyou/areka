use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use windows::core::*;

/// GetWindowLongPtrWのラッパー
#[inline(always)]
pub(crate) fn get_window_long_ptr(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> Result<isize> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        let res = GetWindowLongPtrW(hwnd, index);
        let err = Error::from_thread();
        if err.code() != S_OK {
            return Err(err);
        }
        Ok(res)
    }
}

/// SetWindowLongPtrWのラッパー
#[inline(always)]
pub(crate) fn set_window_long_ptr(
    hwnd: HWND,
    index: WINDOW_LONG_PTR_INDEX,
    value: isize,
) -> Result<isize> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        let res = SetWindowLongPtrW(hwnd, index, value);
        if res == 0 {
            let err = Error::from_thread();
            if err.code() != S_OK {
                return Err(err);
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // set_window_long_ptr の成功経路は自プロセス所有の実在 HWND を要するため GUI 非依存の
    // ユニットテストでは検証不能（統合的な検証はウィンドウ生成を伴う examples / 実機動作に
    // 委ねる）。get_window_long_ptr の成功経路はウィンドウ生成不要の GetDesktopWindow()
    // （常に実在する読み取り可能な HWND）で検証する。null HWND による決定的なエラー経路
    // （SetLastError → Error::from_thread のエラー変換ロジック）も併せて固定する。

    #[test]
    fn get_window_long_ptr_with_null_hwnd_returns_invalid_window_handle() {
        let err = get_window_long_ptr(HWND::default(), GWL_STYLE).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn set_window_long_ptr_with_null_hwnd_returns_invalid_window_handle() {
        let err = set_window_long_ptr(HWND::default(), GWL_EXSTYLE, 0).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn get_window_long_ptr_succeeds_on_desktop_window() {
        // GetDesktopWindow はウィンドウ生成不要で常に有効な HWND を返す（GUI 非依存）。
        // デスクトップウィンドウは常に WS_VISIBLE を持つためスタイル値は非 0。
        let hwnd = unsafe { GetDesktopWindow() };
        let style = get_window_long_ptr(hwnd, GWL_STYLE)
            .expect("desktop window style must be readable");
        assert_ne!(style, 0);
    }

    #[test]
    fn get_window_long_ptr_clears_stale_last_error_before_call() {
        // 特性化: ラッパー冒頭の SetLastError(ERROR_SUCCESS) により、呼び出し前に
        // スレッドへ残留していた無関係なエラーコードが成功判定を汚染しない。
        // （このクリアがなければ残留エラーが Err として誤報告される）
        let hwnd = unsafe { GetDesktopWindow() };
        unsafe { SetLastError(ERROR_ACCESS_DENIED) };
        let res = get_window_long_ptr(hwnd, GWL_STYLE);
        assert!(res.is_ok());
    }
}
