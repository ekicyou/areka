use super::*;
use crate::layout::{LineRect, PositionedGlyph, PositionedLine};
use crate::state::ChoiceSpan;

/// 行内軸位置 `positions` の各グリフ（送り幅 `adv`）を持つ横書き行を作る
/// （行矩形は本注釈で未使用ゆえ最小値を入れる）。
fn line(positions: &[(f32, f32)]) -> PositionedLine {
    let glyphs = positions
        .iter()
        .map(|&(inline_pos, advance)| PositionedGlyph {
            ch: 'あ',
            inline_pos,
            advance,
        })
        .collect();
    PositionedLine {
        rect: LineRect {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        },
        glyphs,
    }
}

/// glyph_range だけを指定した ChoiceSpan（id/label/references は注釈非関与ゆえ最小値）。
fn span(ordinal: usize, range: core::ops::Range<usize>) -> ChoiceSpan {
    ChoiceSpan {
        ordinal,
        id: String::new(),
        label: String::new(),
        references: vec![],
        glyph_range: range,
    }
}

/// 単一行・単一スパン → 行内範囲が「先頭グリフ位置〜末尾グリフ位置＋送り幅」の 1 セグメント。
#[test]
fn single_line_single_span_yields_one_segment_with_inline_range() {
    let lines = [line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)])];
    let spans = [span(0, 0..3)];
    let segs = annotate_lines(&lines, &spans);
    assert_eq!(
        segs,
        vec![LineChoiceSegment {
            line_index: 0,
            ordinal: 0,
            inline_range: (0.0, 30.0), // 先頭 0.0 〜 末尾 20.0 + advance 10.0
        }]
    );
}

/// 半角/全角混在の送り幅でも inline_range は先頭位置〜末尾位置＋末尾送り幅で決まる。
#[test]
fn single_span_inline_range_uses_first_pos_and_last_pos_plus_advance() {
    // 'a'(6) 'あ'(12) 'b'(6): 位置 0,6,18。範囲 0..2 は先頭 0 〜 6+12=18。
    let lines = [line(&[(0.0, 6.0), (6.0, 12.0), (18.0, 6.0)])];
    let spans = [span(0, 0..2)];
    let segs = annotate_lines(&lines, &spans);
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].inline_range, (0.0, 18.0));
}

/// 同一行に複数スパン（正典 `\q\q` 並置）→ スパン順に 2 セグメント（それぞれの行内範囲）。
#[test]
fn two_spans_on_one_line_yield_two_segments() {
    let lines = [line(&[
        (0.0, 10.0),
        (10.0, 10.0),
        (20.0, 10.0),
        (30.0, 10.0),
    ])];
    let spans = [span(0, 0..2), span(1, 2..4)];
    let segs = annotate_lines(&lines, &spans);
    assert_eq!(
        segs,
        vec![
            LineChoiceSegment {
                line_index: 0,
                ordinal: 0,
                inline_range: (0.0, 20.0),
            },
            LineChoiceSegment {
                line_index: 0,
                ordinal: 1,
                inline_range: (20.0, 40.0),
            },
        ]
    );
}

/// Observable: 2 行に折り返された 3 スパン入力 → 行別分割のセグメント。
/// span1 が折返し境界（line0 末尾グリフ 2 と line1 先頭グリフ 3）を跨ぎ、行ごとに分割される。
#[test]
fn three_spans_folded_across_two_lines_split_per_line() {
    // line0: 序数 0,1,2（位置 0,10,20）・line1: 序数 3,4,5（位置 0,10,20）。
    let lines = [
        line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)]),
        line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)]),
    ];
    let spans = [
        span(0, 0..2), // line0 内（序数 0,1）
        span(1, 2..4), // line0 末尾(2) ＋ line1 先頭(3) を跨ぐ
        span(2, 4..6), // line1 内（序数 4,5）
    ];
    let segs = annotate_lines(&lines, &spans);
    assert_eq!(
        segs,
        vec![
            // span0: line0 のみ。
            LineChoiceSegment {
                line_index: 0,
                ordinal: 0,
                inline_range: (0.0, 20.0),
            },
            // span1: 折返し跨ぎ → line0 と line1 の 2 セグメントへ分割。
            LineChoiceSegment {
                line_index: 0,
                ordinal: 1,
                inline_range: (20.0, 30.0), // line0 末尾グリフ（序数 2）: 位置 20 〜 20+10
            },
            LineChoiceSegment {
                line_index: 1,
                ordinal: 1,
                inline_range: (0.0, 10.0), // line1 先頭グリフ（序数 3）: 位置 0 〜 0+10
            },
            // span2: line1 のみ。
            LineChoiceSegment {
                line_index: 1,
                ordinal: 2,
                inline_range: (10.0, 30.0), // line1 グリフ（序数 4,5）: 位置 10 〜 20+10
            },
        ]
    );
}

