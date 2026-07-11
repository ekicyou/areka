//! # layout — 折返し・行送りの決定（純粋層）
//!
//! `GlyphMetrics` trait（グリフ送り幅・行送りピッチの唯一の注入口・R4.5）を通じて
//! metrics 依存を外部化し、折返し位置・行送りの決定アルゴリズム自体は描画方式に
//! 依存しない純粋な形に保つ `LayoutEngine`／`FixedMetrics`／`PositionedLine` を担う。
//! あふれ判定・スクロール可視窓の決定は後続ユニット（同モジュールへ追加）の領分。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 実測 metrics（DWriteMetrics・probe TextLayout 由来）は COM 層（draw）が
//! [`GlyphMetrics`] を実装して注入する。
//!
//! ## 軸読み替え（design.md「軸読み替え正準表」R6.1–6.3）
//!
//! 3 方向は**単一の読み替え規則**で扱う——回るのは軸の役割だけで、
//! アルゴリズム分岐は存在しない:
//!
//! | 項目 | horizontal_tb | vertical_rl | vertical_lr |
//! |---|---|---|---|
//! | 行内軸（文字が進む） | +x | +y | +y |
//! | 行送り軸（行が進む） | +y | −x | +x |
//! | 折返し判定 | 行内位置＋次グリフ幅 > 閾値（3 方向共通・行内軸は常に正方向） | 同 | 同 |
//!
//! 折返し閾値・描画開始点は [`TextRegion`] が解決済みの絶対値（image px）。
//!
//! ## 行内開始位置の規則（design 無言域の実装正準）
//!
//! 折返し・改行後の行は、描画開始点（origin クランプ正準）の**行内軸成分**へ戻る
//! （全行で同一の行内開始＝単一規則。行送り軸成分だけが行ごとに進む）。
//!
//! ## 可視 prefix 規則（typewriter との接続）
//!
//! `layout` は追記順 items の先頭から「`visible_count`+1 個目のグリフ」直前までを
//! 配置対象とする。改行マーカーは prefix 内なら即時反映する（R2.2 の後出し優先・
//! 空行も [`PositionedLine`] として現れる）。リビール時刻の解決
//! （`visible_glyphs(actor, t)`）は state 層の領分で、本層は個数だけを受け取る。
//!
//! ## 行矩形の規約（R9.4 の再利用シーム）
//!
//! [`PositionedLine::rect`] は image px の絶対矩形。行内軸範囲＝行内開始〜最終グリフ
//! 送り終端（空行は零幅）・行送り軸範囲＝行位置から `font_height` 分（horizontal_tb
//! は下方向・vertical_rl は左方向・vertical_lr は右方向＝行送り方向と同符号）。
//! グリフ別の行内位置＋送り幅と併せ、choice-render のクリック可能範囲導出が
//! そのまま再利用できる（導出自体は実装しない・R9.4）。

use crate::region::TextRegion;
use crate::state::{TextItem, TextLayerConfig};
use crate::writing::WritingMode;

/// グリフ送りの注入点（metrics 依存の唯一の口・R4.5）。
///
/// 「グリフ送り幅・行送りピッチ」だけを注入し、折返し位置・行送りの決定
/// アルゴリズム自体は純粋に保つ分離線の正準。構造テストは [`FixedMetrics`]、
/// 実行時は COM 層の DWriteMetrics（測定専用 probe TextLayout 由来）を注入する。
/// 両者で折返し位置は異なってよいが、アルゴリズム分岐は存在しない。
pub trait GlyphMetrics {
    /// グリフの行内送り幅（image px）。writing_mode の行内軸方向の寸。
    fn advance(&self, ch: char, font_height: f32) -> f32;

    /// 行送りピッチ（image px）。M1 正準: `ceil(font_height × 1.25)`
    /// （係数は [`TextLayerConfig::line_pitch_factor`] が正本・既定 1.25）。
    fn line_pitch(&self, font_height: f32) -> f32;
}

