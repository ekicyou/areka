//! [`DWriteMetrics`] の見た目込み計測（タスク 6.4・要件 7.10／11.1／11.3／15.5）。
//!
//! 送り幅は「文字」だけでなく「その文字に効く見た目」（候補列・大きさ・太字・斜体＝
//! `FontKey`）で決まる。既定の見た目は装飾を入れる前と同じ値のまま——本ファイルは
//! **装飾を入れる前に実測した値を字義の期待値**として置き、委譲先が同じだから同じ、
//! という恒真の突き合わせにしない。
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 既定の見た目の計測値が装飾導入前と同一 | 11.1, 11.3 |
//! | §2 | 大きさ・太字が計測値に効く | 7.10, 11.1 |
//! | §3 | 試験用書式は計測鍵ごとに 1 度だけ生成 | 11.3 |
//! | §4 | 計測値＝装飾入りで組んだ行の実際の文字送り量 | 11.3, 15.5 |
//! | §5 | 共有の台帳と束縛書式の解決経路 | 9.8, 11.3 |
//!
//! **較正**（要件 15.5・実測 2026-09-12・摂動はいずれも取り消し済み）——実装を次の
//! 4 通りに壊すと、それぞれ本ファイルの述語が赤になることを確かめてある。
//!
//! | 壊し方 | 赤になる述語 |
//! |---|---|
//! | 見た目を無視して常に束縛書式で測る | §2 §3 §4 §5（5 本） |
//! | 記憶の鍵を文字だけへ戻す | §2 §3 §5（3 本） |
//! | 鍵ごとの試験用書式を毎回作り直す | §3（1 本） |
//! | 家族名を候補列の解決でなく決め打ちにする | `draw_format_metrics_tests.rs` の家族名の檻 |
//!
//! 3 番目は**保持庫の要素数**では捕まらない（同じ鍵で作り直しても要素数は増えない）。
//! §3 が数えるのは実際の生成回数。

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_TEXT_RANGE, IDWriteTextLayout,
};
use windows::core::HSTRING;
use wintf::com::dwrite::{DWriteTextLayoutExt, dwrite_create_factory};

use super::super::test_support::{default_metrics, empty_font, model_with_font};
use super::super::{LineLayoutStore, ResolvedFont, create_text_format};
use super::DWriteMetrics;
use crate::layout::GlyphMetrics;
use crate::look::{StyleId, TextLook};
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

/// 装飾を入れる**前**（2026-09-12 実測）の既定の見た目——ＭＳ ゴシック 12——の送り幅。
/// 委譲の形が変わっても、この字義の値が動いたら退行。
const ADVANCE_FULLWIDTH_AT_12: f32 = 12.0;
/// 同上・半角（ASCII）。
const ADVANCE_ASCII_AT_12: f32 = 6.0;

/// 既定の見た目（ＭＳ ゴシック 12・装飾なし）。
fn default_look() -> TextLook {
    ResolvedFont::resolve(&model_with_font(empty_font()))
        .looks
        .default
}

/// 既定の見た目から 1 項目だけ変えた見た目。
fn look_with(f: impl FnOnce(&mut TextLook)) -> TextLook {
    let mut look = default_look();
    f(&mut look);
    look
}

/// 既定フォントの計測器（本番と同じ生成口）。
fn metrics() -> DWriteMetrics {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    default_metrics(&factory, WritingMode::HorizontalTb)
}

// ────────────────────────── §1 既定の見た目は装飾導入前と同一（R11.1／R11.3）

/// 既定の見た目の計測値が、装飾を入れる前に実測した値と同一。
#[test]
fn the_default_look_measures_exactly_what_it_measured_before_decorations_existed() {
    let m = metrics();
    let look = default_look();
    assert_eq!(
        m.advance_styled('あ', &look),
        ADVANCE_FULLWIDTH_AT_12,
        "既定の見た目の全角の送り幅が装飾導入前の実測値から動いた"
    );
    assert_eq!(
        m.advance_styled('A', &look),
        ADVANCE_ASCII_AT_12,
        "既定の見た目の半角の送り幅が装飾導入前の実測値から動いた"
    );
    // 束縛書式の経路（従来の入口）とも同値——2 つの入口が割れていない。
    assert_eq!(m.advance_styled('あ', &look), m.advance('あ', look.height));
}

// ────────────────────────── §2 大きさ・太字が計測値に効く（R7.10／R11.1）

