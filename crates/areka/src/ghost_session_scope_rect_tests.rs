//! 窓の矩形の読み手の決定論テスト（数の形と選択子の読み）。

use super::*;
use areka_sylphya::parse_dotted;

/// 主の画面より左上の画面の矩形（SSP の段 1 の値）。
const LEFT_SCREEN: ScreenRect = ScreenRect {
    left: -782,
    top: 201,
    right: 49,
    bottom: 1227,
};

/// スコープ 0 にキャラクターとバルーンの印を持つ表と、常に `LEFT_SCREEN` を返す偽の OS。
fn source(target: RectTarget) -> ScopeRectSource {
    let mut map = BTreeMap::new();
    map.insert(
        0,
        ScopeWindowKeys {
            character: Some(WindowKey(1)),
            balloon: Some(WindowKey(2)),
        },
    );
    ScopeRectSource::new(
        target,
        Arc::new(RwLock::new(ScopeWindowTable(map))),
        Arc::new(|_| Some(LEFT_SCREEN)),
    )
}

fn read(src: &ScopeRectSource, name: &str) -> Option<String> {
    src.read(&parse_dotted(name).unwrap())
}

#[test]
fn rect_text_joins_four_numbers_without_spaces() {
    assert_eq!(rect_text(LEFT_SCREEN), "-782,201,49,1227");
}

#[test]
fn index_selector_answers_the_window_rect() {
    let ch = source(RectTarget::Character);
    let ba = source(RectTarget::Balloon);
    assert_eq!(
        read(&ch, "currentghost.scope(0).rect").as_deref(),
        Some("-782,201,49,1227")
    );
    assert_eq!(
        read(&ba, "currentghost.balloon.scope(0).rect").as_deref(),
        Some("-782,201,49,1227")
    );
}

#[test]
fn negative_selector_is_a_scope_without_window() {
    let ch = source(RectTarget::Character);
    let ba = source(RectTarget::Balloon);
    assert_eq!(
        read(&ch, "currentghost.scope(-1).rect").as_deref(),
        Some("0,0,0,0")
    );
    assert_eq!(read(&ba, "currentghost.balloon.scope(-1).rect"), None);
}

#[test]
fn malformed_selector_has_no_value() {
    let ch = source(RectTarget::Character);
    let ba = source(RectTarget::Balloon);
    for sel in ["x", "", "+1", "-"] {
        assert_eq!(
            read(&ch, &format!("currentghost.scope({sel}).rect")),
            None,
            "{sel}"
        );
        assert_eq!(
            read(&ba, &format!("currentghost.balloon.scope({sel}).rect")),
            None,
            "{sel}"
        );
    }
}

/// 壊れた窓の印（0）は値なしで、呼んだスレッドの DPI の扱いは呼ぶ前と同じに戻る（要件 4.4・1.3）。
#[test]
fn os_window_rect_invalid_handle_is_none_and_restores_dpi_context() {
    use windows::Win32::UI::HiDpi::{AreDpiAwarenessContextsEqual, GetThreadDpiAwarenessContext};
    let before = unsafe { GetThreadDpiAwarenessContext() };
    assert_eq!(os_window_rect(WindowKey(0)), None);
    let after = unsafe { GetThreadDpiAwarenessContext() };
    assert!(unsafe { AreDpiAwarenessContextsEqual(before, after) }.as_bool());
}