/// 構造テスト用の決定論 metrics（R4.5/R11.6）。
///
/// 決定論仮想値: 全角（非 ASCII）＝`font_height`・半角（ASCII）＝`font_height / 2`。
/// 行送りピッチは M1 正準式 `ceil(font_height × 既定係数 1.25)`。
/// タイポグラフィ的正確さは目的でない——折返し・行送りアルゴリズムの檻のための値。
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedMetrics;

impl GlyphMetrics for FixedMetrics {
    fn advance(&self, ch: char, font_height: f32) -> f32 {
        if ch.is_ascii() {
            font_height / 2.0
        } else {
            font_height
        }
    }

    fn line_pitch(&self, font_height: f32) -> f32 {
        (font_height * TextLayerConfig::default().line_pitch_factor).ceil()
    }
}

/// 行の画像空間矩形（image px 絶対座標・R9.4 の再利用シーム）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineRect {
    /// 左辺（image px）。
    pub left: f32,
    /// 上辺（image px）。
    pub top: f32,
    /// 右辺（image px）。
    pub right: f32,
    /// 下辺（image px）。
    pub bottom: f32,
}

/// 配置済みグリフ（行内軸の絶対位置＋送り幅・クリック可能範囲導出の入力形・R9.4）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    /// グリフの文字。
    pub ch: char,
    /// 行内軸の配置位置（image px 絶対座標。horizontal_tb＝x・縦書き＝y）。
    pub inline_pos: f32,
    /// 行内送り幅（image px・注入 metrics 由来）。
    pub advance: f32,
}

/// 配置済みの 1 行（行矩形＋グリフ列・choice-render 再利用シーム・R9.4）。
#[derive(Clone, Debug, PartialEq)]
pub struct PositionedLine {
    /// 行の画像空間矩形（規約はモジュール doc「行矩形の規約」）。
    pub rect: LineRect,
    /// 行内のグリフ列（行内軸位置の昇順・空行は空列）。
    pub glyphs: Vec<PositionedGlyph>,
}

/// 折返し・行送りの決定エンジン（純粋・R4.5/R6.1–6.3）。
pub struct LayoutEngine;

