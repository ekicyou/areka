//! 表示されている字の矩形（[`glyph_cells`]）の檻（task 13・要件 9.4）。
//!
//! 箱の文字の面の当たり判定は、この矩形の集まりで決まる。字 1 字ずつの「送りの幅 × 行の高さ」で、
//! 字の形の画素ではない。行の高さ（ブロック軸）は行矩形と選択肢の帯（`line_bands`）の和で、
//! 選択肢の当たり行・強調の帯がはみ出す分も受ける（要件 8.3・R3.3 の単一導出）。面の左上を原点と
//! する物理 px（`(絶対 − 領域の原点) × k`、ブロック軸に面反映済みのスクロール `committed` を足す）。

use super::*;
use crate::layout::{FixedMetrics, LayoutEngine, LineRect, PositionedGlyph, WrapPlan};
use crate::look::StyleId;
use crate::state::{ChoiceSpan, TextItem};
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

/// validrect 原点 (left, top) を持つ横書きの `TextRegion`。
fn region(left: i32, top: i32, right: i32, bottom: i32) -> TextRegion {
    region_in(WritingMode::HorizontalTb, left, top, right, bottom)
}

/// 書字方向 `mode` で解いた `TextRegion`。
fn region_in(mode: WritingMode, left: i32, top: i32, right: i32, bottom: i32) -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(top), Some(bottom), Some(left), Some(right)),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    TextRegion::resolve(&model, (400, 224), mode)
}

/// 行矩形（絶対 image px）と、(行内軸の位置, 送りの幅) の字の列を持つ行。
fn line(rect: (f32, f32, f32, f32), glyphs: &[(f32, f32)]) -> PositionedLine {
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

fn cell(left: f32, top: f32, right: f32, bottom: f32) -> HitRectPx {
    HitRectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// 横書き: 行内軸＝x（送りの幅）・ブロック軸＝y（行の高さ）。面の左上（領域の原点）を引いて × k。
#[test]
fn horizontal_cells_are_advance_by_line_height_from_surface_origin() {
    let region = region(36, 46, 356, 168);
    let lines = [
        line((36.0, 46.0, 356.0, 66.0), &[(36.0, 10.0), (46.0, 12.0)]),
        line((36.0, 66.0, 356.0, 86.0), &[(36.0, 10.0)]),
    ];
    let contract = ScaleContract::new(2.0, None);
    assert_eq!(
        glyph_cells(
            &lines,
            &[],
            WritingMode::HorizontalTb,
            &region,
            0,
            &contract
        ),
        vec![
            cell(0.0, 0.0, 20.0, 40.0),
            cell(20.0, 0.0, 44.0, 40.0),
            cell(0.0, 40.0, 20.0, 80.0),
        ]
    );
}

/// 横書きのスクロール（面反映済みの物理 px）はブロック軸（y）にだけ足す。
#[test]
fn horizontal_committed_scroll_moves_only_the_block_axis() {
    let region = region(36, 46, 356, 168);
    let lines = [line((36.0, 46.0, 356.0, 66.0), &[(46.0, 10.0)])];
    let contract = ScaleContract::new(2.0, None);
    assert_eq!(
        glyph_cells(
            &lines,
            &[],
            WritingMode::HorizontalTb,
            &region,
            -30,
            &contract
        ),
        vec![cell(20.0, -30.0, 40.0, 10.0)]
    );
}

/// 縦書き（rl・lr）: 行内軸＝y（送りの幅）・ブロック軸＝x（行の幅）。スクロールは x にだけ足す。
#[test]
fn vertical_cells_put_advance_on_y_and_line_extent_on_x() {
    let region = region(36, 46, 356, 168);
    let lines = [line(
        (300.0, 46.0, 320.0, 168.0),
        &[(46.0, 10.0), (56.0, 10.0)],
    )];
    let contract = ScaleContract::new(2.0, None);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        assert_eq!(
            glyph_cells(&lines, &[], mode, &region, 50, &contract),
            vec![
                cell(578.0, 0.0, 618.0, 20.0),  // (300−36)×2+50 … (320−36)×2+50
                cell(578.0, 20.0, 618.0, 40.0), // y: (56−46)×2 … (66−46)×2
            ],
            "{mode:?}"
        );
    }
}

/// 送りの幅が 0 の字は矩形を持たない（面積 0 の矩形を出さない）。行の無い（`\c`・台詞の頭・
/// 文字の無い）場所は矩形が 0 個。
#[test]
fn zero_advance_glyphs_and_empty_lines_yield_no_cells() {
    let region = region(0, 0, 100, 100);
    let contract = ScaleContract::new(1.0, None);
    let lines = [line((0.0, 0.0, 100.0, 10.0), &[(0.0, 0.0)])];
    assert_eq!(
        glyph_cells(
            &lines,
            &[],
            WritingMode::HorizontalTb,
            &region,
            0,
            &contract
        ),
        vec![]
    );
    assert_eq!(
        glyph_cells(&[], &[], WritingMode::HorizontalTb, &region, 0, &contract),
        vec![]
    );
}

/// 見えている字（表示の進み具合）だけが矩形になる: 配置の入口へ見えている数を渡した結果から作る。
/// 4 字のうち 2 字が見えていれば 2 個・0 字なら 0 個。
#[test]
fn only_revealed_glyphs_have_cells() {
    let region = region(0, 0, 200, 100);
    let contract = ScaleContract::new(1.0, None);
    let items: Vec<TextItem> = std::iter::repeat_n(TextItem::glyph("あ"), 4).collect();
    for (visible, expected) in [(4, 4), (2, 2), (0, 0)] {
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let cells = glyph_cells(
            &lines,
            &[],
            WritingMode::HorizontalTb,
            &region,
            0,
            &contract,
        );
        assert_eq!(cells.len(), expected, "見えている字 {visible}");
        assert!(
            cells
                .iter()
                .all(|c| c.right > c.left && c.bottom > c.top && c.left >= 0.0 && c.top >= 0.0),
            "どの矩形も面の中で面積を持つ: {cells:?}"
        );
    }
}

/// 帯（`offset`＋`extent`）が行矩形の遠い辺を越える行では、字の矩形のブロック軸は帯の遠い辺まで
/// 伸びる（近い辺は行矩形の近い辺）。帯が行矩形に収まる行は行矩形のまま。横書きと縦書きの両方。
#[test]
fn block_axis_is_union_of_line_rect_and_band() {
    let region = region(36, 46, 356, 168);
    let contract = ScaleContract::new(2.0, None);
    // 1 行目: em 20・帯 3..25（遠い辺 71 > 66）。2 行目: 帯 0..10（行矩形に収まる）。
    let bands = [
        LineBand {
            extent: 22.0,
            offset: 3.0,
        },
        LineBand {
            extent: 10.0,
            offset: 0.0,
        },
    ];
    let horizontal = [
        line((36.0, 46.0, 356.0, 66.0), &[(36.0, 10.0)]),
        line((36.0, 66.0, 356.0, 86.0), &[(36.0, 10.0)]),
    ];
    assert_eq!(
        glyph_cells(
            &horizontal,
            &bands,
            WritingMode::HorizontalTb,
            &region,
            0,
            &contract
        ),
        vec![
            cell(0.0, 0.0, 20.0, 50.0),  // y: (46−46)×2 … (46+3+22−46)×2
            cell(0.0, 40.0, 20.0, 80.0), // 帯が収まる行は行矩形のまま
        ]
    );
    let vertical = [
        line((300.0, 46.0, 320.0, 168.0), &[(46.0, 10.0)]),
        line((280.0, 46.0, 300.0, 168.0), &[(46.0, 10.0)]),
    ];
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        assert_eq!(
            glyph_cells(&vertical, &bands, mode, &region, 50, &contract),
            vec![
                cell(578.0, 0.0, 628.0, 20.0), // x: (300−36)×2+50 … (300+3+22−36)×2+50
                cell(538.0, 0.0, 578.0, 20.0), // 帯が収まる列は行矩形のまま
            ],
            "{mode:?}"
        );
    }
}

