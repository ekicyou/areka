// ── タスク 5.4: canvas 装飾（decorate_canvas）——GlyphRun 住人 → Choice 住人置換 ──
//
// design.md「純粋層 / ChoicePure」・要件 1.1（Choice cue→行 resident 描画）/4.2（cursor.* 指定→
// SquareFill）/4.3（未指定→Invert 反転）/4.5（描画実行の一点写像＝焼込済み純データ）。
// 座標系: LineChoiceSegment.inline_range は絶対 image px・ChoiceRowSegment.inline_range は
// resident-local（from_layout がグリフから行内原点を差し引くのと同一原点で変換）。

use super::*;

use crate::canvas::{GlyphRunContent, RegionTransform, Resident, TextEffects};
use crate::layout::PositionedGlyph;
use crate::look::StyleId;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

/// validrect 原点を持つ `TextRegion`（left()/top() のみ inline 原点差引きに使用）。
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

/// 変換 offset `offset` の GlyphRun 住人（グリフ内容は装飾非関与ゆえ最小の 1 グリフ）。
fn glyph_resident(offset: (f32, f32)) -> Resident {
    Resident {
        content: ResidentContent::GlyphRun(GlyphRunContent {
            glyphs: vec![PositionedGlyph {
                text: "あ".into(),
                inline_pos: 0.0,
                advance: 10.0,
                style: StyleId::DEFAULT,
            }],
            size: (10.0, 10.0),
        }),
        transform: RegionTransform::translation(offset.0, offset.1),
        effects: TextEffects::default(),
    }
}

/// 住人列から canvas を組む（size は validrect 寸相当の固定値）。
fn canvas(residents: Vec<Resident>) -> ContentCanvas {
    ContentCanvas {
        residents,
        size: (400.0, 224.0),
    }
}

/// `(line_index, ordinal, 絶対 inline_range)` の [`LineChoiceSegment`]。
fn seg(line_index: usize, ordinal: usize, range: (f32, f32)) -> LineChoiceSegment {
    LineChoiceSegment {
        line_index,
        ordinal,
        inline_range: range,
    }
}

/// 装飾テストのハイライト帯（em ボックス丈 10.0 と**異なる**値＝焼込みが観測可能）。
const TEST_BAND: f32 = 12.0;

/// 既存ケースの帯の寄せ＝0（行ボックス丈 ＝ 帯の丈のフォント＝既定 ＭＳ ゴシックと同じ関係）。
const NO_OFFSET: f32 = 0.0;

/// 帯の寄せの焼込みが観測可能な値（0 とも em ボックス丈とも異なる）。
const TEST_OFFSET: f32 = 3.0;

/// 住人が Choice ならその中身を取り出す（さもなくば panic）。
fn choice(resident: &Resident) -> &ChoiceLineContent {
    match &resident.content {
        ResidentContent::Choice(c) => c,
        other => panic!("Choice 住人を期待したが {other:?}"),
    }
}

/// fixture 実導出の SquareFill スタイル（square 塗り(105,25,25)＋白文字）。
fn square_fill() -> ResolvedChoiceStyle {
    ResolvedChoiceStyle::SquareFill {
        fill: (105, 25, 25),
        text: (255, 255, 255),
    }
}

// ── 恒等（非退行）: セグメント空 → canvas 無変更 ──

/// セグメント空 → 入力 canvas をまったく同一で返す（恒等・要件 1.4）。
#[test]
fn empty_segments_returns_canvas_unchanged() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    let out = decorate_canvas(
        input.clone(),
        &[],
        Some(0),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    assert_eq!(out, input, "セグメント空は恒等（無変更）");
}

// ── hover: 一致行のみ highlight・他行 None ──

/// hover=Some(0) → ordinal 0 を持つ行のみ hovered/highlight が付き、他行は None。
#[test]
fn hover_sets_highlight_on_matching_line_only() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    let l0 = choice(&out.residents[0]);
    assert_eq!(l0.hovered, Some(0));
    assert_eq!(
        l0.highlight,
        Some(HighlightPaint {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        })
    );
    let l1 = choice(&out.residents[1]);
    assert_eq!(l1.hovered, None, "hover 対象外の行は hovered None");
    assert_eq!(l1.highlight, None, "hover 対象外の行は highlight None");
}