impl LayoutEngine {
    /// 折返し・行送りを解決して行列（[`PositionedLine`] 列）を得る（純粋・決定論）。
    ///
    /// - `items`: 追記順の正本（state 層の `ActorTextState::items`）。
    /// - `visible_count`: 可視グリフ数（state 層 `visible_glyphs` の出力）。
    ///   可視 prefix 規則（モジュール doc）で配置対象を切る。
    /// - 折返し判定: `行内位置＋次グリフ幅 > 閾値`（3 方向共通・正準表）。
    ///   行頭の 1 グリフは閾値超過でも配置する（無限折返しの構造排除・無損失）。
    /// - 行送り量: 自動折返し＝`line_pitch`・改行マーカー＝`line_pitch × ratio`。
    ///
    /// 同一入力→同一出力（R2.5 系）。失敗経路なし（全入力で値を返す純関数）。
    pub fn layout(
        items: &[TextItem],
        visible_count: usize,
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
        metrics: &dyn GlyphMetrics,
    ) -> Vec<PositionedLine> {
        let pitch = metrics.line_pitch(font_height);
        let threshold = region.wrap_threshold();
        let start = region.start();
        // 軸読み替え正準表: 行内軸開始・行送り軸開始・行送り方向（±1）。
        // 行内軸は 3 方向とも正方向（+x／+y）＝折返し判定は共通式で回る。
        let (inline_start, block_start, block_dir) = match mode {
            WritingMode::HorizontalTb => (start.0, start.1, 1.0f32),
            WritingMode::VerticalRl => (start.1, start.0, -1.0f32),
            WritingMode::VerticalLr => (start.1, start.0, 1.0f32),
        };

        let mut lines: Vec<PositionedLine> = Vec::new();
        let mut current: Vec<PositionedGlyph> = Vec::new();
        let mut inline_pos = inline_start;
        let mut block_pos = block_start;
        let mut placed = 0usize;
        let mut opened = false;

        for item in items {
            match *item {
                TextItem::Glyph { ch } => {
                    // 可視 prefix の終端: visible_count+1 個目のグリフ直前で打ち切る。
                    if placed == visible_count {
                        break;
                    }
                    opened = true;
                    let advance = metrics.advance(ch, font_height);
                    // 折返し判定（正準表）: 行内位置＋次グリフ幅 > 閾値。
                    // 行頭グリフは閾値超過でも配置（縮退・グリフを落とさない）。
                    if !current.is_empty() && inline_pos + advance > threshold {
                        lines.push(finish_line(
                            std::mem::take(&mut current),
                            mode,
                            inline_start,
                            inline_pos,
                            block_pos,
                            font_height,
                        ));
                        block_pos += block_dir * pitch;
                        inline_pos = inline_start;
                    }
                    current.push(PositionedGlyph {
                        ch,
                        inline_pos,
                        advance,
                    });
                    inline_pos += advance;
                    placed += 1;
                }
                TextItem::LineBreak { ratio } => {
                    // 改行マーカーは prefix 内なら即時反映（行送り量 = pitch × ratio）。
                    opened = true;
                    lines.push(finish_line(
                        std::mem::take(&mut current),
                        mode,
                        inline_start,
                        inline_pos,
                        block_pos,
                        font_height,
                    ));
                    block_pos += block_dir * pitch * ratio;
                    inline_pos = inline_start;
                }
            }
        }
        if opened {
            lines.push(finish_line(
                current,
                mode,
                inline_start,
                inline_pos,
                block_pos,
                font_height,
            ));
        }
        lines
    }
}

/// 行の確定: 行内範囲（開始〜送り終端）と行送り軸位置から行矩形を組む
/// （行送り軸の厚み方向は行送り方向と同符号——モジュール doc「行矩形の規約」）。
fn finish_line(
    glyphs: Vec<PositionedGlyph>,
    mode: WritingMode,
    inline_start: f32,
    inline_end: f32,
    block_pos: f32,
    font_height: f32,
) -> PositionedLine {
    let rect = match mode {
        WritingMode::HorizontalTb => LineRect {
            left: inline_start,
            top: block_pos,
            right: inline_end,
            bottom: block_pos + font_height,
        },
        WritingMode::VerticalRl => LineRect {
            left: block_pos - font_height,
            top: inline_start,
            right: block_pos,
            bottom: inline_end,
        },
        WritingMode::VerticalLr => LineRect {
            left: block_pos,
            top: inline_start,
            right: block_pos + font_height,
            bottom: inline_end,
        },
    };
    PositionedLine { rect, glyphs }
}

#[cfg(test)]
mod tests {
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    use super::{FixedMetrics, GlyphMetrics, LayoutEngine, LineRect, PositionedLine};
    use crate::region::TextRegion;
    use crate::state::TextItem;
    use crate::writing::WritingMode;

    /// テスト画像原寸（image px・region.rs の檻と同一値）。
    const IMAGE: (u32, u32) = (400, 224);

