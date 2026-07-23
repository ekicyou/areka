//! # choice — 選択肢固有の純粋導出の集約モジュール（純粋層）
//!
//! `Choice` cue が state 層へ載せた選択肢スパン（[`ChoiceSpan`]・items のグリフ序数範囲）を、
//! layout 層が配置した行列（[`PositionedLine`]）へ写し、行×選択肢の注釈
//! （[`LineChoiceSegment`]）を導出する純関数群を単独所有する。描画（装飾）とヒット照会は
//! この単一の注釈導出を源にすることで座標整合を構造保証する（design.md「ChoicePure」・R3.3）。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻・lib.rs 構造檻へ登録）。
//! 全関数は同一入力→同一出力の純関数で失敗経路を持たない（縮退は値で表現）。
//!
//! ## 序数空間の統一（部分リビール整合・design.md「Invariants」）
//!
//! [`ChoiceSpan::glyph_range`] は **items 全体のグリフ序数空間**（`Glyph` のみを数える序数）で
//! 表される。一方 [`annotate_lines`] が受け取る `lines` は **可視 prefix 適用済み**の配置済み
//! グリフ（layout 出力）——追記順の先頭から `visible_count` 個だけが `PositionedLine::glyphs`
//! に現れる。両者はともに「グリフのみ・追記順」で数えるため、配置済みグリフを行を跨いで
//! 0 起点の連番で数えた値が、そのまま items 全体のグリフ序数に一致する。したがって
//! スパン範囲と各行のカバー範囲を**純整数の累積和**で交差判定すれば、可視 prefix より後ろの
//! グリフは配置済み総数（＝`visible_count`）を超えないため交差が自然に打ち切られる
//! （`min(glyph_range.end, visible_count)` の実現・浮動小数比較を交差判定に用いない・R3.4）。
//!
//! ## 折返し跨ぎの行別分割（design.md「Risks」）
//!
//! 1 スパンのグリフ範囲が折返し境界を跨ぐ場合、跨いだ行ごとに 1 つの [`LineChoiceSegment`] を
//! 出す（`\q` は emo2 正典上は自動折返ししないが、構造的に正しく扱う）。

use crate::layout::PositionedLine;
use crate::state::ChoiceSpan;

/// 行×選択肢セグメント注釈（配置済み行へのスパン写像・折返し跨ぎは行ごと分割）。
///
/// `inline_range` は行内軸の image px 絶対範囲（先頭グリフの配置位置〜末尾グリフの配置位置＋
/// 送り幅）。行内軸は 3 方向とも正方向（横書き＝x・縦書き＝y）で、[`PositionedGlyph::inline_pos`]
/// が既にその軸の絶対値ゆえ本注釈は writing mode に依存しない（窓物理写像＝mode 依存の写像は
/// 後段 `to_window_physical` の責務）。
///
/// [`PositionedGlyph::inline_pos`]: crate::layout::PositionedGlyph::inline_pos
#[derive(Clone, Debug, PartialEq)]
pub struct LineChoiceSegment {
    /// セグメントが属する行の index（`lines` に対する添字）。
    pub line_index: usize,
    /// スパンの配送順序数（[`ChoiceSpan::ordinal`]・hover／照会の主キー）。
    pub ordinal: usize,
    /// 行内軸 image px 絶対範囲（先頭グリフ位置〜末尾グリフ位置＋送り幅）。
    pub inline_range: (f32, f32),
}

/// 注釈導出（純粋）: layout 出力の行グリフ列を序数走査してスパンを行セグメントへ写す。
///
/// 各スパンの [`ChoiceSpan::glyph_range`]（items 全体のグリフ序数空間）を、配置済みグリフを
/// 行を跨いで 0 起点で数えた累積和と純整数で交差させ、交差した行ごとに
/// [`LineChoiceSegment`] を 1 つ出す（折返し跨ぎは行別分割・design.md「Risks」）。
///
/// - 空範囲スパン（`start >= end`・空 `text` 選択肢）はセグメントを生まない（非退行）。
/// - 可視 prefix（部分リビール）の打切りは構造的に成立する: `lines` の配置済みグリフ総数は
///   `visible_count` ゆえ、スパン末尾がそれを超えても行カバー範囲との交差が
///   `min(glyph_range.end, visible_count)` で自然に切れる（浮動小数比較を交差判定に用いない）。
/// - 出力順はスパン順（＝ordinal 昇順）× 行昇順。
///
/// 同一入力→同一出力（純粋・決定論）。失敗経路なし（全入力で値を返す純関数）。
pub fn annotate_lines(lines: &[PositionedLine], spans: &[ChoiceSpan]) -> Vec<LineChoiceSegment> {
    let mut segments = Vec::new();
    for span in spans {
        let span_start = span.glyph_range.start;
        let span_end = span.glyph_range.end;
        // 空範囲スパン（空 text 選択肢）はセグメントを生まない（非退行・design.md Invariants）。
        if span_start >= span_end {
            continue;
        }
        // 配置済みグリフを行を跨いで 0 起点で数えた累積和（= items 全体のグリフ序数と一致）。
        let mut line_start = 0usize;
        for (line_index, line) in lines.iter().enumerate() {
            let line_end = line_start + line.glyphs.len();
            // 純整数の交差 [span_start, span_end) ∩ [line_start, line_end)。
            let lo = span_start.max(line_start);
            let hi = span_end.min(line_end);
            if lo < hi {
                // 行内ローカル添字へ変換（lo/hi は行を跨いだ通し序数）。
                let first = &line.glyphs[lo - line_start];
                let last = &line.glyphs[hi - 1 - line_start];
                segments.push(LineChoiceSegment {
                    line_index,
                    ordinal: span.ordinal,
                    inline_range: (first.inline_pos, last.inline_pos + last.advance),
                });
            }
            line_start = line_end;
        }
    }
    segments
}

#[cfg(test)]
mod tests {
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
        let lines = [line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0), (30.0, 10.0)])];
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
        assert!(segs.is_empty(), "空範囲スパンはセグメントを生まない: {segs:?}");
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
}
