//! `viewbox_draw_decoration.rs` の兄弟檻のうち、区間の顔（spec emo-text-dwrite-fidelity 檻 D1〜D3）と
//! 区間の行と 1 字の計測の一致（檻 F3・F4 後半）。
//!
//! `viewbox_draw_decoration_tests.rs` が 1,000 行を超えるので兄弟のファイルへ分けた（中身は移しただけ）。

use super::decoration::{RANGE_CALLS, style_runs};
use super::decoration_tests::{glyph, table_with};
use super::test_support::geo_model;
use crate::draw::ResolvedFont;
use crate::layout::PositionedGlyph;
use crate::look::{StyleId, StyleTable, TextLook};
use crate::writing::WritingMode;
use areka_sakura::cluster::clusters;

// ────────────────────────── 区間の顔（要件 3.2／4.1／4.3／4.6／4.10・檻 D1〜D3）
//
// 実 DirectWrite の TextLayout に偽の台帳（`FontCatalog::with_faces`）で焼き、範囲指定の結果を
// TextLayout から読み戻す。家族名・太さ・幅は TextLayout が持つ値をそのまま返すので、機械の
// フォントの有無に頼らない。

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH, DWRITE_FONT_STRETCH_CONDENSED,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_MEDIUM, IDWriteTextLayout,
};
use wintf::com::dwrite::{DWriteFactoryExt, dwrite_create_factory};

use super::decoration::apply_font_ranges;
use crate::draw::{DEFAULT_FONT_NAME, FontCatalog, FontFace};

/// 偽の GDI の表の顔（家族名「Yu Gothic」・500＝「Yu Gothic Medium」が指す顔）。
fn yu_gothic_medium() -> FontFace {
    FontFace {
        weight: DWRITE_FONT_WEIGHT_MEDIUM,
        ..FontFace::regular("Yu Gothic")
    }
}

/// 「あいう」の真ん中の 1 字だけを `look` の区間にした行へ、束縛書式（`pick` と同じ規則の顔＋
/// 既定の太字・斜体）から作った TextLayout に `apply_font_ranges` を焼く。戻り値は行と範囲指定の回数。
fn bake_middle_run(
    fonts: &FontCatalog,
    default: &TextLook,
    look: &TextLook,
) -> (IDWriteTextLayout, usize) {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let bound = fonts.face_or_default(&default.name);
    let format = crate::draw::try_create_format(
        &factory,
        &bound,
        default.bold,
        default.italic,
        default.height,
    )
    .expect("束縛書式");
    let layout = factory
        .create_text_layout(
            &windows::core::HSTRING::from("あいう"),
            &format,
            200.0,
            50.0,
        )
        .expect("TextLayout");
    let (styles, id) = table_with(look, default);
    let runs = style_runs(&[
        glyph("あ", StyleId::DEFAULT),
        glyph("い", id),
        glyph("う", StyleId::DEFAULT),
    ]);
    RANGE_CALLS.with(|c| c.set(0));
    apply_font_ranges(&layout, &runs, &styles, default, fonts).expect("apply_font_ranges");
    (layout, RANGE_CALLS.with(|c| c.get()))
}

/// 文字位置 `pos` の家族名・太さ・幅。
fn face_at(
    layout: &IDWriteTextLayout,
    pos: u32,
) -> (String, DWRITE_FONT_WEIGHT, DWRITE_FONT_STRETCH) {
    unsafe {
        let mut len = 0u32;
        layout
            .GetFontFamilyNameLength(pos, &mut len, None)
            .expect("GetFontFamilyNameLength");
        let mut buf = vec![0u16; len as usize + 1];
        layout
            .GetFontFamilyName(pos, &mut buf, None)
            .expect("GetFontFamilyName");
        let mut weight = DWRITE_FONT_WEIGHT(0);
        layout
            .GetFontWeight(pos, &mut weight, None)
            .expect("GetFontWeight");
        let mut stretch = DWRITE_FONT_STRETCH(0);
        layout
            .GetFontStretch(pos, &mut stretch, None)
            .expect("GetFontStretch");
        (
            String::from_utf16_lossy(&buf[..len as usize]),
            weight,
            stretch,
        )
    }
}

/// 既定の見た目（候補列だけ差し替える）。
fn default_named(names: &[&str]) -> TextLook {
    TextLook {
        name: names.iter().map(|n| n.to_string()).collect(),
        ..ResolvedFont::resolve(&geo_model(Some(10))).looks.default
    }
}