// ── hover None: セグメントは記録するが highlight は焼かない ──

/// hover=None でもセグメント持ち行は Choice 住人化（セグメント記録・hovered/highlight None）。
#[test]
fn hover_none_still_records_segments_without_highlight() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        None,
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    for (i, ordinal) in [(0usize, 0usize), (1, 1)] {
        let c = choice(&out.residents[i]);
        assert_eq!(c.hovered, None);
        assert_eq!(c.highlight, None);
        assert_eq!(c.segments.len(), 1);
        assert_eq!(c.segments[0].ordinal, ordinal);
    }
}

// ── 座標系: 絶対 inline_range → resident-local（行内原点差引き） ──

/// 横書き・行内 offset 100（rect.left ≠ region.left()）: 絶対 100..120 → local 0..20
/// （resident 原点 = region.left() + offset.0 = 100 を差し引く）。
#[test]
fn segment_inline_range_is_resident_local_subtracting_line_origin() {
    let region = region(0, 0, 400, 224);
    // 住人の行内 offset は 100（rect.left = region.left()(0) + 100 = 100）。
    let input = canvas(vec![glyph_resident((100.0, 0.0))]);
    let segments = [seg(0, 0, (100.0, 120.0))]; // 絶対 image px。
    let out = decorate_canvas(
        input,
        &segments,
        None,
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    let c = choice(&out.residents[0]);
    assert_eq!(
        c.segments[0].inline_range,
        (0.0, 20.0),
        "絶対 100..120 から行内原点 100 を差し引いた resident-local 0..20"
    );
}

/// Observable（R3.3 の帯単一化）: 装飾は受け取った `band_extent` を Choice 住人へそのまま
/// 焼き込む（em ボックス丈 10.0 ではなく 12.0）——COM 層のハイライト矩形／ダーティ帯は
/// この値だけを読み、`derive_hit_rows` へ渡す値と同一にすることで帯が数値一致する。
#[test]
fn decorate_bakes_band_extent_into_choice_residents() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    for i in [0usize, 1] {
        assert_eq!(
            choice(&out.residents[i]).band_extent,
            TEST_BAND,
            "住人 {i}: hover 有無に依らず帯を焼き込む（hover 解除フレームのダーティ帯にも要る）"
        );
    }
}

/// Observable（R13.1/13.2 の帯の寄せの単一化）: 装飾は受け取った `band_offset` も Choice 住人へ
/// そのまま焼き込む——COM 層のハイライト矩形の起点とダーティ帯の超過分はこの値を読み、
/// `derive_hit_rows` へ渡す値と同一にすることで帯の**位置**まで数値一致する。
#[test]
fn decorate_bakes_band_offset_into_choice_residents() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, TEST_OFFSET),
    );
    for i in [0usize, 1] {
        assert_eq!(
            choice(&out.residents[i]).band_offset,
            TEST_OFFSET,
            "住人 {i}: hover 有無に依らず帯の寄せを焼き込む（hover 解除フレームのダーティ帯にも要る）"
        );
    }
}

