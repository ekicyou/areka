use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use super::{FixedMetrics, LayoutEngine, PositionedLine, VisibleWindow, WrapPlan};
use crate::region::TextRegion;
use crate::state::TextItem;
use crate::writing::WritingMode;

/// テスト画像原寸（image px・region.rs の檻と同一値）。
pub(super) const IMAGE: (u32, u32) = (400, 224);

/// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定）。
pub(super) fn model(
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
        None,
    )
}

/// n 個の全角グリフ（'あ'）item 列。
pub(super) fn glyphs(n: usize) -> Vec<TextItem> {
    std::iter::repeat_n(TextItem::Glyph { ch: 'あ' }, n).collect()
}

/// 行のグリフ行内位置列を抜き出す。
pub(super) fn inline_positions(line: &PositionedLine) -> Vec<f32> {
    line.glyphs.iter().map(|g| g.inline_pos).collect()
}

// ── R2.5 系/R11.6: 決定論（同一入力→同一出力・DirectWrite 非依存の構造テスト） ──

// ── 3.2 R7.1/7.2/7.4/7.5（+R6.4）: あふれ判定とスクロール可視窓（visible_window） ──
//
// 幾何の共通前提: FixedMetrics・font 10 → pitch 13（ceil(12.5)）・全角 1 グリフ/行。
// あふれ判定は軸読み替え正準表の行（横書き=最新行の下端 > validrect.bottom・
// vertical_rl=最新列の左端 < validrect.left・vertical_lr=最新列の右端 > validrect.right）。

/// テスト用 BalloonModel 生成ヘルパ（validrect 込み・順序は top,bottom,left,right）。
pub(super) fn model_rect(
    origin: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(origin.0, origin.1),
        WordWrapPoint::new(None, None),
        ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// n 行（各行 全角 1 グリフ・明示改行 ratio 1.0 区切り）の item 列。
pub(super) fn broken_lines(n: usize) -> Vec<TextItem> {
    let mut items = Vec::new();
    for i in 0..n {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::Glyph { ch: 'あ' });
    }
    items
}

/// layout→visible_window の通し（テスト用最短経路・visible は全量）。
pub(super) fn window_for(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> VisibleWindow {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    let lines = LayoutEngine::layout(
        items,
        visible,
        region,
        mode,
        font_height,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    LayoutEngine::visible_window(&lines, region, mode)
}