/// 空範囲スパン（空 text 選択肢）はセグメントを生まない（非退行・design.md Invariants）。
#[test]
fn empty_range_span_produces_no_segment() {
    let lines = [line(&[(0.0, 10.0), (10.0, 10.0)])];
    // start == end（空範囲）。glyph_range が行内でも行末でも生まない。
    let spans = [span(0, 1..1), span(1, 2..2)];
    let segs = annotate_lines(&lines, &spans);
    assert!(
        segs.is_empty(),
        "空範囲スパンはセグメントを生まない: {segs:?}"
    );
}

/// 部分リビール: 配置済みグリフ（可視 prefix）がスパン末尾より短いとき、
/// 交差は配置済みグリフ数で打ち切られる（min(glyph_range.end, visible_count)）。
#[test]
fn partial_reveal_truncates_span_to_placed_glyphs() {
    // スパン範囲 0..5 だが配置済み（可視）は 3 グリフのみ（1 行）。
    let lines = [line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)])];
    let spans = [span(0, 0..5)];
    let segs = annotate_lines(&lines, &spans);
    assert_eq!(
        segs,
        vec![LineChoiceSegment {
            line_index: 0,
            ordinal: 0,
            inline_range: (0.0, 30.0), // 配置済み末尾グリフ（序数 2）まで: 位置 20 + 10
        }],
        "可視 prefix より後ろは打ち切られる"
    );
}

/// 部分リビールが折返し跨ぎと合わさる場合: 可視が line1 の途中で切れると、
/// line1 セグメントは配置済みグリフまで（超過分は line1 に存在しない＝交差せず）。
#[test]
fn partial_reveal_across_wrap_truncates_second_line() {
    // line0: 序数 0,1（配置 2）・line1: 序数 2,3（配置 2）。総配置 4。
    let lines = [
        line(&[(0.0, 10.0), (10.0, 10.0)]),
        line(&[(0.0, 10.0), (10.0, 10.0)]),
    ];
    // スパン範囲 1..6（line0 の 1、line1 の 2,3、そして未配置の 4,5 を含む指定）。
    let spans = [span(0, 1..6)];
    let segs = annotate_lines(&lines, &spans);
    assert_eq!(
        segs,
        vec![
            LineChoiceSegment {
                line_index: 0,
                ordinal: 0,
                inline_range: (10.0, 20.0), // line0 序数 1: 位置 10 + 10
            },
            LineChoiceSegment {
                line_index: 1,
                ordinal: 0,
                inline_range: (0.0, 20.0), // line1 序数 2,3: 位置 0 〜 10+10（未配置 4,5 は打切り）
            },
        ]
    );
}

/// 空の lines・空の spans は空セグメント（純粋・恒等）。
#[test]
fn empty_inputs_yield_empty_segments() {
    assert!(annotate_lines(&[], &[span(0, 0..2)]).is_empty());
    assert!(annotate_lines(&[line(&[(0.0, 10.0)])], &[]).is_empty());
    assert!(annotate_lines(&[], &[]).is_empty());
}

// ── タスク 5.2: ヒット行導出（derive_hit_rows）と窓物理写像（to_window_physical） ──
//
// 座標系: layout 出力（inline_range／行矩形）は絶対 image px（描画開始点＝validrect 原点起点）。
// CanvasHitRow.rect は canvas-local（validrect-local）——from_layout と同一の原点差引きゆえ
// ハイライト矩形と数値一致（正典確定「ハイライト矩形＝ヒット矩形と同一」・R3.3）。
// 窓物理化は to_window_physical: 行内=(origin+inline)×k・ブロック=(origin+block)×k+committed。

use crate::region::{ScaleContract, TextRegion};
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

/// validrect 原点を持つ `TextRegion`（非負 validrect 素通し・derive/写像は left()/top() のみ使用）。
fn region(left: i32, top: i32, right: i32, bottom: i32) -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(top), Some(bottom), Some(left), Some(right)),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    TextRegion::resolve(&model, (400, 224), WritingMode::HorizontalTb)
}

