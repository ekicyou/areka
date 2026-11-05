//! 受け入れの宣言（`WS_EX_ACCEPTFILES`）が透過の付け外しで消えないことの固定（要件 1.10）。
//!
//! `apply_click_through` は `WS_EX_TRANSPARENT` のビットだけを読み書きする。実 HWND に宣言を
//! 付けて付け外しを 36 回繰り返し、宣言が残り透過のビットが最後の値であることを判定する。

use super::*;

#[test]
fn accept_files_survives_36_toggles() {
    with_test_window(
        WS_EX_ACCEPTFILES | WS_EX_TOOLWINDOW,
        WINDOW_STYLE(WS_POPUP.0),
        None,
        |hwnd| {
            let read = || get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style") as u32;
            // 較正: 宣言が最初から在ること（0 件の主張が空振りでない）
            assert_ne!(
                read() & WS_EX_ACCEPTFILES.0,
                0,
                "ACCEPTFILES must start set"
            );

            let mut last = false;
            for i in 0..36 {
                last = i % 2 == 0;
                apply_click_through(hwnd, last).expect("apply_click_through");
                assert_ne!(
                    read() & WS_EX_ACCEPTFILES.0,
                    0,
                    "ACCEPTFILES lost after toggle {i}"
                );
            }

            let after = read();
            assert_ne!(after & WS_EX_ACCEPTFILES.0, 0, "ACCEPTFILES must survive");
            assert_ne!(after & WS_EX_TOOLWINDOW.0, 0, "TOOLWINDOW must survive");
            assert_eq!(
                after & WS_EX_TRANSPARENT.0 != 0,
                last,
                "TRANSPARENT must match the last toggle"
            );
        },
    );
}
