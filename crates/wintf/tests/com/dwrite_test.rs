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
    let _format = make_format(&factory);
}

// ============================================================
// create_text_layout: 手書き wcslen ループの境界
// ============================================================

/// null PCWSTR は明示的な空文字列分岐（`text_pcwstr.is_null()`）を通り、
/// 空レイアウトとして成功する。
#[test]
fn create_text_layout_with_null_text_yields_empty_layout() {
    let layout = make_layout(PCWSTR::null());
    assert!(layout.get_cluster_metrics().expect("metrics").is_empty());
}

/// 空文字列リテラル（非 null、即終端）は wcslen ループが 0 を返す経路。
#[test]
fn create_text_layout_with_empty_literal_yields_empty_layout() {
    let layout = make_layout(w!(""));
    assert!(layout.get_cluster_metrics().expect("metrics").is_empty());
}

/// ASCII 5 文字 → 5 クラスタ（各クラスタ長 1）。
#[test]
fn ascii_text_produces_one_cluster_per_char() {
    let layout = make_layout(w!("Hello"));
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
    assert_eq!(layout.get_cluster_metrics().expect("metrics").len(), 3);
}

/// サロゲートペア（U+20BB7 𠮷）は UTF-16 2 単位で 1 クラスタ。
/// wcslen ループが文字単位でなく UTF-16 単位で走査することの特性化。
#[test]
fn surrogate_pair_is_single_cluster_of_two_utf16_units() {
    let layout = make_layout(w!("𠮷"));
    let metrics = layout.get_cluster_metrics().expect("metrics");
    assert_eq!(metrics.len(), 1);
    assert_eq!(
        metrics[0].length, 2,
        "surrogate pair occupies 2 UTF-16 units"
    );
}

// ============================================================
// get_overhang_metrics（レイアウトボックス基準のインクはみ出し）
// ============================================================

/// get_overhang_metrics はレイアウトボックス各辺からのインクはみ出し（正＝外側・負＝内側）を返す。
/// **ボックス基準**ゆえ、巨大ボックス（`make_layout` 既定 1000×1000）では対応辺のはみ出しが大きな
/// 負値になる（インクは遥か内側）——タイトなインク境界を得るにはボックス寸の確定が要る、という
/// 落とし穴（doc）を特性化する。呼び出し自体はエラーにならず有限値を返すことも固定する。
#[test]
fn get_overhang_metrics_is_box_relative_and_finite() {
    let layout = make_layout(w!("日本語"));
    let o = layout.get_overhang_metrics().expect("get_overhang_metrics");
    assert!(
        o.right < 0.0,
        "巨大 max_width では右辺はみ出しは負（ink はボックス内側）: right={}",
        o.right
    );
    assert!(
        o.bottom < 0.0,
        "巨大 max_height では下辺はみ出しは負: bottom={}",
        o.bottom
    );
    for v in [o.left, o.top, o.right, o.bottom] {
        assert!(v.is_finite(), "overhang 各成分は有限: {v}");
    }
}
