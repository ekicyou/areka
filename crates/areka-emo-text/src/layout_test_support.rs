use super::{FixedMetrics, LayoutEngine, PositionedLine, VisibleWindow, WrapPlan};
use crate::cursor_tag::CursorWarnGuard;
use crate::look::{GlyphStyles, StyleTable, TextLook};
use crate::region::TextRegion;
use crate::segment::{Segment, SegmentPlan};
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::ActorKey;

/// テスト画像原寸（image px・region.rs の檻と同一値）。
pub(crate) const IMAGE: (u32, u32) = (400, 224);

/// テスト用 BalloonModel 生成ヘルパ（幾何成分を全部指定・validrect の順序は top,bottom,left,right）。
///
/// 配置・領域・描画の各テストが同じ組み立てを使う（写しを持たない）。
pub(crate) fn model_full(
    origin: (Option<i32>, Option<i32>),
    wordwrap: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(origin.0, origin.1),
        WordWrapPoint::new(wordwrap.0, wordwrap.1),
        ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定）。
pub(super) fn model(
    origin: (Option<i32>, Option<i32>),
    wordwrap: (Option<i32>, Option<i32>),
) -> BalloonModel {
    model_full(origin, wordwrap, (None, None, None, None))
}

/// テスト用 BalloonModel 生成ヘルパ（`writing_mode` 以外は全成分未指定）。
pub(crate) fn model_writing(writing_mode: Option<&str>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        writing_mode.map(str::to_owned),
        None,
    )
}

/// 横書き・`origin (0, 0)`・wordwrappoint 未指定の領域（文字描画開始点 (0, 0)・折返し閾値＝画像右辺 400）。
pub(super) fn zero_origin_region() -> TextRegion {
    TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    )
}

/// 全角グリフ 1 個（'あ'・`FixedMetrics` で advance 10）。
pub(super) fn glyph() -> TextItem {
    TextItem::glyph("あ")
}

/// 明示改行（ratio 1.0）。
pub(super) fn newline() -> TextItem {
    TextItem::LineBreak { ratio: 1.0 }
}

/// `\_l[x,y]`（両軸とも絶対 px）。
pub(super) fn cursor_px(x: f32, y: f32) -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Absolute {
            value: x,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Absolute {
            value: y,
            unit: CursorUnit::Px,
        },
    }
}

/// 行矩形を `(left, top, right, bottom)` で取り出す。
pub(super) fn rect_of(line: &PositionedLine) -> (f32, f32, f32, f32) {
    let r = &line.rect;
    (r.left, r.top, r.right, r.bottom)
}

/// 全行の行矩形を `(left, top, right, bottom)` の列で取り出す。
pub(super) fn rects(lines: &[PositionedLine]) -> Vec<(f32, f32, f32, f32)> {
    lines.iter().map(rect_of).collect()
}

/// n 個の全角グリフ（'あ'）item 列。
pub(crate) fn glyphs(n: usize) -> Vec<TextItem> {
    std::iter::repeat_n(TextItem::glyph("あ"), n).collect()
}

/// 行のグリフ行内位置列を抜き出す。
pub(super) fn inline_positions(line: &PositionedLine) -> Vec<f32> {
    line.glyphs.iter().map(|g| g.inline_pos).collect()
}

// ── 3.2 R7.1/7.2/7.4/7.5（+R6.4）: あふれ判定とスクロール可視窓（visible_window） ──
//
// 幾何の共通前提: FixedMetrics・font 10 → pitch 12（10 + 行間 2）・全角 1 グリフ/行。
// あふれ判定は軸読み替え正準表の行（横書き=最新行の下端 > validrect.bottom・
// vertical_rl=最新列の左端 < validrect.left・vertical_lr=最新列の右端 > validrect.right）。

/// テスト用 BalloonModel 生成ヘルパ（validrect 込み・順序は top,bottom,left,right）。
pub(crate) fn model_rect(
    origin: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    model_full(origin, (None, None), validrect)
}

/// n 行（各行 全角 1 グリフ・明示改行 ratio 1.0 区切り）の item 列。
pub(crate) fn broken_lines(n: usize) -> Vec<TextItem> {
    let mut items = Vec::new();
    for i in 0..n {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::glyph("あ"));
    }
    items
}

/// items→行の通し（テスト用最短経路・FixedMetrics・1 字ずつ折る・visible は全量）。
///
/// 配置・canvas・描き直しの計画の各テストが同じ通しを使う（写しを持たない）。
pub(crate) fn layout_all(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> Vec<PositionedLine> {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    LayoutEngine::layout(
        items,
        visible,
        region,
        mode,
        font_height,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// guard つきの配置（本番の入口 `layout_styled`・既定の見た目だけ・FixedMetrics・1 字ずつ折る）。
///
/// 既定の見た目の高さを `font_height` にそろえるので、番号列を渡さない配置と同じ行を返す
/// （`glyph_style_advance`・`LineHeights::new`）。`\_l` の縮退の記録を guard で見るテストが使う。
pub(super) fn layout_guarded(
    items: &[TextItem],
    visible_count: usize,
    region: &TextRegion,
    font_height: f32,
    actor: &ActorKey,
    guard: &mut CursorWarnGuard,
) -> Vec<PositionedLine> {
    let table = StyleTable::default();
    let look = TextLook {
        height: font_height,
        ..TextLook::ukadoc_default()
    };
    LayoutEngine::layout_styled(
        items,
        visible_count,
        region,
        WritingMode::HorizontalTb,
        font_height,
        &FixedMetrics,
        WrapPlan::CharByChar,
        GlyphStyles {
            table: &table,
            ids: &[],
            default: &look,
            current: &look,
        },
        actor,
        guard,
    )
}

/// layout→visible_window の通し（テスト用最短経路・visible は全量）。
pub(super) fn window_for(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> VisibleWindow {
    let lines = layout_all(items, region, mode, font_height);
    LayoutEngine::visible_window(&lines, region, mode)
}

/// `(start, len)` 列から手組みの [`SegmentPlan`] を作る（budoux 非依存で塊境界を注入する）。
pub(super) fn plan(segs: &[(usize, usize)]) -> SegmentPlan {
    SegmentPlan::from_segments(
        segs.iter()
            .map(|&(start, len)| Segment { start, len })
            .collect(),
    )
}
