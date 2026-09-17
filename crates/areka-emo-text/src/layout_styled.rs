//! # layout_styled — 装飾入りの配置（純粋層・`layout.rs` の子）
//!
//! 配置の本体 [`LayoutEngine::layout_inner`] へ「グリフ序数 → 見た目」の読み口
//! （[`GlyphStyles`]）を渡す**公開の入口** [`LayoutEngine::layout_styled`] と、その入口が
//! 使う 3 つの決め方を持つ:
//!
//! 1. [`glyph_style_advance`]——1 文字ぶんの「装飾番号・送り幅・em の大きさ」を決める
//!    **唯一の点**。本体の配置ループと塊の合計（[`crate::layout::line_ops::segment_advance_sum`]）
//!    が同じ関数を通ることで、「区間ごとの送り幅合計も同じ見た目で合計する」が
//!    値の一致ではなく**構造**で保たれる（要件 11.2）。
//! 2. [`LineHeights`]——行の丈（行内最大 em）の追跡（要件 7.9）。
//! 3. [`line_pitch_of`]——行送り。計測器の `line_pitch` を呼ぶ綴りは配置層でここ 1 か所にしかなく、
//!    その先は `TextLayerConfig::line_pitch` の 1 点である（要件 7.8）。装飾のための別の式も
//!    係数もここには無い——引数が高さ 1 つだけであることがそれを型で示している。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。

use areka_sakura::contract::ActorKey;

use crate::cursor_tag::CursorWarnGuard;
use crate::look::{GlyphStyles, StyleId};
use crate::region::TextRegion;
use crate::state::TextItem;
use crate::writing::WritingMode;

use super::{GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};

impl LayoutEngine {
    /// [`LayoutEngine::layout_with_cursor_warn`] の全挙動＋文字ごとの見た目
    /// （送り幅・行内最大 em・装飾番号の転写）。
    ///
    /// 装飾を持つ台本の唯一の入口である。番号列が既定だけなら出力は装飾導入前と一致する
    /// （`layout_styled_tests.rs` の非回帰の檻）。
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

/// 1 文字ぶんの `(装飾番号, 送り幅, em の大きさ)`。
///
/// 番号列が無い経路と既定の番号の文字は従来どおり `advance(ch, font_height)` で測り、
/// 高さも `font_height` を返す——既定の見た目の高さが `font_height` と食い違う登録前の
/// 一瞬でも、装飾なしの出力を 1 ビットも動かさないためである（`layout_styled_tests.rs` の
/// 「番号列を渡さない出力」「既定だけの番号列」の 2 本が字義の期待値で見張る）。
pub(super) fn glyph_style_advance(
    ch: char,
    ordinal: usize,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
    styles: Option<&GlyphStyles<'_>>,
) -> (StyleId, f32, f32) {
    let default = || {
        (
            StyleId::DEFAULT,
            metrics.advance(ch, font_height),
            font_height,
        )
    };
    match styles {
        None => default(),
        Some(s) => match s.id_of(ordinal) {
            StyleId::DEFAULT => default(),
            id => {
                let look = s.look_of(ordinal);
                (id, metrics.advance_styled(ch, look), look.height)
            }
        },
    }
}

/// 行の丈（要件 7.9）——閉じる行に置かれた文字の em の最大値を追う。
///
/// 3 つの答えを **1 つの優先順**で返す（[`LineHeights::close`]）:
///
/// 1. 閉じる行に文字が置かれていれば、その行の最大 em。
/// 2. 文字の無い行（改行だけの行）は、そのとき効いている大きさ＝**次に置く文字**の大きさ。
/// 3. 次に置く文字も無いまま閉じるとき（`\_l` の先行実体化）は、スコープの**現在の見た目**の
///    大きさ（[`GlyphStyles::current`]）。
pub(super) struct LineHeights {
    /// スコープの現在の見た目の高さ（番号列が無い経路は `font_height`）。
    current: f32,
    /// まだ閉じていない行に置かれた文字の最大 em（文字が無ければ `None`）。
    line_max: Option<f32>,
}

impl LineHeights {
    /// 追跡を始める（番号列が無い経路は現在の見た目も `font_height`＝従来と同値）。
    pub(super) fn new(styles: Option<&GlyphStyles<'_>>, font_height: f32) -> LineHeights {
        LineHeights {
            current: styles.map_or(font_height, |s| s.current.height),
            line_max: None,
        }
    }

    /// 文字を 1 つ置いた（行内最大 em の更新）。
    pub(super) fn place(&mut self, height: f32) {
        self.line_max = Some(self.line_max.map_or(height, |m| m.max(height)));
    }

    /// 開いている行を閉じる——丈を返し、行内最大を空へ戻す。
    /// `next` は「この直後に置かれる文字の大きさ」（先行実体化の経路では `None`）。
    pub(super) fn close(&mut self, next: Option<f32>) -> f32 {
        self.line_max.take().or(next).unwrap_or(self.current)
    }

    /// いま閉じたとしたら返る丈を、**閉じずに**覗く（`\_l` の実効位置の先読み用）。
    ///
    /// [`LineHeights::close`] と同じ優先順から「次に置く文字」だけを落とした形である
    /// ——`\_l` の時点で次に置かれる文字はまだ読めていないので、文字の無い行はスコープの
    /// 現在の見た目へ落ちる。先読みが `close` と別の規則を持つと、`\_l` を挟んだ行だけ
    /// 行送りが食い違う（`layout_styled_tests.rs` の
    /// `the_cursor_preview_of_a_pending_newline_uses_the_closing_lines_pitch`）。
    pub(super) fn peek(&self) -> f32 {
        self.line_max.unwrap_or(self.current)
    }
}

/// 行送り——配置層で `TextLayerConfig::line_pitch` へ届く**唯一の点**（要件 7.8）。
///
/// 装飾は「どの高さを渡すか」だけを変え、式そのものには触れない。
pub(super) fn line_pitch_of(metrics: &dyn GlyphMetrics, height: f32) -> f32 {
    metrics.line_pitch(height)
}
