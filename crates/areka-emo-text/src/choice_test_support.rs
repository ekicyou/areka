//! 選択肢のテストの共有の部品（`choice_*_tests.rs` が引く・写しを 1 つに）。

use super::LineChoiceSegment;
use crate::layout::test_support::model_rect;
use crate::layout::{LineRect, PositionedGlyph, PositionedLine};
use crate::look::StyleId;
use crate::region::TextRegion;
use crate::state::{ChoiceSpan, SpanKind, TextItem};
use crate::writing::WritingMode;

/// 帯の寄せ＝0（行ボックス丈 ＝ 帯の丈のフォント＝既定 ＭＳ ゴシックと同じ関係）。
/// 帯は行矩形の block 近端に揃い、寄せを入れる前と完全一致する（非退行）。
pub(crate) const NO_OFFSET: f32 = 0.0;

/// validrect 原点 (left, top) を持つ横書きの `TextRegion`（画像原寸 400×224）。
pub(crate) fn region(left: i32, top: i32, right: i32, bottom: i32) -> TextRegion {
    region_in(WritingMode::HorizontalTb, left, top, right, bottom)
}

/// 書字方向 `mode` で解いた `TextRegion`（画像原寸 400×224）。
pub(crate) fn region_in(
    mode: WritingMode,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> TextRegion {
    let model = model_rect(
        (None, None),
        (Some(top), Some(bottom), Some(left), Some(right)),
    );
    TextRegion::resolve(&model, (400, 224), mode)
}

/// 行矩形（絶対 image px）と、(行内軸の位置, 送りの幅) の字の列を持つ行。
pub(crate) fn line(rect: (f32, f32, f32, f32), glyphs: &[(f32, f32)]) -> PositionedLine {
    PositionedLine {
        rect: LineRect {
            left: rect.0,
            top: rect.1,
            right: rect.2,
            bottom: rect.3,
        },
        glyphs: glyphs
            .iter()
            .map(|&(inline_pos, advance)| PositionedGlyph {
                text: "あ".into(),
                inline_pos,
                advance,
                style: StyleId::DEFAULT,
            })
            .collect(),
    }
}

/// `(line_index, ordinal, inline_range)` の [`LineChoiceSegment`]。
pub(crate) fn seg(line_index: usize, ordinal: usize, range: (f32, f32)) -> LineChoiceSegment {
    LineChoiceSegment {
        line_index,
        ordinal,
        inline_range: range,
    }
}

/// glyph_range だけを指定した選択肢の [`ChoiceSpan`]（id/label/references は最小値）。
pub(crate) fn span(ordinal: usize, glyph_range: core::ops::Range<usize>) -> ChoiceSpan {
    ChoiceSpan {
        kind: SpanKind::Choice,
        ordinal,
        id: String::new(),
        label: String::new(),
        references: vec![],
        glyph_range,
    }
}

/// 「は い ⏎ い い え」の字の列と、「はい」「いいえ」の選択肢 2 つ。
pub(crate) fn yes_no() -> (Vec<TextItem>, [ChoiceSpan; 2]) {
    let items = vec![
        TextItem::glyph("は"),
        TextItem::glyph("い"),
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::glyph("い"),
        TextItem::glyph("い"),
        TextItem::glyph("え"),
    ];
    (items, [span(0, 0..2), span(1, 2..5)])
}
