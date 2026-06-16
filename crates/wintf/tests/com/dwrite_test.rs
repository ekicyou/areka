//! com/dwrite.rs — DirectWrite ラッパーのヘッドレステスト
//!
//! DirectWrite はフォントシステムのみで GPU デバイスを要しないため
//! ヘッドレスで全経路を検証できる。`create_text_layout` の手書き
//! wcslen ループ（null / 空文字列 / 多バイト / サロゲートペア）を重点的に固定する。

use windows::Win32::Graphics::DirectWrite::*;
use windows::core::*;
use wintf::com::dwrite::*;

fn make_factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWriteCreateFactory")
}

fn make_format(factory: &IDWriteFactory2) -> IDWriteTextFormat {
    factory
        .create_text_format(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            16.0,
            w!("en-us"),
        )
        .expect("CreateTextFormat")
}

fn make_layout(text: PCWSTR) -> IDWriteTextLayout {
    let factory = make_factory();
    let format = make_format(&factory);
    factory
        .create_text_layout(text, &format, 1000.0, 1000.0)
        .expect("CreateTextLayout")
}

// ============================================================
// ファクトリ / フォーマット生成
// ============================================================

#[test]
fn dwrite_factory_and_text_format_can_be_created_headless() {
    let factory = make_factory();
    let format = make_format(&factory);
    format
        .set_text_alignment(DWRITE_TEXT_ALIGNMENT_CENTER)
        .expect("set_text_alignment");
    format
        .set_paragraph_alignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)
        .expect("set_paragraph_alignment");
}

/// 範囲外の列挙値はラッパーが E_INVALIDARG をそのまま伝播する。
#[test]
fn set_text_alignment_propagates_invalid_arg_error() {
    let factory = make_factory();
    let format = make_format(&factory);
    let result = format.set_text_alignment(DWRITE_TEXT_ALIGNMENT(9999));
    assert!(result.is_err(), "invalid alignment should fail");
}

// ============================================================
// create_text_layout: 手書き wcslen ループの境界
// ============================================================

/// null PCWSTR は明示的な空文字列分岐（`text_pcwstr.is_null()`）を通り、
/// 空レイアウトとして成功する。
#[test]
fn create_text_layout_with_null_text_yields_empty_layout() {
    let layout = make_layout(PCWSTR::null());
    assert_eq!(layout.get_cluster_count().expect("count"), 0);
    assert!(layout.get_cluster_metrics().expect("metrics").is_empty());
}

/// 空文字列リテラル（非 null、即終端）は wcslen ループが 0 を返す経路。
#[test]
fn create_text_layout_with_empty_literal_yields_empty_layout() {
    let layout = make_layout(w!(""));
    assert_eq!(layout.get_cluster_count().expect("count"), 0);
}

/// ASCII 5 文字 → 5 クラスタ（各クラスタ長 1）。
#[test]
fn ascii_text_produces_one_cluster_per_char() {
    let layout = make_layout(w!("Hello"));
    assert_eq!(layout.get_cluster_count().expect("count"), 5);

    let metrics = layout.get_cluster_metrics().expect("metrics");
    assert_eq!(metrics.len(), 5);
    for m in &metrics {
        assert_eq!(m.length, 1, "each ASCII char is one UTF-16 unit");
        assert!(m.width > 0.0, "visible glyph should have positive width");
    }
}

/// 日本語 3 文字 → 3 クラスタ（wcslen が UTF-16 単位で正しく数える）。
#[test]
fn japanese_text_produces_one_cluster_per_character() {
    let layout = make_layout(w!("日本語"));
    assert_eq!(layout.get_cluster_count().expect("count"), 3);
}

/// サロゲートペア（U+20BB7 𠮷）は UTF-16 2 単位で 1 クラスタ。
/// wcslen ループが文字単位でなく UTF-16 単位で走査することの特性化。
#[test]
fn surrogate_pair_is_single_cluster_of_two_utf16_units() {
    let layout = make_layout(w!("𠮷"));
    let metrics = layout.get_cluster_metrics().expect("metrics");
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].length, 2, "surrogate pair occupies 2 UTF-16 units");
}

// ============================================================
// get_cluster_count / get_cluster_metrics の整合
// ============================================================

#[test]
fn cluster_metrics_len_matches_cluster_count() {
    let layout = make_layout(w!("abc あいう"));
    let count = layout.get_cluster_count().expect("count");
    let metrics = layout.get_cluster_metrics().expect("metrics");
    assert_eq!(metrics.len(), count as usize);
}

// ============================================================
// hit_test_text_position
// ============================================================

#[test]
fn hit_test_text_position_returns_leading_and_trailing_points() {
    let layout = make_layout(w!("AB"));

    let leading = layout
        .hit_test_text_position(0, false)
        .expect("leading hit test");
    let trailing = layout
        .hit_test_text_position(0, true)
        .expect("trailing hit test");

    assert!(leading.point_x >= 0.0);
    assert!(leading.point_y >= 0.0);
    assert_eq!(leading.metrics.textPosition, 0);
    assert!(leading.metrics.length >= 1);
    // 同一文字の trailing は leading より右（LTR テキスト）
    assert!(
        trailing.point_x > leading.point_x,
        "trailing ({}) should be right of leading ({})",
        trailing.point_x,
        leading.point_x
    );
}

/// 範囲外テキスト位置はエラーにならず末尾へクランプされる（DirectWrite 仕様の特性化）。
#[test]
fn hit_test_text_position_clamps_out_of_range_position() {
    let layout = make_layout(w!("AB"));
    let result = layout.hit_test_text_position(999, false);
    assert!(result.is_ok(), "out-of-range position should clamp, not fail");
}