/// 大きさを 2 倍にすると送り幅が増える。**既定を先に測ってから** styled を測る——
/// 記憶の鍵が文字だけへ戻ると、ここで既定の値が返ってきて赤くなる。
#[test]
fn doubling_the_size_widens_the_advance_even_after_the_default_was_measured_first() {
    let m = metrics();
    let base = m.advance_styled('あ', &default_look());
    assert_eq!(base, ADVANCE_FULLWIDTH_AT_12);

    let doubled = m.advance_styled(
        'あ',
        &look_with(|l| l.height = ADVANCE_FULLWIDTH_AT_12 * 2.0),
    );
    assert!(
        doubled > base,
        "大きさ 2 倍の送り幅 {doubled} が既定 {base} を超えていない（見た目が計測に効いていない）"
    );
    assert_eq!(
        doubled,
        ADVANCE_FULLWIDTH_AT_12 * 2.0,
        "等幅の全角は大きさに正比例する"
    );
}

/// 太字は送り幅を縮めない（同じ鍵の別項目が計測へ届いていることの対照）。
#[test]
fn bold_does_not_shrink_the_advance() {
    let m = metrics();
    let base = m.advance_styled('あ', &default_look());
    let bold = m.advance_styled('あ', &look_with(|l| l.bold = true));
    assert!(bold >= base, "太字の送り幅 {bold} が既定 {base} より狭い");
}

// ────────────────────────── §3 試験用書式は鍵ごとに 1 度だけ（R11.3）

/// 同じ計測鍵で 2 度目を測っても試験用書式は**生成されない**。鍵が違えば生成される。
///
/// 数えるのは保持庫の要素数ではなく**実際の生成回数**——同じ鍵で作り直しても要素数は
/// 増えないので、要素数では「鍵ごとに 1 度だけ」を見張れない（実測 2026-09-12）。
#[test]
fn a_probe_format_is_created_once_per_measurement_key() {
    let m = metrics();
    let big = look_with(|l| l.height = 24.0);

    assert_eq!(
        m.probe_format_creations(),
        0,
        "既定の見た目しか測っていない間は試験用書式を作らない（束縛書式を使う）"
    );
    m.advance_styled('あ', &default_look());
    assert_eq!(m.probe_format_creations(), 0);

    m.advance_styled('あ', &big);
    assert_eq!(
        m.probe_format_creations(),
        1,
        "鍵 1 つ目で 1 度だけ生成する"
    );
    m.advance_styled('あ', &big);
    m.advance_styled('い', &big);
    assert_eq!(
        m.probe_format_creations(),
        1,
        "同じ鍵の 2 度目・別の文字では試験用書式を作り直さない"
    );

    m.advance_styled('あ', &look_with(|l| l.bold = true));
    assert_eq!(m.probe_format_creations(), 2, "鍵が違えばもう 1 度作る");
}

/// 記憶の鍵は「文字と計測鍵」——同じ文字でも鍵が違えば別の記憶になる。
#[test]
fn the_measurement_cache_is_keyed_by_character_and_measurement_key() {
    let m = metrics();
    let big = look_with(|l| l.height = 24.0);

    m.advance_styled('あ', &default_look());
    assert_eq!(m.cached_probe_count(), 1);
    m.advance_styled('あ', &big);
    assert_eq!(
        m.cached_probe_count(),
        2,
        "同じ文字でも鍵が違えば別の記憶（鍵が文字だけなら 1 のまま＝赤）"
    );
    m.advance_styled('あ', &big);
    assert_eq!(m.cached_probe_count(), 2, "同じ組は測り直さない");
}

// ────────────────────────── §4 計測値＝装飾入りで組んだ行の送り量（R11.3／R15.5）

/// 後半 3 文字（UTF-16 で 1 文字 1 単位）へ焼く範囲。
const STYLED_RANGE: DWRITE_TEXT_RANGE = DWRITE_TEXT_RANGE {
    startPosition: 3,
    length: 3,
};