/// D1（3.2・4.6）: GDI の名前の区間は家族「Yu Gothic」・500。区間の外は束縛（ＭＳ ゴシック 400）のまま。
#[test]
fn a_gdi_named_run_draws_with_the_family_and_weight_of_its_face() {
    let fonts = FontCatalog::with_faces(&[], &[("Yu Gothic Medium", yu_gothic_medium())]);
    let default = default_named(&[DEFAULT_FONT_NAME]);
    let look = TextLook {
        name: vec!["Yu Gothic Medium".into()],
        ..default.clone()
    };
    let (layout, _) = bake_middle_run(&fonts, &default, &look);
    let (family, weight, _) = face_at(&layout, 1);
    assert_eq!(family, "Yu Gothic", "区間は GDI の名前が指す家族名");
    assert_eq!(weight.0, 500, "区間は名前の太さ（中太）");
    for pos in [0, 2] {
        let (family, weight, _) = face_at(&layout, pos);
        assert_eq!(
            (family.as_str(), weight.0),
            (DEFAULT_FONT_NAME, 400),
            "区間の外 {pos} は束縛のまま"
        );
    }
}

/// D2（4.3）: 束縛が中太・既定が太字の行で `\f[bold,0]` の区間は 500（Regular の 400 へ戻らない）。
/// 束縛が中太で区間が Regular の家族名なら 400（中太を受け継がない）。
#[test]
fn run_weights_follow_the_same_rule_as_the_bound_face() {
    let fonts =
        FontCatalog::with_faces(&["Yu Gothic"], &[("Yu Gothic Medium", yu_gothic_medium())]);

    let bold_default = TextLook {
        bold: true,
        ..default_named(&["Yu Gothic Medium"])
    };
    let unbold = TextLook {
        bold: false,
        ..bold_default.clone()
    };
    let (layout, _) = bake_middle_run(&fonts, &bold_default, &unbold);
    assert_eq!(face_at(&layout, 0).1.0, 700, "前提: 束縛は中太＋太字＝700");
    assert_eq!(face_at(&layout, 1).1.0, 500, "太字を切った区間は名前の太さ");

    let medium_default = default_named(&["Yu Gothic Medium"]);
    let regular = TextLook {
        name: vec!["Yu Gothic".into()],
        ..medium_default.clone()
    };
    let (layout, _) = bake_middle_run(&fonts, &medium_default, &regular);
    assert_eq!(face_at(&layout, 0).1.0, 500, "前提: 束縛は中太");
    let (family, weight, _) = face_at(&layout, 1);
    assert_eq!(family, "Yu Gothic");
    assert_eq!(
        weight.0, 400,
        "家族名の区間は Regular（束縛の中太を受け継がない）"
    );
}

/// D3（4.6・4.10）: 幅の違う GDI の顔は幅を焼く。全滅の区間名は束縛の顔のまま範囲指定 0 回。
#[test]
fn a_run_face_bakes_its_stretch_and_an_all_missing_run_stays_bound() {
    let condensed = FontFace {
        stretch: DWRITE_FONT_STRETCH_CONDENSED,
        ..FontFace::regular("Narrow Sans")
    };
    let fonts = FontCatalog::with_faces(&[], &[("Narrow Sans Condensed", condensed)]);
    let default = default_named(&[DEFAULT_FONT_NAME]);

    let narrow = TextLook {
        name: vec!["Narrow Sans Condensed".into()],
        ..default.clone()
    };
    let (layout, _) = bake_middle_run(&fonts, &default, &narrow);
    assert_eq!(
        face_at(&layout, 1).2,
        DWRITE_FONT_STRETCH_CONDENSED,
        "区間の顔の幅"
    );
    assert_ne!(
        face_at(&layout, 0).2,
        DWRITE_FONT_STRETCH_CONDENSED,
        "区間の外は束縛の幅"
    );

    let missing = TextLook {
        name: vec!["NoSuchFontXYZ".into()],
        ..default.clone()
    };
    let (layout, calls) = bake_middle_run(&fonts, &default, &missing);
    assert_eq!(
        calls, 0,
        "全滅の区間名は束縛の顔のまま（範囲指定を発行しない）"
    );
    assert_eq!(face_at(&layout, 1).0, DEFAULT_FONT_NAME);
}

// ────────────────────────── 区間の行と 1 字の計測の一致（spec emo-text-dwrite-fidelity 檻 F3・F4 後半・要件 4.1／4.10）
//
// 計測器と範囲指定に同じ台帳を渡し、束縛書式の行へ**本番の** `apply_font_ranges` で区間を焼いて、
// 区間を含む行の cluster の幅と `DWriteMetrics::advance_styled` の 1 字ずつの計測を突き合わせる。
// 描画と計測のどちらか片方だけが顔の規則から外れると赤（`draw_metrics_styled_tests.rs` の F2／F4
// 前半は試験用書式の顔そのものを見る）。