/// 非退行（R13.6）: 寄せ 0（行ボックス丈 ＝ 帯の丈の既定 ＭＳ ゴシック）を渡すと住人へ焼かれる
/// 値も 0 になり、下流（ハイライト矩形／ダーティ帯）は寄せを入れる前と 1 画素も変わらない。
#[test]
fn decorate_bakes_zero_band_offset_for_flat_line_box_fonts() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![glyph_resident((0.0, 0.0))]);
    let segments = [seg(0, 0, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    assert_eq!(choice(&out.residents[0]).band_offset, 0.0);
}

/// 縦書き（vertical_rl/lr）: 行内軸＝y。行内 offset.1=50 → 絶対 50..70 は top 原点差引きで local 0..20。
#[test]
fn segment_inline_range_vertical_subtracts_top_origin() {
    let region = region(0, 0, 400, 224);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        // 縦書きは inline 軸 = y。住人 offset.1 = 50（rect.top = region.top()(0) + 50）。
        let input = canvas(vec![glyph_resident((377.0, 50.0))]);
        let segments = [seg(0, 0, (50.0, 70.0))]; // 絶対 y 範囲。
        let out = decorate_canvas(
            input,
            &segments,
            None,
            square_fill(),
            (0, 0, 0),
            &region,
            mode,
            &bands_of(TEST_BAND, NO_OFFSET),
        );
        let c = choice(&out.residents[0]);
        assert_eq!(
            c.segments[0].inline_range,
            (0.0, 20.0),
            "{mode:?}: 縦書きは top 原点（region.top()+offset.1）を差し引く"
        );
    }
}

// ── スタイル分岐: NoMarker は hover でも塗らない・Invert は dfc から解決 ──

