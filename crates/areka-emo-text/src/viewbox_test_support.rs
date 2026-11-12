use super::{FramePlan, PhysicalRect, ScrollPlanner};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, GlyphRunContent, RegionTransform, Resident,
    ResidentContent, TextEffects,
};
use crate::layout::test_support::{layout_all, model_rect};
use crate::layout::{PositionedGlyph, PositionedLine, VisibleWindow};
use crate::look::StyleId;
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::writing::WritingMode;
use areka_sakura::cluster::{cluster_count, clusters};

/// テスト画像原寸と `broken_lines` は配置の支えのものを使う（写しを持たない）。
pub(super) use crate::layout::test_support::{IMAGE, broken_lines};

/// PhysicalRect 短縮構築。
pub(super) fn phys(x: u32, y: u32, w: u32, h: u32) -> PhysicalRect {
    PhysicalRect { x, y, w, h }
}

/// items→(行, canvas) の通し（visible は全量）。canvas・描き直しの計画・描画のテストが共用する。
pub(crate) fn layout_canvas(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> (Vec<PositionedLine>, ContentCanvas) {
    let lines = layout_all(items, region, mode, font_height);
    let canvas = ContentCanvas::from_layout(&lines, region, mode);
    (lines, canvas)
}

/// items→canvas の通し（origin は書字方向の既定の開始角・validrect-local・visible は全量）。
/// 縦書きの右から左を origin (0,0) で組むと最初の列が面の外（x∈[-10,0]）に出るため空の組を使う。
/// 幾何の共通前提: FixedMetrics・font 10 → pitch 12（10 + 行間 2）・全角 1 グリフ/行。
/// 露出帯の辺は写像正準表（横書き＝下端・vertical_rl＝左端・vertical_lr＝右端）。
pub(super) fn canvas_for(
    items: &[TextItem],
    mode: WritingMode,
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    font_height: f32,
) -> ContentCanvas {
    let region = TextRegion::resolve(&model_rect((None, None), validrect), IMAGE, mode);
    layout_canvas(items, &region, mode, font_height).1
}

/// 行内のグリフ住人の中身（inline_pos 連番・全角 advance 10・丈 10）。
/// `ids` は字ごとの装飾番号の列で、空なら全部 [`StyleId::DEFAULT`]。空でなければ長さは文字数と
/// 一致させる（配置が `glyph_styles[ordinal]` を写す不変条件と同じ形）。
pub(super) fn run_content(text: &str, ids: &[u32]) -> GlyphRunContent {
    let count = cluster_count(text);
    assert!(
        ids.is_empty() || count == ids.len(),
        "番号列の長さは文字数と一致させる"
    );
    let glyphs = clusters(text)
        .enumerate()
        .map(|(i, ch)| PositionedGlyph {
            text: ch.into(),
            inline_pos: i as f32 * 10.0,
            advance: 10.0,
            style: ids.get(i).map_or(StyleId::DEFAULT, |&id| StyleId(id)),
        })
        .collect();
    GlyphRunContent {
        glyphs,
        size: (count as f32 * 10.0, 10.0),
    }
}

/// GlyphRun 住人（非 Choice・ブロック軸位置 dy・装飾番号の列は [`run_content`] と同じ）。
pub(super) fn glyph_resident(text: &str, dy: f32, ids: &[u32]) -> Resident {
    Resident {
        content: ResidentContent::GlyphRun(run_content(text, ids)),
        transform: RegionTransform::translation(0.0, dy),
        effects: TextEffects::default(),
    }
}

/// Choice 住人（hover の ordinal を注入・ブロック軸位置 dy・素描画は GlyphRun と同一）。
pub(super) fn choice_resident(
    text: &str,
    dy: f32,
    ids: &[u32],
    hovered: Option<usize>,
) -> Resident {
    let run = run_content(text, ids);
    let w = run.size.0;
    Resident {
        content: ResidentContent::Choice(ChoiceLineContent {
            run,
            segments: vec![ChoiceRowSegment {
                ordinal: 0,
                inline_range: (0.0, w),
            }],
            hovered,
            highlight: None,
            // 帯は指紋／ダーティ導出の入力ではない（ダーティ帯の拡張は COM 層
            // `expand_overhang_for_band` の領分）——ここでは em ボックス丈で足りる。
            band_extent: 10.0,
            band_offset: 0.0,
        }),
        transform: RegionTransform::translation(0.0, dy),
        effects: TextEffects::default(),
    }
}

/// 可視窓のみ移動（content 不変・block_offset だけ変化・blit≠0）。
pub(super) fn window(first_visible_line: usize, block_offset: f32) -> VisibleWindow {
    VisibleWindow {
        first_visible_line,
        block_offset,
    }
}

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

/// `FramePlan::Update` から blit とダーティ**矩形**を取り出す（他 variant は panic）。
/// 矩形ごとの交差行（[`super::DirtyRect::lines`]）は落とす——矩形の並びだけを見る檻の共通口。
pub(super) fn expect_update(plan: &FramePlan) -> ((i32, i32), Vec<PhysicalRect>) {
    match plan {
        FramePlan::Update { blit, dirty, .. } => (*blit, dirty.iter().map(|d| d.rect).collect()),
        other => panic!("Update を期待したが {other:?} が現れた"),
    }
}