    /// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定）。
    fn model(
        origin: (Option<i32>, Option<i32>),
        wordwrap: (Option<i32>, Option<i32>),
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(origin.0, origin.1),
            WordWrapPoint::new(wordwrap.0, wordwrap.1),
            ValidRect::new(None, None, None, None),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
        )
    }

    /// n 個の全角グリフ（'あ'）item 列。
    fn glyphs(n: usize) -> Vec<TextItem> {
        std::iter::repeat_n(TextItem::Glyph { ch: 'あ' }, n).collect()
    }

    /// 行のグリフ行内位置列を抜き出す。
    fn inline_positions(line: &PositionedLine) -> Vec<f32> {
        line.glyphs.iter().map(|g| g.inline_pos).collect()
    }

    // ── R4.5: FixedMetrics の決定論仮想値（全角=height・半角=height/2・pitch=ceil(×1.25)） ──

    /// 全角（非 ASCII）は font_height・半角（ASCII）は font_height/2。
    #[test]
    fn fixed_metrics_advance_full_width_for_nonascii_half_for_ascii() {
        let m = FixedMetrics;
        assert_eq!(m.advance('あ', 12.0), 12.0);
        assert_eq!(m.advance('a', 12.0), 6.0);
        assert_eq!(m.advance(' ', 10.0), 5.0);
        assert_eq!(m.advance('漢', 10.0), 10.0);
    }

    /// 行送りピッチは ceil(font_height × 1.25)——端数ケースで檻化
    /// （tasks.md Implementation Notes: floor/丸めなし変異を殺す値を選ぶ）。
    #[test]
    fn fixed_metrics_line_pitch_ceils_fractional_values() {
        let m = FixedMetrics;
        assert_eq!(m.line_pitch(12.0), 15.0); // 15.0（割り切れ）
        assert_eq!(m.line_pitch(10.0), 13.0); // 12.5 → 13（ceil でなければ fail）
    }

    // ── R6.1: 横書き——行内 +x・行送り +y・折返し閾値 wordwrappoint.x ──

    /// 横書きの折返し: 行内位置＋次グリフ幅 > 閾値で折返し、
    /// ちょうど閾値で終わるグリフは折返さない（> 判定・境界檻）。
    #[test]
    fn horizontal_wraps_before_glyph_exceeding_threshold() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // 全角 6 グリフ・font 10: 5 個目は送り終端 50＝閾値ちょうど→残留、
        // 6 個目は 50+10=60 > 50 → 折返し。
        let lines = LayoutEngine::layout(
            &glyphs(6),
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0, 30.0, 40.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0]);
        // 行矩形: 行内範囲＝開始〜送り終端・行送り軸は +y へ pitch(13) 進む。
        assert_eq!(
            lines[0].rect,
            LineRect {
                left: 0.0,
                top: 0.0,
                right: 50.0,
                bottom: 10.0
            }
        );
        assert_eq!(
            lines[1].rect,
            LineRect {
                left: 0.0,
                top: 13.0,
                right: 10.0,
                bottom: 23.0
            }
        );
    }

    /// 半角/全角混在の送り幅が累積する（グリフ別 advance の出力整合・R9.4）。
    #[test]
    fn horizontal_mixed_width_advances_accumulate() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'a' },
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'b' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 6.0, 18.0]);
        let advances: Vec<f32> = lines[0].glyphs.iter().map(|g| g.advance).collect();
        assert_eq!(advances, vec![6.0, 12.0, 6.0]);
        assert_eq!(lines[0].rect.right, 24.0);
    }

    // ── R6.2: 日本語縦書き（vertical_rl）——行内 +y・行送り −x・閾値 wordwrappoint.y ──

    /// vertical_rl: 折返しは y 閾値・列は右→左へ pitch 分進む・列矩形は左方向へ厚み。
    #[test]
    fn vertical_rl_wraps_on_y_threshold_and_feeds_leftward() {
        let region = TextRegion::resolve(
            &model((None, None), (None, Some(30))),
            IMAGE,
            WritingMode::VerticalRl,
        );
        // 書字開始角＝validrect 右上 (400, 0)。全角 5 グリフ・font 10:
        // 3 個目は送り終端 30＝閾値ちょうど→残留、4 個目で折返し。
        assert_eq!(region.start(), (400.0, 0.0));
        let lines = LayoutEngine::layout(
            &glyphs(5),
            5,
            &region,
            WritingMode::VerticalRl,
            10.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0, 10.0]);
        // 列 0: x 帯 [390,400]・y 0..30。列 1: block 400−13=387 → x 帯 [377,387]。
        assert_eq!(
            lines[0].rect,
            LineRect {
                left: 390.0,
                top: 0.0,
                right: 400.0,
                bottom: 30.0
            }
        );
        assert_eq!(
            lines[1].rect,
            LineRect {
                left: 377.0,
                top: 0.0,
                right: 387.0,
                bottom: 20.0
            }
        );
    }

    // ── R6.3: vertical_lr——行内軸は rl と同一・行送りだけ +x（単一読み替え規則） ──

    /// vertical_lr: 折返し・行内位置は vertical_rl と完全一致し、
    /// 列送りの向きだけが +x へ反転する（軸読み替えの単一規則の檻）。
    #[test]
    fn vertical_lr_mirrors_rl_inline_layout_and_feeds_rightward() {
        let model = model((None, None), (None, Some(30)));
        let region_lr = TextRegion::resolve(&model, IMAGE, WritingMode::VerticalLr);
        // 書字開始角＝validrect 左上 (0, 0)。
        assert_eq!(region_lr.start(), (0.0, 0.0));
        let lines_lr = LayoutEngine::layout(
            &glyphs(5),
            5,
            &region_lr,
            WritingMode::VerticalLr,
            10.0,
            &FixedMetrics,
        );
        assert_eq!(lines_lr.len(), 2);
        assert_eq!(
            lines_lr[0].rect,
            LineRect {
                left: 0.0,
                top: 0.0,
                right: 10.0,
                bottom: 30.0
            }
        );
        assert_eq!(
            lines_lr[1].rect,
            LineRect {
                left: 13.0,
                top: 0.0,
                right: 23.0,
                bottom: 20.0
            }
        );
        // 行内軸の配置は vertical_rl と同一（読み替え規則は 1 つ・分岐なし）。
        let region_rl = TextRegion::resolve(&model, IMAGE, WritingMode::VerticalRl);
        let lines_rl = LayoutEngine::layout(
            &glyphs(5),
            5,
            &region_rl,
            WritingMode::VerticalRl,
            10.0,
            &FixedMetrics,
        );
        for (lr, rl) in lines_lr.iter().zip(&lines_rl) {
            assert_eq!(inline_positions(lr), inline_positions(rl));
        }
    }

    // ── 改行マーカー（NewLine{ratio}）: 行送り量 = line_pitch × ratio（正準表） ──

    /// 明示改行の ratio（1.0／0.5）が行送り量へ反映される
    /// （font 12 → pitch 15: 行送り軸位置 0 → 15 → 22.5・端数檻）。
    #[test]
    fn explicit_line_break_ratio_scales_line_feed() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 3);
        let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
        assert_eq!(tops, vec![0.0, 15.0, 22.5]);
        assert!(lines.iter().all(|l| l.glyphs.len() == 1));
    }

    /// 縦書きでも改行マーカーは列送り（−x）として同じ規則で効く（正準表「同（列送り）」）。
    #[test]
    fn vertical_line_break_feeds_column_axis() {
        let region = TextRegion::resolve(
            &model((None, None), (None, None)),
            IMAGE,
            WritingMode::VerticalRl,
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            2,
            &region,
            WritingMode::VerticalRl,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2);
        // pitch 15 × 0.5 = 7.5 だけ左へ: 列 1 の右辺 = 400 − 7.5。
        assert_eq!(lines[0].rect.right, 400.0);
        assert_eq!(lines[1].rect.right, 392.5);
    }

    // ── 縮退・境界 ──

    /// 行頭の 1 グリフが閾値超過でも配置される（無限折返しなし・グリフを落とさない）。
    #[test]
    fn single_glyph_exceeding_threshold_is_placed_per_line() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(3), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let lines = LayoutEngine::layout(
            &glyphs(2),
            2,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2, "1 行 1 グリフで前進する（無限ループしない）");
        assert_eq!(inline_positions(&lines[0]), vec![0.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0]);
    }

    /// 空入力は空の行列（行なし）。可視 0 のグリフ列も行を生まない。
    #[test]
    fn empty_input_and_zero_visible_yield_no_lines() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let empty = LayoutEngine::layout(
            &[],
            0,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert!(empty.is_empty());
        let unrevealed = LayoutEngine::layout(
            &glyphs(3),
            0,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert!(unrevealed.is_empty());
    }

    /// 可視 prefix: visible_count 個のグリフだけが配置され、超過指定は全量へ飽和する。
    #[test]
    fn visible_count_gates_placed_glyphs() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items: Vec<TextItem> = "abcde".chars().map(|ch| TextItem::Glyph { ch }).collect();
        let partial = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(partial.len(), 1);
        assert_eq!(inline_positions(&partial[0]), vec![0.0, 6.0, 12.0]);
        let saturated = LayoutEngine::layout(
            &items,
            99,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(saturated[0].glyphs.len(), 5);
    }

    /// 可視 prefix 内の改行マーカーは即時反映され（R2.2）、直後のグリフが未リビール
    /// でも空行として現れる。prefix 外（打ち切り後）の item は反映されない。
    #[test]
    fn line_break_within_visible_prefix_opens_empty_line() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'b' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            1,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].glyphs.len(), 1);
        assert!(lines[1].glyphs.is_empty(), "改行は即時反映＝空行が現れる");
        // 空行の矩形: 行内零幅・行送り軸位置は pitch(15) 分進んでいる。
        assert_eq!(
            lines[1].rect,
            LineRect {
                left: 0.0,
                top: 15.0,
                right: 0.0,
                bottom: 27.0
            }
        );
    }

    /// 末尾改行（全グリフ可視）は空の新行を開く（後続 3.2 のあふれ判定入力になる形）。
    #[test]
    fn trailing_line_break_opens_empty_line() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [TextItem::Glyph { ch: 'あ' }, TextItem::LineBreak { ratio: 1.0 }];
        let lines = LayoutEngine::layout(
            &items,
            1,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2);
        assert!(lines[1].glyphs.is_empty());
    }

    /// 折返し・改行後の行は描画開始点の行内成分へ戻る（origin が validrect 内部の場合も
    /// 同一規則・行送り軸だけが進む）。
    #[test]
    fn wrapped_lines_return_to_start_inline_component() {
        let region = TextRegion::resolve(
            &model((Some(100), Some(50)), (Some(150), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.start(), (100.0, 50.0));
        // font 10: 100,110,120,130,140（5 個目終端 150＝閾値ちょうど）→ 6 個目で折返し。
        let lines = LayoutEngine::layout(
            &glyphs(6),
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(inline_positions(&lines[1]), vec![100.0]);
        assert_eq!(lines[1].rect.top, 63.0); // 50 + pitch 13
    }

    // ── R2.5 系/R11.6: 決定論（同一入力→同一出力・DirectWrite 非依存の構造テスト） ──

    /// 同一入力に対する layout 出力は完全一致する（純関数・決定論檻）。
    #[test]
    fn same_input_yields_identical_output() {
        let model = model((Some(0), Some(0)), (Some(50), None));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let region = TextRegion::resolve(&model, IMAGE, mode);
            let items = [
                TextItem::Glyph { ch: 'あ' },
                TextItem::LineBreak { ratio: 0.5 },
                TextItem::Glyph { ch: 'a' },
            ];
            let first = LayoutEngine::layout(&items, 2, &region, mode, 10.0, &FixedMetrics);
            let second = LayoutEngine::layout(&items, 2, &region, mode, 10.0, &FixedMetrics);
            assert_eq!(first, second, "mode {mode:?} で決定論が崩れている");
        }
    }
}