/// 行矩形だけ指定した `PositionedLine`（derive_hit_rows は行矩形の block 帯のみ使用）。
fn prow(left: f32, top: f32, right: f32, bottom: f32) -> PositionedLine {
    PositionedLine {
        rect: LineRect {
            left,
            top,
            right,
            bottom,
        },
        glyphs: vec![],
    }
}

/// `(line_index, ordinal, inline_range)` の [`LineChoiceSegment`]。
fn seg(line_index: usize, ordinal: usize, range: (f32, f32)) -> LineChoiceSegment {
    LineChoiceSegment {
        line_index,
        ordinal,
        inline_range: range,
    }
}

/// 既存ケースのハイライト帯＝行矩形の em ボックス丈（font 10）——`band_extent == font_height`
/// のとき従来（行矩形 block 帯）と完全一致することを既存期待値がそのまま檻にする（非退行）。
const HIT_BAND: f32 = 10.0;

// ── derive_hit_rows: canvas-local ヒット矩形（文字幅・block 帯・原点差引き） ──

/// 横書き・原点 (0,0): ヒット矩形の行内範囲＝セグメントの inline_range（**文字幅**）・
/// ブロック帯＝行矩形の top..bottom。行全幅（0..50）ではなくセグメント範囲（10..30）で切る。
#[test]
fn derive_horizontal_uses_char_width_inline_and_line_block_band() {
    let region = region(0, 0, 400, 224);
    // 行全幅 0..50・block 帯 0..10（font 10）。
    let lines = [prow(0.0, 0.0, 50.0, 10.0)];
    let segs = [seg(0, 7, (10.0, 30.0))]; // 選択肢グリフ範囲（文字幅）＝10..30。
    let rows = derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND);
    assert_eq!(
        rows,
        vec![CanvasHitRow {
            ordinal: 7,
            rect: LineRect {
                left: 10.0, // 行全幅 0..50 ではなくセグメント 10..30（文字幅）。
                top: 0.0,
                right: 30.0,
                bottom: 10.0,
            },
        }]
    );
}

/// 横書き・非零 validrect 原点 (36,46): from_layout と同一の原点差引きで canvas-local 化する
/// （絶対 image px の行矩形/セグメントから validrect 原点を引く＝ハイライト矩形と同座標系）。
#[test]
fn derive_horizontal_subtracts_validrect_origin_to_canvas_local() {
    let region = region(36, 46, 356, 168);
    // 絶対 image px: 行矩形 left36/top46/right86/bottom56・セグメント 46..66。
    let lines = [prow(36.0, 46.0, 86.0, 56.0)];
    let segs = [seg(0, 0, (46.0, 66.0))];
    let rows = derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND);
    assert_eq!(
        rows[0].rect,
        LineRect {
            left: 10.0,   // 46 − 36
            top: 0.0,     // 46 − 46
            right: 30.0,  // 66 − 36
            bottom: 10.0, // 56 − 46
        },
        "絶対 image px から validrect 原点を差し引いた canvas-local"
    );
}

/// 縦書き（vertical_rl / vertical_lr）: 行内軸＝y（inline_range）・ブロック軸＝x（行矩形の
/// left..right 帯）。軸割当が横書きと入れ替わる（正準表）。
#[test]
fn derive_vertical_assigns_inline_to_y_and_block_to_x() {
    let region = region(0, 0, 400, 224);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        // 縦書き列矩形: x 帯 377..387（block・font 10）・y 全長 0..20。
        let lines = [prow(377.0, 0.0, 387.0, 20.0)];
        let segs = [seg(0, 1, (5.0, 15.0))]; // 行内軸（y）の文字幅範囲。
        let rows = derive_hit_rows(&lines, &segs, mode, &region, HIT_BAND);
        assert_eq!(
            rows[0].rect,
            LineRect {
                left: 377.0, // block 帯＝行矩形 left（x）
                top: 5.0,    // inline＝セグメント i0（y）
                right: 387.0,
                bottom: 15.0,
            },
            "{mode:?}: 行内=y・ブロック=x の軸割当"
        );
    }
}