/// NoMarker: hover 一致でも paint→None ゆえ highlight None（hovered は Some のまま）。
#[test]
fn no_marker_style_yields_no_highlight_even_when_hovered() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![glyph_resident((0.0, 0.0))]);
    let segments = [seg(0, 0, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        ResolvedChoiceStyle::NoMarker,
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    let c = choice(&out.residents[0]);
    assert_eq!(c.hovered, Some(0), "hover 印は付く");
    assert_eq!(c.highlight, None, "NoMarker は hover でも塗らない");
}

/// Invert: dfc=(10,20,30)・hover 一致 → 塗り=dfc・文字=各成分 255−c=(245,235,225)。
#[test]
fn invert_style_resolves_highlight_from_default_font_color() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![glyph_resident((0.0, 0.0))]);
    let segments = [seg(0, 0, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        ResolvedChoiceStyle::Invert,
        (10, 20, 30),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    let c = choice(&out.residents[0]);
    assert_eq!(
        c.highlight,
        Some(HighlightPaint {
            fill: (10, 20, 30),
            text: (245, 235, 225),
        })
    );
}

// ── 集約: 同一行の複数 ordinal は 1 住人・折返し跨ぎは 2 住人 ──

/// `\q\q` 並置（同一行 2 ordinal）→ 1 つの Choice 住人へ両セグメント集約。hover=Some(1)。
#[test]
fn two_choices_on_one_line_group_into_one_resident() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![glyph_resident((0.0, 0.0))]);
    let segments = [seg(0, 0, (0.0, 20.0)), seg(0, 1, (20.0, 40.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(1),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    assert_eq!(out.residents.len(), 1, "住人数は不変（1 行 1 住人）");
    let c = choice(&out.residents[0]);
    assert_eq!(c.segments.len(), 2, "同一行の 2 ordinal を 1 住人へ集約");
    assert_eq!(c.segments[0].ordinal, 0);
    assert_eq!(c.segments[1].ordinal, 1);
    assert_eq!(c.hovered, Some(1), "hover 対象 ordinal 1 が住人内にある");
    assert!(c.highlight.is_some());
}

/// 折返し跨ぎ（同一 ordinal 0 が 2 行）→ 2 つの Choice 住人・hover=Some(0) で両方 highlight。
#[test]
fn wrapped_choice_highlights_both_lines() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    // 同一 ordinal 0 が line0 末尾と line1 先頭に跨る（annotate_lines の行別分割）。
    let segments = [seg(0, 0, (20.0, 30.0)), seg(1, 0, (0.0, 10.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(0),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    for i in [0usize, 1] {
        let c = choice(&out.residents[i]);
        assert_eq!(c.hovered, Some(0), "行 {i}: 跨いだ ordinal 0 が hover");
        assert!(c.highlight.is_some(), "行 {i}: 両行とも highlight");
    }
}

// ── 素通し: セグメントを持たない行は GlyphRun のまま ──

/// セグメントが line 1 のみ → line 0/2 は GlyphRun 素通し・line 1 のみ Choice 化。
#[test]
fn lines_without_segments_stay_glyph_run() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
        glyph_resident((0.0, 24.0)),
    ]);
    let segments = [seg(1, 0, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        None,
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    assert!(
        matches!(out.residents[0].content, ResidentContent::GlyphRun(_)),
        "セグメント無し行 0 は GlyphRun 素通し"
    );
    assert!(
        matches!(out.residents[1].content, ResidentContent::Choice(_)),
        "セグメント有り行 1 は Choice 化"
    );
    assert!(
        matches!(out.residents[2].content, ResidentContent::GlyphRun(_)),
        "セグメント無し行 2 は GlyphRun 素通し"
    );
}

// ── stale ordinal: hover が存在しない ordinal → どの行も highlight 無し（不変条件 3・4.5/6.5） ──

/// hover=Some(99)（どのセグメントも持たない ordinal）→ 全住人 hovered/highlight None・
/// ただしセグメント持ち行は Choice 住人化しセグメントは記録する（stale-safe 不変条件・
/// Data Models §不変条件 3・要件 4.5/6.5）。
#[test]
fn hover_stale_ordinal_yields_no_highlight() {
    let region = region(0, 0, 400, 224);
    let input = canvas(vec![
        glyph_resident((0.0, 0.0)),
        glyph_resident((0.0, 12.0)),
    ]);
    // セグメントは ordinal 0/1 のみ。hover はそのどちらでもない 99（stale）。
    let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
    let out = decorate_canvas(
        input,
        &segments,
        Some(99),
        square_fill(),
        (0, 0, 0),
        &region,
        WritingMode::HorizontalTb,
        &bands_of(TEST_BAND, NO_OFFSET),
    );
    for (i, ordinal) in [(0usize, 0usize), (1, 1)] {
        let c = choice(&out.residents[i]);
        assert_eq!(c.hovered, None, "行 {i}: stale ordinal は hovered None");
        assert_eq!(
            c.highlight, None,
            "行 {i}: stale ordinal はどの行も塗らない"
        );
        // ただしセグメントは記録される（Choice 住人化は hover に依存しない）。
        assert_eq!(c.segments.len(), 1, "行 {i}: セグメントは記録される");
        assert_eq!(c.segments[0].ordinal, ordinal);
    }
}

// ── 縦書きの帯の純粋の検査（spec `vertical-choice-hover-band` タスク 3.1・設計 P1〜P3） ──
//
// 配置（`LayoutEngine`・`FixedMetrics`＝行ボックス 1.33em）→ `line_bands` → `decorate_canvas`／
// `derive_hit_rows` の本番と同じ道で、焼いた帯と当たりの帯をブロック軸（x）で突き合わせる。

/// 書字方向 `mode` で解いた `TextRegion`（validrect 10,20〜390,220）。
fn vregion(mode: WritingMode) -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(20), Some(220), Some(10), Some(390)),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    TextRegion::resolve(&model, (400, 224), mode)
}

/// 縦書きの配置・行ごとの帯・選択肢の注釈の組。
type Columns = (
    TextRegion,
    Vec<PositionedLine>,
    Vec<LineBand>,
    Vec<LineChoiceSegment>,
);

/// 「はい」「いいえ」を 1 列ずつ並べた選択肢 2 つ（em 28）の配置と、その帯・注釈。
fn two_choice_columns(mode: WritingMode) -> Columns {
    use crate::layout::{FixedMetrics, LayoutEngine, WrapPlan};
    use crate::state::{SpanKind, TextItem};
    let items = vec![
        TextItem::glyph("は"),
        TextItem::glyph("い"),
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::glyph("い"),
        TextItem::glyph("い"),
        TextItem::glyph("え"),
    ];
    let span = |ordinal, glyph_range| ChoiceSpan {
        kind: SpanKind::Choice,
        ordinal,
        id: String::new(),
        label: String::new(),
        references: vec![],
        glyph_range,
    };
    let region = vregion(mode);
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
    let segments = annotate_lines(&lines, &[span(0, 0..2), span(1, 2..5)]);
    (region, lines, bands, segments)
}

/// P1（要件 2.1・6.2）: 右から左・左から右で、`decorate_canvas` が Choice 住人へ焼いた
/// `band_extent`／`band_offset` から描画と同じ式（住人原点 ＋ 寄せ 〜 ＋ 厚み・`highlight_rect`）で
/// 組んだ帯と、`derive_hit_rows` の当たりの四角のブロック軸の範囲が数値まで同じ。右から左の寄せは負。
///
/// 較正: `line_bands` の縦書きの枝を横書きの式（丈 30・寄せ 4）へ戻すと、厚み・寄せの数で赤。
#[test]
fn vertical_baked_band_equals_hit_rect_block_range() {
    // em 28・s ＝ 28 × 0.33 ＝ 9.24: em の箱は列の箱の真ん中（既定の実装）で右から左 round(−4.62) ＝ −5・
    // 左から右 round(4.62) ＝ 5。
    for (mode, want_offset) in [
        (WritingMode::VerticalRl, -5.0),
        (WritingMode::VerticalLr, 5.0),
    ] {
        let (region, lines, bands, segments) = two_choice_columns(mode);
        assert_eq!(lines.len(), 2, "{mode:?}: 2 列に並ぶ");
        let base = ContentCanvas::from_layout(&lines, &region, mode);
        let out = decorate_canvas(
            base,
            &segments,
            Some(0),
            square_fill(),
            (0, 0, 0),
            &region,
            mode,
            &bands,
        );
        let rows = derive_hit_rows(&lines, &segments, mode, &region, &bands);
        assert_eq!(rows.len(), 2, "{mode:?}: 当たりの行 2 つ");
        for (seg, row) in segments.iter().zip(&rows) {
            let resident = &out.residents[seg.line_index];
            let c = choice(resident);
            assert_eq!(c.band_extent, 28.0, "{mode:?}: 焼いた厚みは列の em");
            assert_eq!(c.band_offset, want_offset, "{mode:?}: 焼いた寄せ");
            let x0 = resident.transform.offset().0 + c.band_offset;
            assert_eq!(
                (x0, x0 + c.band_extent),
                (row.rect.left, row.rect.right),
                "{mode:?}: 選択肢 {} の焼いた帯と当たりのブロック軸の範囲が違う",
                row.ordinal
            );
        }
        if mode == WritingMode::VerticalRl {
            assert!(
                bands.iter().all(|b| b.offset < 0.0),
                "右から左の寄せは負（帯は行矩形の左へ出る）: {bands:?}"
            );
        }
    }
}

/// P2（要件 1.4・2.3）: 同じ大きさの隣り合う列の帯は重ならず、間は行間の分（行送り − em）。
/// 帯の境の点は、半開区間（`x >= left && x < right`）でたかだか 1 列に当たる。
#[test]
fn vertical_adjacent_same_size_column_bands_do_not_overlap() {
    use crate::layout::{FixedMetrics, GlyphMetrics};
    let gap = FixedMetrics.line_pitch(28.0) - 28.0;
    assert!(gap > 0.0, "前提: 行間がある");
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let (region, lines, bands, segments) = two_choice_columns(mode);
        let rows = derive_hit_rows(&lines, &segments, mode, &region, &bands);
        // 右から左は 1 列目が右・左から右は 1 列目が左。左右の順に並べ直す。
        let (l, r) = if rows[0].rect.left < rows[1].rect.left {
            (rows[0].rect, rows[1].rect)
        } else {
            (rows[1].rect, rows[0].rect)
        };
        assert_eq!(
            r.left - l.right,
            gap,
            "{mode:?}: 隣の列の帯の間が行間 {gap} でない: 左 {l:?}・右 {r:?}"
        );
        // 両方の列が持つ行内の位置で、帯の辺の上と辺の際の点を見る。
        let y = l.top.max(r.top);
        for x in [
            l.left,
            l.right,
            r.left,
            r.right,
            l.right - 0.5,
            r.left - 0.5,
        ] {
            let hits = rows
                .iter()
                .filter(|row| {
                    let q = row.rect;
                    x >= q.left && x < q.right && y >= q.top && y < q.bottom
                })
                .count();
            assert!(hits <= 1, "{mode:?}: 点 ({x},{y}) が {hits} 列に当たる");
        }
    }
}