/// 束縛（`font`）の計測器と、`line` の各字を `looks` の見た目で焼いた行を作り、全字の cluster の幅と
/// 1 字の計測が一致することを確かめる。`fonts` は計測器と範囲指定の両方へ渡す（共有の台帳）。
fn assert_line_matches_measurement(
    fonts: std::rc::Rc<FontCatalog>,
    font: &ResolvedFont,
    line: &str,
    looks: &[&TextLook],
) {
    use crate::draw::DWriteMetrics;
    use crate::layout::GlyphMetrics;
    use crate::state::TextLayerConfig;
    use wintf::com::dwrite::DWriteTextLayoutExt;

    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let default = &font.looks.default;
    let metrics = DWriteMetrics::new_shared(
        &factory,
        font,
        WritingMode::HorizontalTb,
        &TextLayerConfig::default(),
        std::rc::Rc::clone(&fonts),
    )
    .expect("DWriteMetrics 生成");
    let format = crate::draw::try_create_format(
        &factory,
        &fonts.face_or_default(&default.name),
        default.bold,
        default.italic,
        default.height,
    )
    .expect("束縛書式");
    let layout = factory
        .create_text_layout(&windows::core::HSTRING::from(line), &format, 1000.0, 100.0)
        .expect("TextLayout");
    let mut table = StyleTable::default();
    let glyphs: Vec<PositionedGlyph> = clusters(line)
        .zip(looks)
        .map(|(ch, look)| glyph(ch, table.intern(look, default)))
        .collect();
    assert_eq!(glyphs.len(), line.chars().count(), "見た目は 1 字に 1 つ");
    apply_font_ranges(&layout, &style_runs(&glyphs), &table, default, &fonts)
        .expect("apply_font_ranges");

    let widths: Vec<f32> = layout
        .get_cluster_metrics()
        .expect("cluster metrics")
        .iter()
        .map(|c| c.width)
        .collect();
    assert_eq!(
        widths.len(),
        glyphs.len(),
        "検証テキストは 1 文字＝1 クラスタ"
    );
    for (i, ((ch, look), width)) in clusters(line).zip(looks).zip(&widths).enumerate() {
        assert_eq!(
            metrics.advance_styled(ch, look),
            *width,
            "{ch:?}（{i} 文字目）: 1 字の計測と区間を含む行の送り量が食い違う"
        );
    }
}

/// 游ゴシックがこの機械に無ければ赤で止める（中太の期待値が代替フォントで緑にならないように）。
fn require_yu_gothic() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    assert!(
        FontCatalog::new(&factory)
            .expect("FontCatalog::new")
            .family_for(&["Yu Gothic".to_owned()])
            .is_some(),
        "実フォント 游ゴシック（Yu Gothic）がこの機械に無い。本試験の期待値は当該フォントの\
         中太を前提にしているので、代替フォントへ縮退したまま緑にしない"
    );
}

/// F3（4.1）: 束縛が ＭＳ ゴシック の行で、GDI の名前（「Yu Gothic Medium」＝中太）の区間の英字の
/// cluster の幅が、同じ見た目の 1 字の計測と一致する。中太の英字は Regular と送り幅が違うので、
/// 太さが計測か描画の片方だけに効くと赤（実測: 片方 400 だと "i" が 3.146 と 3.193 で割れる）。
#[test]
fn f3_gdi_named_run_measures_the_same_latin_advances_as_the_line() {
    use areka_parsers::balloon::parse_str;

    require_yu_gothic();
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let fonts = std::rc::Rc::new(FontCatalog::new(&factory).expect("FontCatalog::new"));
    let font = ResolvedFont::resolve(&parse_str("font.height,12\r\n", None));
    let default = &font.looks.default;
    let medium = TextLook {
        name: vec!["Yu Gothic Medium".into()],
        ..default.clone()
    };
    assert_eq!(
        fonts.face_for(&medium.name).map(|f| f.weight.0),
        Some(500),
        "前提: GDI の名前は中太"
    );
    let mut looks = vec![default; 1];
    looks.extend([&medium; 9]);
    looks.push(default);
    assert_line_matches_measurement(fonts, &font, "あWikipediaう", &looks);
}

/// F4 後半（4.10）: 束縛が中太（偽の GDI の表）の行で、候補が全滅した区間と、全滅かつ太字の区間を
/// 含む行の cluster の幅が、同じ見た目の 1 字の計測と一致する（全滅は描画も計測も束縛の顔）。
///
/// 較正（実測 2026-10-29・取り消し済み）: `apply_font_ranges` で太字の全滅の区間の顔を
/// `face_or_default(&look.name)`（＝ＭＳ ゴシック）にすると赤。
#[test]
fn f4_all_missing_runs_measure_the_same_advances_as_the_line() {
    use areka_parsers::balloon::parse_str;

    require_yu_gothic();
    let fonts = std::rc::Rc::new(FontCatalog::with_faces(
        &[],
        &[("Yu Gothic Medium", yu_gothic_medium())],
    ));
    let font = ResolvedFont::resolve(&parse_str("font.name,Yu Gothic Medium\r\n", None));
    let default = &font.looks.default;
    let missing = TextLook {
        name: vec!["NoSuchFontXYZ".into()],
        ..default.clone()
    };
    let bold = TextLook {
        bold: true,
        ..missing.clone()
    };
    let mut looks = vec![default; 1];
    looks.extend([&missing; 4]);
    looks.extend([&bold; 5]);
    looks.push(default);
    assert_line_matches_measurement(fonts, &font, "aWikipediab", &looks);
}
