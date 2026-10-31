//! 窓を作るときの拡張スタイルと常に手前の印（`stay-on-top-directives` 要件 2.3・3.4・4.4・4.5）。
//!
//! design.md「CreationExStyle」: 印のある窓は `compute_ex_style` に `WS_EX_TOPMOST` を足し、
//! 印の無い窓は今と同じ。

use super::*;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;

/// 印のある窓は、今の拡張スタイルに `WS_EX_TOPMOST` を足したもので作る。
#[test]
fn creation_ex_style_adds_topmost_when_marked() {
    let style = WindowStyle::default();
    let ex = creation_ex_style(&style, true);
    assert_eq!(ex, compute_ex_style(&style) | WS_EX_TOPMOST);
}

/// 印の無い窓の拡張スタイルは今と同じ（`WS_EX_TOPMOST` を足さない）。
#[test]
fn creation_ex_style_unchanged_when_unmarked() {
    let style = WindowStyle::default();
    let ex = creation_ex_style(&style, false);
    assert_eq!(ex, compute_ex_style(&style));
    assert_eq!((ex & WS_EX_TOPMOST).0, 0, "既定の窓は帯に入れない");
}
