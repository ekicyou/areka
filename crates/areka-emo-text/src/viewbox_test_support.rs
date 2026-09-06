use super::{FramePlan, PhysicalRect, ScrollPlanner};
use crate::canvas::ContentCanvas;
use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

// ── 3.2 R2.2/3.2/3.3/4.2: ダーティ導出（露出帯 ∪ 変化行 ∪ 全域）の檻 ──
//
// 幾何の共通前提: FixedMetrics・font 10 → pitch 12（10 + 行間 2）・全角 1 グリフ/行。
// 露出帯の辺は写像正準表（横書き＝下端・vertical_rl＝左端・vertical_lr＝右端）。

/// テスト画像原寸（image px・他モジュール檻と同一値）。
pub(super) const IMAGE: (u32, u32) = (400, 224);

/// PhysicalRect 短縮構築。
pub(super) fn phys(x: u32, y: u32, w: u32, h: u32) -> PhysicalRect {
    PhysicalRect { x, y, w, h }
}

/// テスト用 BalloonModel（origin (0,0)・折返し既定・validrect 指定）。
pub(super) fn model_rect(
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
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

/// items→canvas の通し（validrect-local・visible は全量）。
pub(super) fn canvas_for(
    items: &[TextItem],
    mode: WritingMode,
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    font_height: f32,
) -> ContentCanvas {
    let region = TextRegion::resolve(&model_rect(validrect), IMAGE, mode);
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    let lines = LayoutEngine::layout(
        items,
        visible,
        &region,
        mode,
        font_height,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    ContentCanvas::from_layout(&lines, &region, mode)
}

/// 可視窓のみ移動（content 不変・block_offset だけ変化・blit≠0）。
pub(super) fn window(first_visible_line: usize, block_offset: f32) -> VisibleWindow {
    VisibleWindow {
        first_visible_line,
        block_offset,
    }
}

// ══════════════════════════════════════════════════════════════════════
// 3.4 ScrollPlanner 純粋層ユニットテスト一式（design Testing Strategy →
//     Unit Tests の 5 項目を檻化）。純粋層規律: windows 非依存・テストのみ追加。
// ══════════════════════════════════════════════════════════════════════

/// 初回フレーム（window 0）を plan→commit して prev_lines を張る共通前処理。
/// 以後の plan は Update（初回全域を確定済みゆえスクロール/伸長を弁別できる）。
pub(super) fn commit_initial(
    planner: &mut ScrollPlanner,
    canvas: &ContentCanvas,
    mode: WritingMode,
    contract: &ScaleContract,
    surface: (u32, u32),
) {
    let w = window(0, 0.0);
    let first = planner.plan(canvas, &w, mode, contract, surface);
    planner.commit(canvas, &w, mode, contract, &first);
}

/// `FramePlan::Update` から blit とダーティ矩形を取り出す（他 variant は panic）。
pub(super) fn expect_update(plan: &FramePlan) -> ((i32, i32), Vec<PhysicalRect>) {
    match plan {
        FramePlan::Update { blit, dirty, .. } => (*blit, dirty.clone()),
        other => panic!("Update を期待したが {other:?} が現れた"),
    }
}
