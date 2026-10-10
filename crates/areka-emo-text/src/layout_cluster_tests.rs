//! 配置の単位が書記素クラスタであることを固定するテスト（areka-P0-balloon-color-emoji タスク 3.2・
//! 要件 3.4・3.5・4.1〜4.4・4.7・6.2・6.3・6.8）。
//!
//! 6 形（家族の ZWJ 列・国旗・肌色つき・異体字セレクタつき・キーキャップ・結合文字つき）は、
//! どれも部品のスカラー値が 2 つ以上あるのに、配置では 1 つの字として扱われなければならない:
//! 送り幅はフォント高 1 つ分・行の切れ目はクラスタの境目だけ・選択肢の範囲と `\_l` の基点は
//! クラスタ 1 つ分。どの判定も横書き・縦書き 2 方式の 3 方式で回す。
//!
//! 共通前提: `FixedMetrics`・文字高さ 10（全角 1 つ＝10・ASCII だけのクラスタ＝5）・
//! `origin` は未宣言（行内の開始は 3 方式とも 0）・validrect は画像全域。折り返しの基準は
//! 横書き＝wordwrappoint の x・縦書き＝y で与える。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

use super::test_support::{IMAGE, inline_positions, model};
use super::{FixedMetrics, GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::choice::{annotate_lines, derive_hit_rows, line_bands};
use crate::region::TextRegion;
use crate::segment::{Segment, SegmentPlan};
use crate::state::{ChoiceSpan, CursorCoord, CursorUnit, SpanKind, TextItem};
use crate::writing::WritingMode;

/// 共通前提の文字高さ。
const FONT: f32 = 10.0;

/// 3 方式。
const MODES: [WritingMode; 3] = [
    WritingMode::HorizontalTb,
    WritingMode::VerticalRl,
    WritingMode::VerticalLr,
];

/// 要件 2.2 の 6 形。
const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // 👨 ZWJ 👩 ZWJ 👧
const FLAG_JP: &str = "\u{1F1EF}\u{1F1F5}"; // 🇯 🇵（地域表示記号の対）
const THUMBS_LIGHT: &str = "\u{1F44D}\u{1F3FB}"; // 👍 ＋肌色の修飾 🏻
const HEART_VS16: &str = "\u{2764}\u{FE0F}"; // ❤ ＋異体字セレクタ U+FE0F
const KEYCAP_ONE: &str = "\u{0031}\u{FE0F}\u{20E3}"; // 1 ＋U+FE0F ＋囲みキーキャップ U+20E3
const KA_SEMIVOICED: &str = "\u{304B}\u{309A}"; // か ＋結合半濁点 U+309A
const FORMS: [&str; 6] = [
    FAMILY,
    FLAG_JP,
    THUMBS_LIGHT,
    HEART_VS16,
    KEYCAP_ONE,
    KA_SEMIVOICED,
];

/// 文字列の列をグリフのアイテム列にする（1 要素＝クラスタ 1 つ）。
fn glyph_items(texts: &[&str]) -> Vec<TextItem> {
    texts.iter().map(|t| TextItem::glyph(t)).collect()
}

/// 折り返しの基準を行内軸に `wrap` だけ置いた描画範囲（`None`＝基準なし）。
fn region_for(mode: WritingMode, wrap: Option<i32>) -> TextRegion {
    let wordwrap = match mode {
        WritingMode::HorizontalTb => (wrap, None),
        WritingMode::VerticalRl | WritingMode::VerticalLr => (None, wrap),
    };
    TextRegion::resolve(&model((None, None), wordwrap), IMAGE, mode)
}

/// 全グリフを見せて配置する。
fn lay(
    items: &[TextItem],
    mode: WritingMode,
    wrap: Option<i32>,
    plan: WrapPlan<'_>,
) -> Vec<PositionedLine> {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    LayoutEngine::layout(
        items,
        visible,
        &region_for(mode, wrap),
        mode,
        FONT,
        &FixedMetrics,
        plan,
    )
}

/// 行ごとのグリフの文字列。
fn line_texts(lines: &[PositionedLine]) -> Vec<Vec<&str>> {
    lines
        .iter()
        .map(|l| l.glyphs.iter().map(|g| &*g.text).collect())
        .collect()
}

/// 塊を 1 つだけ持つ手組みの分かち書きの計画（`start`・`len` はアイテムの序数）。
fn one_chunk(start: usize, len: usize) -> SegmentPlan {
    SegmentPlan::from_segments(vec![Segment { start, len }])
}

/// 行内軸の `\_l`（横書き＝`\_l[@N,]`・縦書き＝`\_l[,@N]`）。
fn cursor_inline_relative(mode: WritingMode, value: f32) -> TextItem {
    let rel = CursorCoord::Relative {
        value,
        unit: CursorUnit::Px,
    };
    match mode {
        WritingMode::HorizontalTb => TextItem::CursorMove {
            x: rel,
            y: CursorCoord::Omitted,
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: rel,
        },
    }
}

/// 前提: 6 形はどれも部品のスカラー値が 2 つ以上ある（1 つなら以降の判定は何も弁別しない）。
#[test]
fn every_form_has_more_than_one_scalar_value() {
    for form in FORMS {
        assert!(form.chars().count() >= 2, "{form:?} は複数のスカラー値");
        assert_eq!(areka_sakura::cluster::cluster_count(form), 1, "{form:?}");
    }
}

/// 要件 3.5: 縮退の見積もりでは 6 形の送り幅がそれぞれフォント高 1 つ分（部品の数で増えない）。
/// キーキャップは ASCII の「1」で始まるが、クラスタ全体は ASCII だけではないので全角の幅。
/// 配置でも 3 方式で 0, 10, …, 50 に並ぶ。
#[test]
fn fixed_metrics_gives_each_form_one_font_height() {
    assert_eq!(FixedMetrics.advance("a", FONT), 5.0, "対照: ASCII は半角");
    assert_eq!(FixedMetrics.advance("あ", FONT), 10.0, "対照: 全角");
    for form in FORMS {
        assert_eq!(FixedMetrics.advance(form, FONT), FONT, "{form:?}");
    }
    let items = glyph_items(&FORMS);
    for mode in MODES {
        let lines = lay(&items, mode, None, WrapPlan::CharByChar);
        assert_eq!(line_texts(&lines), vec![FORMS.to_vec()], "{mode:?}");
        assert_eq!(
            inline_positions(&lines[0]),
            vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0],
            "{mode:?}"
        );
        assert!(
            lines[0].glyphs.iter().all(|g| g.advance == FONT),
            "{mode:?}"
        );
    }
}

