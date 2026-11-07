// 断りの無い要件の番号と design.md の参照は spec `areka-P0-choice-render` のもの（別の spec のものはその場で名を書く）。

// ── ハイライトスタイル解決（ResolvedChoiceStyle::resolve / paint） ──
//
// design.md「純粋層 / ChoicePure」Service Interface（ResolvedChoiceStyle enum + resolve/paint）・
// 縮退表（cursor.style underline 系→SquareFill／cursor.blendmethod ROP 系→none 扱い／
// cursor.* 全キー未指定→Invert）・正典確定（cursor.* マップ「既定 square」・fixture 実導出形＝
// square 塗り(105,25,25)＋白文字／矩形反転縮退「塗り=既定 font.color・文字=各成分 255−c」）。

use super::*;
use areka_parsers::balloon::{BalloonCursor, CursorColor};
use log_capture_kit::capture;

/// cursor 全体を組む簡易ビルダ。
fn cursor(
    style: Option<&str>,
    brush: (Option<u8>, Option<u8>, Option<u8>),
    font: (Option<u8>, Option<u8>, Option<u8>),
    blend: Option<&str>,
) -> BalloonCursor {
    BalloonCursor::new(
        style.map(str::to_string),
        CursorColor::new(brush.0, brush.1, brush.2),
        CursorColor::new(None, None, None),
        CursorColor::new(font.0, font.1, font.2),
        blend.map(str::to_string),
    )
}

// ── resolve: 未指定 → Invert（縮退ではない） ──

/// cursor 不在（`None`）→ Invert（未指定バルーン判定・4.3/6.1）。
#[test]
fn resolve_none_cursor_is_invert() {
    assert_eq!(
        ResolvedChoiceStyle::resolve(None, (0, 0, 0)),
        ResolvedChoiceStyle::Invert
    );
}

/// cursor.* 全キー未指定（`BalloonCursor::default()`）→ Invert（4.3/6.1）。
#[test]
fn resolve_all_unspecified_cursor_is_invert() {
    let c = BalloonCursor::default();
    assert_eq!(
        ResolvedChoiceStyle::resolve(Some(&c), (12, 34, 56)),
        ResolvedChoiceStyle::Invert
    );
}

// ── resolve: style=none → NoMarker ──

/// `cursor.style,none`（正典・マーカー無し）→ NoMarker。
#[test]
fn resolve_style_none_is_no_marker() {
    let c = cursor(Some("none"), (None, None, None), (None, None, None), None);
    assert_eq!(
        ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
        ResolvedChoiceStyle::NoMarker
    );
}

/// `cursor.style,NONE`（大文字）も大小を問わず NoMarker（黒い四角の印に変わらない）。
#[test]
fn resolve_style_upper_none_is_no_marker() {
    let c = cursor(Some("NONE"), (None, None, None), (None, None, None), None);
    assert_eq!(
        ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
        ResolvedChoiceStyle::NoMarker
    );
}

// ── resolve: fixture square → SquareFill{(105,25,25),(255,255,255)} ──

/// fixture 実導出形（square＋brush(105,25,25)＋font(255,255,255)）→ SquareFill。
#[test]
fn resolve_fixture_square_is_square_fill() {
    // fixture 実導出の cursor.*（pen・blendmethod は未指定）。
    let c = cursor(
        Some("square"),
        (Some(105), Some(25), Some(25)),
        (Some(255), Some(255), Some(255)),
        None,
    );
    assert_eq!(
        ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        }
    );
}

/// style 未指定でも色/キーが在れば「既定 square」→ SquareFill（正典確定 cursor.* マップ）。
#[test]
fn resolve_specified_colors_without_style_defaults_to_square_fill() {
    let c = cursor(
        None,
        (Some(105), Some(25), Some(25)),
        (Some(255), Some(255), Some(255)),
        None,
    );
    assert_eq!(
        ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        }
    );
}

// ── paint: 描画実行用 (fill, text) 正規形 ──

/// SquareFill.paint → Some((fill, text))（dfc 非依存）。
#[test]
fn paint_square_fill_returns_fill_and_text() {
    let s = ResolvedChoiceStyle::SquareFill {
        fill: (105, 25, 25),
        text: (255, 255, 255),
    };
    assert_eq!(
        s.paint((77, 88, 99)),
        Some(((105, 25, 25), (255, 255, 255)))
    );
}

/// Invert.paint（dfc=(0,0,0)）→ 塗り=(0,0,0)・文字=各成分 255−c=(255,255,255)（古典反転同観）。
#[test]
fn paint_invert_black_default_is_black_fill_white_text() {
    assert_eq!(
        ResolvedChoiceStyle::Invert.paint((0, 0, 0)),
        Some(((0, 0, 0), (255, 255, 255)))
    );
}

/// Invert.paint（dfc=(10,20,30)）→ 塗り=既定 font 色・文字=(245,235,225)（各成分 255−c）。
#[test]
fn paint_invert_uses_default_font_fill_and_per_component_complement() {
    assert_eq!(
        ResolvedChoiceStyle::Invert.paint((10, 20, 30)),
        Some(((10, 20, 30), (245, 235, 225)))
    );
}