/// 選択肢の当たり行（`derive_hit_rows` → `to_window_physical`）から面の装着位置（領域の原点 × k）を
/// 引いた四角は、どの点も字の矩形のどれかに入る（絵が透明な所でも選択肢の行の全部でクリックを受ける・
/// 要件 8.3・9.4）。帯が em を越える計測（`FixedMetrics` の 28: 行ボックス 1.33 倍）で、横書きと
/// 縦書きの両方を、表示と同じ配置・同じ帯の列から確かめる。
#[test]
fn every_choice_hit_row_lies_inside_the_glyph_cells() {
    let items = vec![
        TextItem::glyph("は"),
        TextItem::glyph("い"),
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::glyph("い"),
        TextItem::glyph("い"),
        TextItem::glyph("え"),
    ];
    let span = |ordinal, glyph_range| ChoiceSpan {
        ordinal,
        id: String::new(),
        label: String::new(),
        references: vec![],
        glyph_range,
    };
    let spans = [span(0, 0..2), span(1, 2..5)];
    let (k, committed) = (2.0f32, 6);
    let contract = ScaleContract::new(k, None);
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let region = region_in(mode, 10, 20, 390, 220);
        let lines = LayoutEngine::layout(
            &items,
            5,
            &region,
            mode,
            28.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let bands = line_bands(&lines, mode, &FixedMetrics);
        // 前提: 帯の遠い辺が em（行矩形）を越える（越えなければこの檻は何も確かめない）。
        assert!(
            lines.iter().zip(&bands).all(|(l, b)| {
                let em = match mode {
                    WritingMode::HorizontalTb => l.rect.bottom - l.rect.top,
                    _ => l.rect.right - l.rect.left,
                };
                b.offset + b.extent > em
            }),
            "{mode:?}: 前提の帯が em を越えていない: {bands:?}"
        );
        let cells = glyph_cells(&lines, &bands, mode, &region, committed, &contract);
        let rows = derive_hit_rows(
            &lines,
            &annotate_lines(&lines, &spans),
            mode,
            &region,
            &bands,
        );
        assert_eq!(rows.len(), 2, "{mode:?}: 選択肢 2 行");
        let (mx, my) = (region.left() * k, region.top() * k);
        for row in &rows {
            let w = to_window_physical(row, &region, mode, committed, &contract, (0.0, 0.0));
            let r = cell(w.left - mx, w.top - my, w.right - mx, w.bottom - my);
            let mut y = r.top;
            while y < r.bottom {
                let mut x = r.left;
                while x < r.right {
                    assert!(
                        cells
                            .iter()
                            .any(|c| x >= c.left && x < c.right && y >= c.top && y < c.bottom),
                        "{mode:?}: 選択肢 {} の当たり行の点 ({x},{y}) が字の矩形の外: 行 {r:?}",
                        row.ordinal
                    );
                    x += 0.5;
                }
                y += 0.5;
            }
        }
    }
}