/// 行の前半は既定・後半は「2 倍＋太字＋斜体」で焼いた行を実際に組み、DirectWrite が
/// 返す文字ごとの送り量と、計測器の値が一致する（計算どうしの突き合わせにしない）。
#[test]
fn styled_measurements_match_the_cluster_advances_of_a_decorated_line() {
    const LINE: &str = "あいうかきく";

    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let plain = default_look();
    let styled = look_with(|l| {
        l.height = 24.0;
        l.bold = true;
        l.italic = true;
    });
    let m = default_metrics(&factory, WritingMode::HorizontalTb);
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let format = create_text_format(&factory, &resolved, WritingMode::HorizontalTb)
        .expect("既定フォントの TextFormat");

    let mut store = LineLayoutStore::new(&factory);
    let ids = [
        StyleId::DEFAULT,
        StyleId::DEFAULT,
        StyleId::DEFAULT,
        StyleId(1),
        StyleId(1),
        StyleId(1),
    ];
    let layout: IDWriteTextLayout = store
        .line_layout_decorated(
            0,
            LINE,
            &format,
            styled.height,
            WritingMode::HorizontalTb,
            &ids,
            |layout| {
                unsafe {
                    layout
                        .SetFontFamilyName(&HSTRING::from(styled.name[0].as_str()), STYLED_RANGE)
                        .expect("SetFontFamilyName");
                    layout
                        .SetFontSize(styled.height, STYLED_RANGE)
                        .expect("SetFontSize");
                    layout
                        .SetFontWeight(DWRITE_FONT_WEIGHT_BOLD, STYLED_RANGE)
                        .expect("SetFontWeight");
                    layout
                        .SetFontStyle(DWRITE_FONT_STYLE_ITALIC, STYLED_RANGE)
                        .expect("SetFontStyle");
                }
                Ok(())
            },
        )
        .expect("装飾入りの行が組める");

    let widths: Vec<f32> = layout
        .get_cluster_metrics()
        .expect("cluster metrics")
        .iter()
        .map(|c| c.width)
        .collect();
    assert_eq!(
        widths.len(),
        LINE.chars().count(),
        "検証テキストは 1 文字＝1 クラスタの前提"
    );

    for (i, (ch, width)) in LINE.chars().zip(&widths).enumerate() {
        let look = if i < 3 { &plain } else { &styled };
        assert_eq!(
            m.advance_styled(ch, look),
            *width,
            "{ch:?}（{i} 文字目・{}）: 計測値と実際に組んだ行の送り量が食い違う",
            if i < 3 { "既定" } else { "装飾" }
        );
    }
    // 対照: 前半と後半の送り量が実際に違う（全部が既定で焼かれていたら組が壊れている）。
    assert!(
        widths[3] > widths[0],
        "装飾側の送り量 {} が既定側 {} を超えていない＝行に装飾が焼かれていない",
        widths[3],
        widths[0]
    );
}

// ────────────────────────── §5 較正の対照（R15.5）

/// 束縛書式の生成が候補列の解決を通る（要件 9.8——バルーン定義の候補列にも効く）。
///
/// 振る舞いでは映らない（単一名のバルーン定義では解決しても同じ名前が返る）ので、
/// 経路そのものを字面で固定する。試験用書式の側の家族名の入口は
/// `draw_format_metrics_tests.rs` の家族名の檻が持つ。
#[test]
fn the_bound_format_is_built_from_the_catalog_resolved_family() {
    const SRC: &str = include_str!("draw_metrics.rs");
    assert!(
        SRC.contains("let picked = fonts.pick(font);"),
        "束縛書式が候補列の解決（FontCatalog::pick）を通っていない"
    );
}

/// 共有の台帳を受け取る構築口が、自前の台帳を作る構築口と同じ計測値を返す。
#[test]
fn the_shared_catalog_constructor_measures_the_same_as_the_owning_one() {
    use std::rc::Rc;

    use super::super::FontCatalog;

    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    let shared = Rc::new(FontCatalog::new(&factory).expect("FontCatalog::new"));
    let m = DWriteMetrics::new_shared(
        &factory,
        &resolved,
        WritingMode::HorizontalTb,
        &TextLayerConfig::default(),
        Rc::clone(&shared),
    )
    .expect("共有台帳で DWriteMetrics が作れる");

    assert_eq!(
        m.advance_styled('あ', &default_look()),
        ADVANCE_FULLWIDTH_AT_12
    );
    assert_eq!(
        m.advance_styled('あ', &look_with(|l| l.height = 24.0)),
        24.0
    );
    assert!(
        std::ptr::eq(m.fonts(), Rc::as_ptr(&shared)),
        "共有で渡した台帳がそのまま使われていない（警告の源が 2 つに割れる）"
    );
}