/// 複数セグメント（同一行 2 個＋別行 1 個）→ 入力順どおり 3 行、それぞれ ordinal と矩形を保持。
#[test]
fn derive_multiple_segments_yield_rows_in_input_order() {
    let region = region(0, 0, 400, 224);
    let lines = [prow(0.0, 0.0, 40.0, 10.0), prow(0.0, 13.0, 30.0, 23.0)];
    let segs = [
        seg(0, 0, (0.0, 20.0)),
        seg(0, 1, (20.0, 40.0)),
        seg(1, 2, (0.0, 30.0)),
    ];
    let rows = derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].ordinal, 0);
    assert_eq!(
        rows[0].rect,
        LineRect {
            left: 0.0,
            top: 0.0,
            right: 20.0,
            bottom: 10.0
        }
    );
    assert_eq!(rows[1].ordinal, 1);
    assert_eq!(
        rows[1].rect,
        LineRect {
            left: 20.0,
            top: 0.0,
            right: 40.0,
            bottom: 10.0
        }
    );
    assert_eq!(rows[2].ordinal, 2);
    assert_eq!(
        rows[2].rect,
        LineRect {
            left: 0.0,
            top: 13.0,
            right: 30.0,
            bottom: 23.0
        }
    );
}

/// 空セグメント列 → 空のヒット行（非退行）。
#[test]
fn derive_empty_segments_yield_no_rows() {
    let region = region(0, 0, 400, 224);
    let lines = [prow(0.0, 0.0, 30.0, 10.0)];
    assert!(derive_hit_rows(&lines, &[], WritingMode::HorizontalTb, &region, HIT_BAND).is_empty());
}

/// 空範囲セグメント（`i0 >= i1`）は行を生まない（防御・annotate 既除外）。
#[test]
fn derive_empty_range_segment_produces_no_row() {
    let region = region(0, 0, 400, 224);
    let lines = [prow(0.0, 0.0, 30.0, 10.0)];
    let segs = [seg(0, 0, (10.0, 10.0)), seg(0, 1, (20.0, 15.0))];
    assert!(
        derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND).is_empty(),
        "空/逆順範囲はヒット行を生まない"
    );
}

/// `line_index` 範囲外セグメントは防御的にスキップ（annotate は有効添字のみ出す）。
#[test]
fn derive_out_of_range_line_index_is_skipped() {
    let region = region(0, 0, 400, 224);
    let lines = [prow(0.0, 0.0, 30.0, 10.0)];
    let segs = [seg(5, 0, (0.0, 10.0))];
    assert!(
        derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND).is_empty()
    );
}

// ── to_window_physical: §座標写像式（行内=(origin+inline)×k・ブロック=(origin+block)×k+committed） ──

/// canvas-local (10,0,30,10)・原点 (36,46)。横書きは committed をブロック軸（y=top/bottom）へ。
#[test]
fn to_window_physical_horizontal_applies_formula() {
    let region = region(36, 46, 356, 168);
    let row = CanvasHitRow {
        ordinal: 0,
        rect: LineRect {
            left: 10.0,
            top: 0.0,
            right: 30.0,
            bottom: 10.0,
        },
    };
    // k=2・committed=50: 行内 x=(36+inline)×2・ブロック y=(46+block)×2+50。
    let contract = ScaleContract::new(2.0, None);
    assert_eq!(
        to_window_physical(&row, &region, WritingMode::HorizontalTb, 50, &contract),
        HitRectPx {
            left: 92.0,    // (36+10)×2
            top: 142.0,    // (46+0)×2 + 50
            right: 132.0,  // (36+30)×2
            bottom: 162.0  // (46+10)×2 + 50
        }
    );
}

/// 縦書き（rl/lr）は committed をブロック軸（x=left/right）へ・行内軸（y）は ×k のみ。
#[test]
fn to_window_physical_vertical_puts_committed_on_x_block_axis() {
    let region = region(36, 46, 356, 168);
    let row = CanvasHitRow {
        ordinal: 0,
        rect: LineRect {
            left: 10.0,
            top: 0.0,
            right: 30.0,
            bottom: 10.0,
        },
    };
    let contract = ScaleContract::new(2.0, None);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        assert_eq!(
            to_window_physical(&row, &region, mode, 50, &contract),
            HitRectPx {
                left: 142.0,   // (36+10)×2 + 50（x=ブロック）
                top: 92.0,     // (46+0)×2（y=行内・committed 非加算）
                right: 182.0,  // (36+30)×2 + 50
                bottom: 112.0  // (46+10)×2
            },
            "{mode:?}: committed は x（ブロック軸）のみ"
        );
    }
}