/// P3（要件 1.6）: `\f[height,40]` の字を含む列の帯の厚みは 40。隣の em 20 の列との帯の重なりの
/// 量（正＝重なる幅・負＝間の幅、image px）を数で固定する。字のインクは隣と重ならないので列送りは
/// 直さない（research.md 13.3 節）——帯だけが重なり得るのを、横書きの大きい行と同じ扱いで受け入れる。
#[test]
fn vertical_big_glyph_column_band_is_40_and_overlap_is_fixed() {
    use crate::cursor_tag::CursorWarnGuard;
    use crate::layout::{FixedMetrics, LayoutEngine, WrapPlan};
    use crate::look::{GlyphStyles, StyleTable, TextLook};
    use crate::state::TextItem;
    use areka_sakura::contract::ActorKey;
    let default = TextLook {
        height: 20.0,
        ..TextLook::ukadoc_default()
    };
    let mut table = StyleTable::default();
    let big = table.intern(
        &TextLook {
            height: 40.0,
            ..default.clone()
        },
        &default,
    );
    let mut items = Vec::new();
    let mut ids = Vec::new();
    for (word, id) in [
        ("雨だよ", StyleId::DEFAULT),
        ("晴れてる", big),
        ("曇ってる", StyleId::DEFAULT),
    ] {
        if !items.is_empty() {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        for ch in word.chars() {
            items.push(TextItem::glyph(&ch.to_string()));
            ids.push(id);
        }
    }
    // (向き, 大きい列と「雨だよ」の帯の重なり, 大きい列と「曇ってる」の帯の重なり)。
    // 右から左: 行矩形 x370..390・328..368・306..326 → 帯 367..387・321..361・303..323（寄せ −3・−7・−3）。
    // 左から右: 行矩形 x10..30・32..72・74..94 → 帯 13..33・39..79・77..97（寄せ 3・7・3）。
    // em の箱は列の箱の真ん中（既定の実装）なので 2 つの向きで同じ量: 大きい列の帯は後ろ隣（字の
    // 進む側）の列の帯へ 2 画素食い込み、前の列とは 6 画素の間が空く。
    for (mode, want_prev, want_next) in [
        (WritingMode::VerticalRl, -6.0, 2.0),
        (WritingMode::VerticalLr, -6.0, 2.0),
    ] {
        let region = vregion(mode);
        let lines = LayoutEngine::layout_styled(
            &items,
            ids.len(),
            &region,
            mode,
            20.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
            GlyphStyles {
                table: &table,
                ids: &ids,
                default: &default,
                current: &default,
            },
            &ActorKey::from("0"),
            &mut CursorWarnGuard::default(),
        );
        assert_eq!(lines.len(), 3, "{mode:?}: 3 列に並ぶ");
        let bands = line_bands(&lines, mode, &FixedMetrics);
        let extents: Vec<f32> = bands.iter().map(|b| b.extent).collect();
        assert_eq!(
            extents,
            [20.0, 40.0, 20.0],
            "{mode:?}: 帯の厚みは列ごとの em"
        );
        let span = |i: usize| {
            let x0 = lines[i].rect.left + bands[i].offset;
            (x0, x0 + bands[i].extent)
        };
        // 正なら重なる幅、負なら間の幅（image px）。
        let overlap = |a: (f32, f32), b: (f32, f32)| a.1.min(b.1) - a.0.max(b.0);
        assert_eq!(
            overlap(span(1), span(0)),
            want_prev,
            "{mode:?}: 雨だよとの帯の重なり"
        );
        assert_eq!(
            overlap(span(1), span(2)),
            want_next,
            "{mode:?}: 曇ってるとの帯の重なり"
        );
    }
}

/// P5（要件 4.2）: `vertical_em_box_left` の既定の実装（`FixedMetrics` は上書きしない）は em の箱を
/// 列の箱の真ん中に置く: 右から左 −s/2・左から右 ＋s/2・横書き 0（s ＝ 行の箱 − em）。比 1 の
/// 書体（行の箱 ＝ em）では 0。
#[test]
fn vertical_em_box_left_default_centres_the_em_box_in_the_column_box() {
    use crate::layout::{FixedMetrics, GlyphMetrics};
    for em in [10.0f32, 20.0, 28.0, 40.0] {
        let half = (FixedMetrics.line_box_height(em) - em) / 2.0;
        assert_eq!(
            FixedMetrics.vertical_em_box_left(em, WritingMode::VerticalRl),
            -half,
            "em {em}: 右から左"
        );
        assert_eq!(
            FixedMetrics.vertical_em_box_left(em, WritingMode::VerticalLr),
            half,
            "em {em}: 左から右"
        );
        assert_eq!(
            FixedMetrics.vertical_em_box_left(em, WritingMode::HorizontalTb),
            0.0,
            "em {em}: 横書き"
        );
    }
    // 字義の値（em 20 で ∓3.3）。
    assert!((FixedMetrics.vertical_em_box_left(20.0, WritingMode::VerticalRl) + 3.3).abs() < 1e-4);

    /// 行の箱 ＝ em の書体（ＭＳ ゴシックの形）。
    struct RatioOne;
    impl GlyphMetrics for RatioOne {
        fn advance(&self, text: &str, h: f32) -> f32 {
            FixedMetrics.advance(text, h)
        }
        fn line_pitch(&self, h: f32) -> f32 {
            FixedMetrics.line_pitch(h)
        }
        fn line_box_height(&self, h: f32) -> f32 {
            h
        }
    }
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        assert_eq!(RatioOne.vertical_em_box_left(28.0, mode), 0.0, "{mode:?}");
    }
}