/// NoMarker.paint → None（マーカー無し＝素描画・dfc 非依存）。
#[test]
fn paint_no_marker_is_none() {
    assert_eq!(ResolvedChoiceStyle::NoMarker.paint((10, 20, 30)), None);
}

// ══ spec real-run-log-noise T6（要件 2.5・2.6・5.4）: resolve は記録を出さず、縮退の値は取り出し口が返す ══

/// 白抜きの fixture 色で `style`・`blendmethod` だけを替えた cursor。
fn styled(style: Option<&str>, blend: Option<&str>) -> BalloonCursor {
    cursor(
        style,
        (Some(105), Some(25), Some(25)),
        (Some(255), Some(255), Some(255)),
        blend,
    )
}

/// ROP 系・下線系のバルーンで resolve の返り値は今のまま（SquareFill）・記録は 0 件
/// （`draw.rs` からの 2 度目の呼び出しも同じく無音になる・`real-run-log-noise` 要件 2.5・2.7）。blendmethod は
/// ROP 系でも `none` でも variant を変えない（色ベース・警告は登録の側が出す）。
#[test]
fn resolve_rop_and_underline_keeps_answer_and_writes_no_record() {
    let square = ResolvedChoiceStyle::SquareFill {
        fill: (105, 25, 25),
        text: (255, 255, 255),
    };
    for c in [
        styled(Some("square"), Some("mergepennot")),
        styled(Some("underline"), None),
        styled(Some("square+underline"), Some("xor")),
        styled(Some("square"), Some("notmaskpen")),
        styled(Some("square"), Some("none")),
    ] {
        let (answer, events) = capture(|| ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)));
        assert_eq!(answer, square, "返り値は今のまま: {c:?}");
        assert!(
            events.is_empty(),
            "resolve は記録を出さない: {c:?} → {events:?}"
        );
    }
}

/// 取り出し口は ROP 系の `blendmethod` と下線系の `style` の元の値を返す。
#[test]
fn cursor_fallback_returns_rop_blendmethod_and_underline_style() {
    assert_eq!(
        CursorFallback::of(&styled(Some("square"), Some("mergepennot"))),
        CursorFallback {
            blendmethod: Some("mergepennot".to_string()),
            underline_style: None,
            unknown_style: None,
        }
    );
    assert_eq!(
        CursorFallback::of(&styled(Some("underline"), None)),
        CursorFallback {
            blendmethod: None,
            underline_style: Some("underline".to_string()),
            unknown_style: None,
        }
    );
    assert_eq!(
        CursorFallback::of(&styled(Some("square+underline"), None)),
        CursorFallback {
            blendmethod: None,
            underline_style: Some("square+underline".to_string()),
            unknown_style: None,
        }
    );
}

/// 下線系の `style` は大小を問わない（ukadoc の値と大小だけ違う `UNDERLINE`・`Square+Underline` も
/// 下線系として返し、正典に無い値の側へは落とさない）。
#[test]
fn cursor_fallback_matches_underline_style_ignoring_case() {
    for style in ["UNDERLINE", "Square+Underline"] {
        assert_eq!(
            CursorFallback::of(&styled(Some(style), None)),
            CursorFallback {
                blendmethod: None,
                underline_style: Some(style.to_string()),
                unknown_style: None,
            },
            "{style}"
        );
    }
}

/// `blendmethod` が `none`（大小無視）・未指定、`style` が `square`・未指定・`none` では縮退の値は無し。
#[test]
fn cursor_fallback_is_empty_for_none_unspecified_and_square() {
    for c in [
        styled(Some("square"), Some("none")),
        styled(Some("square"), Some("NONE")),
        styled(Some("square"), None),
        styled(None, None),
        styled(Some("none"), None),
        styled(Some("SQUARE"), None),
        // `none` も大小無視で正典（spec audit-emo-text-state-actor の 2 回目のかけ直し R2-12）。
        styled(Some("NONE"), None),
        BalloonCursor::default(),
    ] {
        assert_eq!(CursorFallback::of(&c), CursorFallback::default(), "{c:?}");
    }
}

// ══ spec audit-emo-text-state-actor（2・3 回目のかけ直し R2-12・R3-4）: 正典の 4 つでない style は元の値を返す ══

/// 正典の 4 つのどれでもない `style` は元の値を返す（描く形は既定の square）。綴り誤りの `squre` の
/// ほか、`underline` を含むだけの `squareunderline`（`+` の抜け）・`underlined` も下線系に数えない。
#[test]
fn cursor_fallback_returns_unknown_style() {
    for style in ["squre", "squareunderline", "underlined"] {
        let c = styled(Some(style), None);
        assert_eq!(
            CursorFallback::of(&c),
            CursorFallback {
                blendmethod: None,
                underline_style: None,
                unknown_style: Some(style.to_string()),
            }
        );
        assert!(matches!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill { .. }
        ));
    }
}
