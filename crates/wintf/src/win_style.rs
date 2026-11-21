use crate::api::*;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use windows::core::*;

//================================================================================
// クリック透過（WS_EX_TRANSPARENT 動的トグル）
//================================================================================

/// 対象 HWND の `WS_EX_TRANSPARENT` を `transparent` フラグに一致させ、`FRAMECHANGED` で反映する。
/// 他の ex-style ビット（`WS_EX_LAYERED` 等）には触れない。
pub fn apply_click_through(hwnd: HWND, transparent: bool) -> Result<()> {
    let current = get_window_long_ptr(hwnd, GWL_EXSTYLE)?;
    let mask = WS_EX_TRANSPARENT.0 as isize;
    let next = if transparent {
        current | mask
    } else {
        current & !mask
    };
    set_window_long_ptr(hwnd, GWL_EXSTYLE, next)?;

    // SAFETY: Win32 境界。SetWindowPos に安全ラッパは無いため直接呼ぶ。所有権・寿命に影響しない。
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}

/// 対象 HWND に `WS_EX_LAYERED` を同伴フラグとして立てる（クリック透過の必須条件）。
///
/// DComp（`WS_EX_NOREDIRECTIONBITMAP`）窓では `WS_EX_TRANSPARENT` 単独ではマウス透過が
/// 効かず窓が全クリックを吸う。`WS_EX_LAYERED` を併設して初めて別プロセス透過が成立し、
/// DComp 描画は消えずに共存する（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/REPORT.md`）。
///
/// LAYERED ビットを立てるだけで（落とさない・冪等）、レイヤード描画
/// （`UpdateLayeredWindow`／`SetLayeredWindowAttributes`）は呼ばない。
/// 既に立っている場合は Win32 を呼ばずに `Ok(())` を返す。
pub fn apply_layered_companion(hwnd: HWND) -> Result<()> {
    let current = get_window_long_ptr(hwnd, GWL_EXSTYLE)?;
    let mask = WS_EX_LAYERED.0 as isize;
    if current & mask != 0 {
        return Ok(());
    }
    set_window_long_ptr(hwnd, GWL_EXSTYLE, current | mask)?;

    // SAFETY: Win32 境界。SetWindowPos に安全ラッパは無いため直接呼ぶ。所有権・寿命に影響しない。
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    //================================================================================
    // apply_click_through: WS_EX_TRANSPARENT 動的トグル
    //================================================================================

    #[test]
    fn apply_click_through_toggles_only_transparent_bit() {
        let initial_ex = WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;

        with_test_window(initial_ex, WINDOW_STYLE(WS_POPUP.0), None, |hwnd| {
            let before =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style before") as u32;
            assert_eq!(
                before & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must start clear"
            );
            let non_transparent_before = before & !WS_EX_TRANSPARENT.0;

            apply_click_through(hwnd, true).expect("apply true");
            let on = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on") as u32;
            assert_ne!(
                on & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must be set after true"
            );
            assert_eq!(
                on & !WS_EX_TRANSPARENT.0,
                non_transparent_before,
                "non-TRANSPARENT bits must be preserved after ON"
            );

            apply_click_through(hwnd, false).expect("apply false");
            let off = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style off") as u32;
            assert_eq!(
                off & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must be clear after false"
            );
            assert_eq!(
                off & !WS_EX_TRANSPARENT.0,
                non_transparent_before,
                "non-TRANSPARENT bits must be preserved after OFF"
            );

            apply_click_through(hwnd, false).expect("apply false idempotent");
            let off2 = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style off2") as u32;
            assert_eq!(off2, off, "idempotent OFF must not change ex-style");
        });
    }

    #[test]
    fn apply_click_through_null_hwnd_returns_err() {
        // 無効 HWND では GetWindowLongPtrW が失敗し Err が伝播する（ウィンドウ生成不要）
        assert!(apply_click_through(HWND::default(), true).is_err());
    }

    //================================================================================
    // apply_layered_companion: WS_EX_LAYERED 同伴フラグ（pilot 必須条件）
    //================================================================================

    #[test]
    fn apply_layered_companion_sets_layered_and_preserves_other_bits() {
        let initial_ex = WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;

        with_test_window(initial_ex, WINDOW_STYLE(WS_POPUP.0), None, |hwnd| {
            let before =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style before") as u32;
            assert_eq!(before & WS_EX_LAYERED.0, 0, "LAYERED must start clear");
            let non_layered_before = before & !WS_EX_LAYERED.0;

            apply_layered_companion(hwnd).expect("apply companion");
            let on = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on") as u32;
            assert_ne!(
                on & WS_EX_LAYERED.0,
                0,
                "LAYERED must be set after companion"
            );
            assert_eq!(
                on & !WS_EX_LAYERED.0,
                non_layered_before,
                "non-LAYERED bits must be preserved"
            );

            // 既に立っていれば API を撃たない経路。
            apply_layered_companion(hwnd).expect("apply companion idempotent");
            let on2 = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on2") as u32;
            assert_eq!(on2, on, "idempotent companion must not change ex-style");

            apply_click_through(hwnd, true).expect("toggle on");
            let toggled =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style toggled") as u32;
            assert_ne!(
                toggled & WS_EX_LAYERED.0,
                0,
                "LAYERED must survive TRANSPARENT toggle"
            );
            assert_ne!(toggled & WS_EX_TRANSPARENT.0, 0, "TRANSPARENT must be set");
        });
    }

    #[test]
    fn apply_layered_companion_null_hwnd_returns_err() {
        // 無効 HWND では GetWindowLongPtrW が失敗し Err が伝播する（ウィンドウ生成不要）
        assert!(apply_layered_companion(HWND::default()).is_err());
    }
}

#[cfg(test)]
#[path = "win_style_accept_files_tests.rs"]
mod accept_files_tests;
