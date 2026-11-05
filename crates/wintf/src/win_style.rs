use crate::api::*;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use windows::core::*;

//================================================================================
// クリック透過（WS_EX_TRANSPARENT 動的トグル）
//================================================================================

/// 対象 HWND の `WS_EX_TRANSPARENT` を `transparent` フラグに一致させ、`FRAMECHANGED` で反映する。
///
/// 現在の ex-style を読み戻し、`WS_EX_TRANSPARENT` ビットのみを set/clear して
/// `SetWindowLongPtr(GWL_EXSTYLE)` で書き戻し、`SetWindowPos(SWP_FRAMECHANGED | SWP_NOMOVE |
/// SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE)` で非クライアント領域へ反映する
/// (`apply_initial_state` のレシピ準拠)。他の ex-style ビット（`WS_EX_LAYERED` 等）には触れない。
pub fn apply_click_through(hwnd: HWND, transparent: bool) -> Result<()> {
    // 現在の ex-style を読み、TRANSPARENT ビットのみを目標へ一致させる。
    let current = get_window_long_ptr(hwnd, GWL_EXSTYLE)?;
    let mask = WS_EX_TRANSPARENT.0 as isize;
    let next = if transparent {
        current | mask
    } else {
        current & !mask
    };
    set_window_long_ptr(hwnd, GWL_EXSTYLE, next)?;

    // スタイル変更を非クライアント領域へ反映する。位置・サイズ・z オーダー・アクティブ化は
    // 変えず（NOMOVE/NOSIZE/NOZORDER/NOACTIVATE）、フレーム再計算（FRAMECHANGED）のみ要求する。
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
/// pilot 実証（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/REPORT.md`）:
/// DComp（`WS_EX_NOREDIRECTIONBITMAP`）窓では `WS_EX_TRANSPARENT` 単独ではマウス透過が
/// 効かず窓が全クリックを吸う。`WS_EX_LAYERED` をフラグとして併設して初めて別プロセス
/// 透過が成立し、DComp 描画は LAYERED 併設でも消えず共存する（実測 ex_style 0x280028）。
///
/// 本関数は LAYERED ビットを **立てるのみ**（落とさない・冪等）で、レイヤード描画
/// （`UpdateLayeredWindow`／`SetLayeredWindowAttributes`）は呼ばない＝当たり判定を
/// 効かせる同伴フラグ用途に限定する（R6.2）。他の ex-style ビットには触れない。
/// 既に立っている場合は Win32 呼び出しを行わず `Ok(())` を返す。
pub fn apply_layered_companion(hwnd: HWND) -> Result<()> {
    let current = get_window_long_ptr(hwnd, GWL_EXSTYLE)?;
    let mask = WS_EX_LAYERED.0 as isize;
    if current & mask != 0 {
        // 既に同伴フラグ済み（冪等・無駄な FRAMECHANGED を撃たない）。
        return Ok(());
    }
    set_window_long_ptr(hwnd, GWL_EXSTYLE, current | mask)?;

    // スタイル変更の反映レシピは apply_click_through と同一（FRAMECHANGED のみ要求）。
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
        // 実所有 HWND を用意（"Static" 定義済クラス・非表示ポップアップ）。
        // 初期 ex-style に WS_EX_TRANSPARENT 以外のビット（TOOLWINDOW/NOREDIRECTIONBITMAP）を
        // 立てておき、トグル後に「TRANSPARENT のみが変化し他ビットは保存」を検証する。
        let initial_ex = WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;

        with_test_window(initial_ex, WINDOW_STYLE(WS_POPUP.0), None, |hwnd| {
            let before =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style before") as u32;
            // 前提: 初期状態で TRANSPARENT は立っておらず、他ビットは立っている
            assert_eq!(
                before & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must start clear"
            );
            let non_transparent_before = before & !WS_EX_TRANSPARENT.0;

            // ON: TRANSPARENT を立てる
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

            // OFF: TRANSPARENT を落とす
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

            // 冪等: 既に OFF の状態で false を再適用しても不変
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
        // LAYERED 以外のビット（TOOLWINDOW/NOREDIRECTIONBITMAP）を立てた実所有 HWND を用意し、
        // 同伴フラグ適用後に「LAYERED のみが加わり他ビットは保存」を検証する。
        let initial_ex = WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;

        with_test_window(initial_ex, WINDOW_STYLE(WS_POPUP.0), None, |hwnd| {
            let before =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style before") as u32;
            assert_eq!(before & WS_EX_LAYERED.0, 0, "LAYERED must start clear");
            let non_layered_before = before & !WS_EX_LAYERED.0;

            // 同伴フラグ適用: LAYERED が立つ。
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

            // 冪等: 再適用しても不変（既フラグ時は API を撃たない early-return 経路）。
            apply_layered_companion(hwnd).expect("apply companion idempotent");
            let on2 = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on2") as u32;
            assert_eq!(on2, on, "idempotent companion must not change ex-style");

            // TRANSPARENT トグルと共存: トグル後も LAYERED は保持される。
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