/// k∈{1.0,2.0}×committed∈{0,50}×3 方向の全網羅パラメタライズ（独立算出の期待値と一致）。
#[test]
fn to_window_physical_parameterized_over_k_committed_and_modes() {
    let region = region(36, 46, 356, 168);
    let (ox, oy) = (36.0f32, 46.0f32);
    let row = CanvasHitRow {
        ordinal: 0,
        rect: LineRect {
            left: 10.0,
            top: 2.0,
            right: 30.0,
            bottom: 12.0,
        },
    };
    let r = row.rect;
    for k in [1.0f32, 2.0] {
        let contract = ScaleContract::new(k, None);
        for c in [0i32, 50] {
            let cf = c as f32;
            for mode in [
                WritingMode::HorizontalTb,
                WritingMode::VerticalRl,
                WritingMode::VerticalLr,
            ] {
                let got = to_window_physical(&row, &region, mode, c, &contract);
                let expected = match mode {
                    // horizontal: x=行内（×k）・y=ブロック（×k+committed）。
                    WritingMode::HorizontalTb => HitRectPx {
                        left: (ox + r.left) * k,
                        top: (oy + r.top) * k + cf,
                        right: (ox + r.right) * k,
                        bottom: (oy + r.bottom) * k + cf,
                    },
                    // vertical: x=ブロック（×k+committed）・y=行内（×k）。
                    WritingMode::VerticalRl | WritingMode::VerticalLr => HitRectPx {
                        left: (ox + r.left) * k + cf,
                        top: (oy + r.top) * k,
                        right: (ox + r.right) * k + cf,
                        bottom: (oy + r.bottom) * k,
                    },
                };
                assert_eq!(got, expected, "k={k} committed={c} mode={mode:?}");
            }
        }
    }
}

/// 現行契約 k=1.0・committed=0（非スクロール）: 窓物理 px は絶対 image px と一致
/// （canvas-local + 原点 = 絶対）——2 空間モデルの恒等（R2.2 の DPI=96 経路）。
#[test]
fn to_window_physical_unit_scale_no_scroll_equals_absolute_image_px() {
    let region = region(36, 46, 356, 168);
    let row = CanvasHitRow {
        ordinal: 0,
        rect: LineRect {
            left: 10.0,
            top: 0.0,
            right: 30.0,
            bottom: 10.0,
        },
    };
    let contract = ScaleContract::new(1.0, None);
    assert_eq!(
        to_window_physical(&row, &region, WritingMode::HorizontalTb, 0, &contract),
        HitRectPx {
            left: 46.0,
            top: 46.0,
            right: 66.0,
            bottom: 56.0
        } // = 絶対 image px
    );
}

// ── R3.3: 描画とヒットの単一導出（derive_hit_rows と from_layout の canvas-local が一致） ──

/// Observable: 固定入力に対し、derive_hit_rows の canvas-local 矩形は、ハイライト描画が
/// 同一 `LineChoiceSegment`＋行矩形から `from_layout` と同一の原点差引きで組む矩形と
/// **完全一致**する（表示とヒットが同一導出パス＝座標整合の構造保証）。
#[test]
fn hit_row_rect_matches_canvas_local_highlight_derivation() {
    let region = region(36, 46, 356, 168);
    let line = prow(36.0, 46.0, 86.0, 56.0); // 絶対 image px の行矩形。
    let segment = seg(0, 3, (46.0, 66.0)); // 絶対 image px のセグメント inline 範囲。
    let rows = derive_hit_rows(
        std::slice::from_ref(&line),
        std::slice::from_ref(&segment),
        WritingMode::HorizontalTb,
        &region,
        HIT_BAND,
    );
    // ハイライト描画が使う canvas-local 矩形を from_layout と同一手順で独立算出:
    // 行矩形 block 近端（top）＋帯（HIT_BAND）＋セグメント inline 範囲（i0/i1）を validrect 原点差引き。
    let (ox, oy) = (region.left(), region.top());
    let (i0, i1) = segment.inline_range;
    let expected_highlight = LineRect {
        left: i0 - ox,
        top: line.rect.top - oy,
        right: i1 - ox,
        bottom: line.rect.top + HIT_BAND - oy,
    };
    assert_eq!(
        rows[0].rect, expected_highlight,
        "ヒット矩形とハイライト矩形は同一 canvas-local 座標（単一導出）"
    );
}