/// P6（要件 4.2）: `vertical_em_box_left` だけを真ん中から外した偽の metrics（行の箱は `FixedMetrics`
/// と同じ）で、`line_bands` の縦書きの寄せがその値の丸め（−7.4 → −7・＋0.6 → 1）になる——行の箱の
/// 比からは出ない（`FixedMetrics` の真ん中なら em 28 で −5／5）。横書きの出力は `FixedMetrics` と同じ。
///
/// 較正: `line_bands` の縦書きの枝を 2.1 の割合の定数の式へ戻すと −4／5 で赤。
#[test]
fn vertical_line_bands_read_the_em_box_from_metrics() {
    use crate::layout::{FixedMetrics, GlyphMetrics};
    struct OffCentre;
    impl GlyphMetrics for OffCentre {
        fn advance(&self, text: &str, h: f32) -> f32 {
            FixedMetrics.advance(text, h)
        }
        fn line_pitch(&self, h: f32) -> f32 {
            FixedMetrics.line_pitch(h)
        }
        fn line_box_height(&self, h: f32) -> f32 {
            FixedMetrics.line_box_height(h)
        }
        fn vertical_em_box_left(&self, _h: f32, mode: WritingMode) -> f32 {
            match mode {
                WritingMode::VerticalRl => -7.4,
                WritingMode::VerticalLr => 0.6,
                WritingMode::HorizontalTb => 99.0,
            }
        }
    }
    for (mode, want) in [
        (WritingMode::VerticalRl, -7.0),
        (WritingMode::VerticalLr, 1.0),
    ] {
        let (_, lines, _, _) = two_choice_columns(mode);
        let bands = line_bands(&lines, mode, &OffCentre);
        assert!(
            bands.iter().all(|b| b.offset == want && b.extent == 28.0),
            "{mode:?}: 寄せは metrics の値の丸め {want}・厚みは em: {bands:?}"
        );
        assert_eq!(
            line_bands(&lines, WritingMode::HorizontalTb, &OffCentre),
            line_bands(&lines, WritingMode::HorizontalTb, &FixedMetrics),
            "{mode:?}: 横書きの帯は vertical_em_box_left を読まない"
        );
    }
}
