// ── タスク 5.3: ハイライトスタイル解決（ResolvedChoiceStyle::resolve / paint） ──
//
// design.md「純粋層 / ChoicePure」Service Interface（ResolvedChoiceStyle enum + resolve/paint）・
// 縮退表（cursor.style underline 系→SquareFill／cursor.blendmethod ROP 系→none 扱い／
// cursor.* 全キー未指定→Invert）・正典確定（cursor.* マップ「既定 square」・fixture 実導出形＝
// square 塗り(105,25,25)＋白文字／矩形反転縮退「塗り=既定 font.color・文字=各成分 255−c」）。

use super::*;
use areka_parsers::balloon::{BalloonCursor, CursorColor};

/// fixture 実導出の cursor.*（square・brush=(105,25,25)・font=(255,255,255)・pen/blend 未指定）。
fn fixture_cursor() -> BalloonCursor {
    BalloonCursor::new(
        Some("square".to_string()),
        CursorColor::new(Some(105), Some(25), Some(25)), // brush.color＝矩形内色
        CursorColor::new(None, None, None),              // pen.color（M1 非参照）
        CursorColor::new(Some(255), Some(255), Some(255)), // font.color＝hover 白文字
        None,                                            // blendmethod（既定 none）
    )
}

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

// ── resolve: 未指定 → Invert（M1 実導出・縮退ではない） ──

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

// ── resolve: fixture square → SquareFill{(105,25,25),(255,255,255)} ──

/// fixture 実導出形（square＋brush(105,25,25)＋font(255,255,255)）→ SquareFill。
#[test]
fn resolve_fixture_square_is_square_fill() {
    let c = fixture_cursor();
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

// ── resolve: underline 系 → warn-once + SquareFill 縮退（縮退表・6.5） ──

/// `cursor.style,underline` → SquareFill へ縮退（在る色を採る・warn は解決時 1 回）。
#[test]
fn resolve_underline_degrades_to_square_fill() {
    let c = cursor(
        Some("underline"),
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

/// `cursor.style,square+underline`（underline 系）→ SquareFill へ縮退。
#[test]
fn resolve_square_plus_underline_degrades_to_square_fill() {
    let c = cursor(
        Some("square+underline"),
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

// ── resolve: blendmethod ROP 系 → warn-once + none 扱い（variant は変えない・6.5） ──

/// `cursor.blendmethod,notmaskpen`（ROP 系）→ none 扱い（色ベース）へ縮退し variant は
/// style（square）どおり SquareFill のまま（blend は無視・warn は解決時 1 回）。
#[test]
fn resolve_rop_blendmethod_is_treated_as_none_and_keeps_square_fill() {
    let c = cursor(
        Some("square"),
        (Some(105), Some(25), Some(25)),
        (Some(255), Some(255), Some(255)),
        Some("notmaskpen"),
    );
    assert_eq!(
        ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        }
    );
}

/// `cursor.blendmethod,none`（大小無視）は縮退警告を出さず variant はスタイルどおり。
#[test]
fn resolve_blendmethod_none_keeps_square_fill_without_degrade() {
    let c = cursor(
        Some("square"),
        (Some(105), Some(25), Some(25)),
        (Some(255), Some(255), Some(255)),
        Some("none"),
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
    assert_eq!(s.paint((77, 88, 99)), Some(((105, 25, 25), (255, 255, 255))));
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