// ── descent 込みハイライト帯（highlight_band_extent／derive_hit_rows の帯適用） ──
//
// 実機不具合（実 fixture Yu Gothic UI 28px で hover 文字の下が帯からはみ出す）の真因は
// 「帯＝em ボックス丈（font_height）」だった。帯は実 font metrics の行ボックス丈
// （ascent+descent）を源にし、行送りピッチで頭打ちにする。

/// 行ボックス丈が em ボックスとピッチの間にあるとき、帯は**行ボックス丈そのもの**
/// （descent を覆う。ＭＳ ゴシック系＝比 1.0 は em と一致し従来どおり）。
#[test]
fn band_extent_takes_line_box_height_between_em_and_pitch() {
    // font 28・行ボックス 32.0・ピッチ 35 → 32.0（行ボックス丈が採られる）。
    assert_eq!(highlight_band_extent(28.0, 32.0, 35.0), 32.0);
    // 比 1.0 のフォント（ＭＳ ゴシック実測）は em ボックス丈と一致＝従来挙動（非退行）。
    assert_eq!(highlight_band_extent(28.0, 28.0, 35.0), 28.0);
}

/// 行ボックス丈が行送りピッチを超えるときはピッチで頭打ち（隣接行の帯／ヒット矩形と
/// 重ならせない）。実測 Yu Gothic UI 28px（行ボックス 37.242・ピッチ 35）＝35。
#[test]
fn band_extent_is_capped_by_line_pitch() {
    assert_eq!(highlight_band_extent(28.0, 37.242, 35.0), 35.0);
    // 隣接行の帯は接するだけで重ならない（行送り 35 ＋ 帯 35）。
    assert!(highlight_band_extent(28.0, 37.242, 35.0) <= 35.0);
}

/// 下限は em ボックス丈——行ボックス丈がそれより小さくても行矩形より痩せない（非退行）。
/// ピッチが em を下回る病的設定でも下限が勝つ。
#[test]
fn band_extent_never_below_font_height() {
    assert_eq!(highlight_band_extent(28.0, 20.0, 35.0), 28.0);
    assert_eq!(highlight_band_extent(28.0, 37.0, 20.0), 28.0);
}

/// Observable（本不具合の純粋層側の檻）: 帯が em ボックス丈を超えるとき、ヒット矩形の
/// ブロック軸下端は**行矩形の bottom（em ボックス下端）より下**へ伸びる——描画側
/// `highlight_rect`（住人原点＋band_extent）と同一の式ゆえ数値一致する（R3.3）。
#[test]
fn hit_row_block_band_extends_beyond_em_box_when_band_is_larger() {
    let region = region(36, 46, 356, 168);
    // 実 fixture 相当: 行矩形 block 帯 46..74（font 28）・帯 35（Yu Gothic UI 実測の頭打ち値）。
    let line = prow(36.0, 46.0, 200.0, 74.0);
    let segment = seg(0, 0, (36.0, 100.0));
    let band = highlight_band_extent(28.0, 37.242, 35.0);
    let rows = derive_hit_rows(
        std::slice::from_ref(&line),
        std::slice::from_ref(&segment),
        WritingMode::HorizontalTb,
        &region,
        band,
    );
    assert_eq!(
        rows[0].rect.top, 0.0,
        "帯の起点は行矩形 block 近端（46−46）"
    );
    assert_eq!(
        rows[0].rect.bottom, 35.0,
        "帯の終端は近端＋band_extent（em ボックス下端 28 ではない＝descent を覆う）"
    );
    assert!(
        rows[0].rect.bottom > line.rect.bottom - region.top(),
        "em ボックス丈（28）より下へ伸びる"
    );
}

/// 縦書きも同一規則: 帯は行矩形の block 近端（left）から band_extent 分（右方向）。
#[test]
fn hit_row_vertical_block_band_uses_band_extent_from_left_edge() {
    let region = region(0, 0, 400, 224);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let lines = [prow(377.0, 0.0, 387.0, 20.0)]; // block 帯 377..387（font 10）。
        let segs = [seg(0, 1, (5.0, 15.0))];
        let rows = derive_hit_rows(&lines, &segs, mode, &region, 13.0);
        assert_eq!(
            rows[0].rect,
            LineRect {
                left: 377.0,
                top: 5.0,
                right: 390.0,
                bottom: 15.0
            },
            "{mode:?}: block 帯は left から band_extent（13）分"
        );
    }
}
