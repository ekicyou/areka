// ── タスク 5.4: canvas 装飾（decorate_canvas）——GlyphRun 住人 → Choice 住人置換 ──
//
// design.md「純粋層 / ChoicePure」・要件 1.1（Choice cue→行 resident 描画）/4.2（cursor.* 指定→
// SquareFill）/4.3（未指定→Invert 反転）/4.5（描画実行の一点写像＝焼込済み純データ）。
// 座標系: LineChoiceSegment.inline_range は絶対 image px・ChoiceRowSegment.inline_range は
// resident-local（from_layout がグリフから行内原点を差し引くのと同一原点で変換）。

use super::*;
use crate::canvas::{GlyphRunContent, RegionTransform, Resident, TextEffects};
use crate::layout::PositionedGlyph;
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
                ch: 'あ',
                inline_pos: 0.0,
                advance: 10.0,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
    );
    for i in [0usize, 1] {
        assert_eq!(
            choice(&out.residents[i]).band_extent,
            TEST_BAND,
            "住人 {i}: hover 有無に依らず帯を焼き込む（hover 解除フレームのダーティ帯にも要る）"
        );
    }
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
            TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
        TEST_BAND,
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