/// 要件 4.1・4.7: 文字ごとの折り返しの狭い行（基準 25）で、行の切れ目はクラスタの境目だけ。
/// 半角の「a」（5）と 6 形（10）を交互に並べ、各行の先頭が必ずアイテムの先頭になる:
/// 行 0 = a(0) 家族(5) a(15)／国旗は 20+10 > 25 で次行、以降同じ。
#[test]
fn char_by_char_narrow_lines_break_only_between_clusters() {
    let texts = [
        "a",
        FAMILY,
        "a",
        FLAG_JP,
        "a",
        THUMBS_LIGHT,
        "a",
        HEART_VS16,
        "a",
        KEYCAP_ONE,
        "a",
        KA_SEMIVOICED,
    ];
    let items = glyph_items(&texts);
    for mode in MODES {
        let lines = lay(&items, mode, Some(25), WrapPlan::CharByChar);
        assert_eq!(
            line_texts(&lines),
            vec![
                vec!["a", FAMILY, "a"],
                vec![FLAG_JP, "a", THUMBS_LIGHT],
                vec!["a", HEART_VS16, "a"],
                vec![KEYCAP_ONE, "a", KA_SEMIVOICED],
            ],
            "{mode:?}"
        );
        assert_eq!(
            inline_positions(&lines[0]),
            vec![0.0, 5.0, 15.0],
            "{mode:?}"
        );
        assert_eq!(
            inline_positions(&lines[1]),
            vec![0.0, 10.0, 15.0],
            "{mode:?}"
        );
    }
}

/// 要件 4.2・4.7: 手組みの分かち書きで 6 形を 1 つの塊にすると、合計 60 は行幅 25 を超えるので
/// 文字の規則へ戻る。戻っても切れ目はクラスタの境目だけで、文字ごとの折り返しと同じ行になる。
#[test]
fn overlong_chunk_falls_back_to_char_rule_without_splitting_a_cluster() {
    let items = glyph_items(&FORMS);
    let plan = one_chunk(0, FORMS.len());
    for mode in MODES {
        let seg = lay(&items, mode, Some(25), WrapPlan::Segmented(&plan));
        assert_eq!(
            line_texts(&seg),
            vec![
                vec![FAMILY, FLAG_JP],
                vec![THUMBS_LIGHT, HEART_VS16],
                vec![KEYCAP_ONE, KA_SEMIVOICED],
            ],
            "{mode:?}"
        );
        let chars = lay(&items, mode, Some(25), WrapPlan::CharByChar);
        assert_eq!(seg, chars, "{mode:?}: 縮退は文字の規則と同じ");
    }
}

/// 要件 4.3・4.7: 行幅（基準 5）より広いクラスタは割られず、1 行に 1 つずつ行頭へ置かれる。
/// 文字ごとの折り返しでも、塊が長すぎて文字の規則へ戻る分かち書きでも同じ。
#[test]
fn cluster_wider_than_the_line_is_placed_whole_on_its_own_line() {
    let items = glyph_items(&FORMS);
    let plan = one_chunk(0, FORMS.len());
    for mode in MODES {
        for wrap in [WrapPlan::CharByChar, WrapPlan::Segmented(&plan)] {
            let lines = lay(&items, mode, Some(5), wrap);
            let expected: Vec<Vec<&str>> = FORMS.iter().map(|f| vec![*f]).collect();
            assert_eq!(line_texts(&lines), expected, "{mode:?} {wrap:?}");
            for line in &lines {
                assert_eq!(line.glyphs[0].inline_pos, 0.0, "{mode:?} {wrap:?}");
                assert_eq!(line.glyphs[0].advance, FONT, "{mode:?} {wrap:?}");
            }
        }
    }
}

