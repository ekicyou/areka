//! # layout_styled — 装飾入りの配置（純粋層・`layout.rs` の子）
//!
//! 公開の入口 [`LayoutEngine::layout_styled`] と、その入口が使う 3 つの決め方を持つ:
//! [`glyph_style_advance`]（1 文字ぶんの装飾番号・送り幅・em の大きさ）、
//! [`LineHeights`]（行の丈の追跡）、[`line_pitch_of`]（行送り）。
//!
//! 純粋層: `windows` 系 crate へ依存しない。

use areka_sakura::contract::ActorKey;

use crate::cursor_tag::CursorWarnGuard;
use crate::look::{GlyphStyles, StyleId};
use crate::region::TextRegion;
use crate::state::TextItem;
use crate::writing::WritingMode;

use super::{GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};

impl LayoutEngine {
    /// 装飾（グリフ序数 → 見た目）つきの配置。装飾を持つ台本の唯一の入口で、
    /// 番号列が既定だけなら出力は [`LayoutEngine::layout`] と一致する。
    #[allow(clippy::too_many_arguments)]
    pub fn layout_styled(
        items: &[TextItem],
        visible_count: usize,
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
        metrics: &dyn GlyphMetrics,
        wrap: WrapPlan<'_>,
        styles: GlyphStyles<'_>,
        actor: &ActorKey,
        warn: &mut CursorWarnGuard,
    ) -> Vec<PositionedLine> {
        Self::layout_inner(
            items,
            visible_count,
            region,
            mode,
            font_height,
            metrics,
            wrap,
            Some((actor, warn)),
            Some(styles),
        )
    }
}

/// 1 文字ぶんの `(装飾番号, 送り幅, em の大きさ)`。配置ループと塊の送り幅合計がこの関数だけを通る。
///
/// 番号列が無い経路と既定の番号の文字は `advance(text, font_height)` で測り、高さも
/// `font_height` を返す。既定の見た目の高さが `font_height` と食い違う登録前の一瞬でも、
/// 装飾なしの出力を動かさないため。
pub(super) fn glyph_style_advance(
    text: &str,
    ordinal: usize,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
    styles: Option<&GlyphStyles<'_>>,
) -> (StyleId, f32, f32) {
    // 目印の字は全角の空白として測る（描くときと同じ）。
    let text = crate::inline_marker::stand_in(text);
    let default = || {
        (
            StyleId::DEFAULT,
            metrics.advance(text, font_height),
            font_height,
        )
    };
    match styles {
        None => default(),
        Some(s) => match s.id_of(ordinal) {
            StyleId::DEFAULT => default(),
            id => {
                let look = s.look_of(ordinal);
                (id, metrics.advance_styled(text, look), look.height)
            }
        },
    }
}

/// 行の丈——閉じる行に置かれた文字の em の最大値を追う。
///
/// [`LineHeights::close`] は次の優先順で返す:
///
/// 1. 閉じる行に文字が置かれていれば、その行の最大 em。
/// 2. 文字の無い行（改行だけの行）は、後ろに置かれる文字の大きさ。
/// 3. 後ろにも文字が無いときだけ `font_height`（配置の結果に出ない）。
pub(super) struct LineHeights {
    /// 後ろに字が無いときの丈（`font_height`）。
    fallback: f32,
    /// まだ閉じていない行に置かれた文字の最大 em（文字が無ければ `None`）。
    line_max: Option<f32>,
}

impl LineHeights {
    /// 追跡を始める。
    pub(super) fn new(font_height: f32) -> LineHeights {
        LineHeights {
            fallback: font_height,
            line_max: None,
        }
    }

    /// 文字を 1 つ置いた（行内最大 em の更新）。
    pub(super) fn place(&mut self, height: f32) {
        self.line_max = Some(self.line_max.map_or(height, |m| m.max(height)));
    }

    /// 開いている行を閉じて丈を返す。`next` は後ろに置かれる文字の大きさ（無ければ `None`）。
    pub(super) fn close(&mut self, next: Option<f32>) -> f32 {
        self.line_max.take().or(next).unwrap_or(self.fallback)
    }

    /// いま閉じたとしたら返る丈を、閉じずに覗く（`\_l` の実効位置の先読み用）。
    /// `close` と別の規則を持つと、`\_l` を挟んだ行だけ行送りが食い違う。
    pub(super) fn peek(&self, next: Option<f32>) -> f32 {
        self.line_max.or(next).unwrap_or(self.fallback)
    }
}

/// 行送り——配置層で `TextLayerConfig::line_pitch` へ届く唯一の点。
///
/// 装飾は渡す高さだけを変え、式には触れない。
pub(super) fn line_pitch_of(metrics: &dyn GlyphMetrics, height: f32) -> f32 {
    metrics.line_pitch(height)
}