/// 要件 4.3・4.7（絶対上限の側）: 折返し基準は広く（300）、描画範囲の行内軸の遠辺だけを
/// 狭く（5）したバルーンでも、6 形は割られず 1 行に 1 つずつ行頭へ置かれる。
/// 描画範囲の組み方は `layout_hard_limit_tests.rs` に倣う（開始点 (0,0) を宣言・行内軸と
/// 直交する側は広く取る）。前提として 2 つの値が意図どおりに解決されていることを先に確かめる。
#[test]
fn cluster_wider_than_the_hard_limit_is_placed_whole_on_its_own_line() {
    let items = glyph_items(&FORMS);
    let plan = one_chunk(0, FORMS.len());
    for mode in MODES {
        let (wordwrap, validrect) = match mode {
            WritingMode::HorizontalTb => {
                ((Some(300), None), (Some(0), Some(200), Some(0), Some(5)))
            }
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                ((None, Some(300)), (Some(0), Some(5), Some(0), Some(300)))
            }
        };
        let model = BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(Some(0), Some(0)),
            WordWrapPoint::new(wordwrap.0, wordwrap.1),
            ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
            None,
        );
        let region = TextRegion::resolve(&model, IMAGE, mode);
        assert_eq!(
            (region.wrap_threshold(), region.inline_limit()),
            (300.0, 5.0),
            "{mode:?}: 折返し基準は広く・遠辺だけが狭い"
        );
        for wrap in [WrapPlan::CharByChar, WrapPlan::Segmented(&plan)] {
            let lines = LayoutEngine::layout(
                &items,
                FORMS.len(),
                &region,
                mode,
                FONT,
                &FixedMetrics,
                wrap,
            );
            let expected: Vec<Vec<&str>> = FORMS.iter().map(|f| vec![*f]).collect();
            assert_eq!(line_texts(&lines), expected, "{mode:?} {wrap:?}");
            for line in &lines {
                assert_eq!(line.glyphs[0].inline_pos, 0.0, "{mode:?} {wrap:?}");
                assert_eq!(line.glyphs[0].advance, FONT, "{mode:?} {wrap:?}");
            }
        }
    }
}

/// 要件 4.4・4.7・6.3: 選択肢の範囲が絵文字 1 つを覆うとき、範囲も当たり範囲も行内軸の幅が
/// 送り幅 1 つ分（「あ」10 の後ろ・10〜20）。
#[test]
fn choice_over_one_emoji_is_one_advance_wide() {
    for mode in MODES {
        for form in FORMS {
            let items = glyph_items(&["あ", form, "い"]);
            let lines = lay(&items, mode, None, WrapPlan::CharByChar);
            let spans = [ChoiceSpan {
                kind: SpanKind::Choice,
                ordinal: 0,
                id: String::new(),
                label: String::new(),
                references: vec![],
                glyph_range: 1..2,
            }];
            let segs = annotate_lines(&lines, &spans);
            assert_eq!(segs.len(), 1, "{mode:?} {form:?}");
            let advance = lines[0].glyphs[1].advance;
            assert_eq!(advance, FONT, "{mode:?} {form:?}");
            assert_eq!(segs[0].inline_range, (10.0, 20.0), "{mode:?} {form:?}");

            let region = region_for(mode, None);
            let bands = line_bands(&lines, mode, &FixedMetrics);
            let rows = derive_hit_rows(&lines, &segs, mode, &region, &bands);
            assert_eq!(rows.len(), 1, "{mode:?} {form:?}");
            let r = rows[0].rect;
            let inline_width = match mode {
                WritingMode::HorizontalTb => r.right - r.left,
                WritingMode::VerticalRl | WritingMode::VerticalLr => r.bottom - r.top,
            };
            assert_eq!(inline_width, advance, "{mode:?} {form:?}: 当たり範囲の幅");
        }
    }
}

/// 要件 3.4・4.7: 絵文字の後ろの `\_l`（行内軸を今の位置から +5）は、基点が絵文字 1 つ分
/// （10）だけ進んだ位置なので、続く「あ」は 15 に着く（部品の合計分は進まない）。
#[test]
fn cursor_after_an_emoji_starts_one_advance_further() {
    for mode in MODES {
        for form in FORMS {
            let items = vec![
                TextItem::glyph(form),
                cursor_inline_relative(mode, 5.0),
                TextItem::glyph("あ"),
            ];
            let lines = lay(&items, mode, None, WrapPlan::CharByChar);
            let landed: Vec<f32> = lines
                .iter()
                .flat_map(|l| l.glyphs.iter())
                .filter(|g| &*g.text == "あ")
                .map(|g| g.inline_pos)
                .collect();
            assert_eq!(landed, vec![15.0], "{mode:?} {form:?}");
        }
    }
}
