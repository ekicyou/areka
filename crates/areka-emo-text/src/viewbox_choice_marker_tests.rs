use super::test_support::{phys, window};
use super::{PhysicalRect, ScrollPlanner, line_fingerprint};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, GlyphRunContent, RegionTransform, Resident,
    ResidentContent, TextEffects,
};
use crate::layout::PositionedGlyph;
use crate::region::ScaleContract;
use crate::writing::WritingMode;

// ── 6.1 R4.4: 行指紋の hover 印（choice_marker）——hover 切替は当該行の指紋だけを変える ──

/// 行内 n グリフの GlyphRunContent（inline_pos 連番・全角 advance 10）。
fn run_content(text: &str) -> GlyphRunContent {
    let glyphs = text
        .chars()
        .enumerate()
        .map(|(i, ch)| PositionedGlyph {
            ch,
            inline_pos: i as f32 * 10.0,
            advance: 10.0,
        })
        .collect();
    GlyphRunContent {
        glyphs,
        size: (text.chars().count() as f32 * 10.0, 10.0),
    }
}

/// GlyphRun 住人（非 Choice・ブロック軸位置 dy）。
fn glyph_resident(text: &str, dy: f32) -> Resident {
    Resident {
        content: ResidentContent::GlyphRun(run_content(text)),
        transform: RegionTransform::translation(0.0, dy),
        effects: TextEffects::default(),
    }
}

/// Choice 住人（hover ordinal 注入・ブロック軸位置 dy・素描画は GlyphRun と同一）。
fn choice_resident(text: &str, dy: f32, hovered: Option<usize>) -> Resident {
    let run = run_content(text);
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
        }),
        transform: RegionTransform::translation(0.0, dy),
        effects: TextEffects::default(),
    }
}

/// hover を選択肢行 A→B へ切り替えると、変化した 2 行（choice_marker が動いた行）の指紋のみが
/// 差分となり、非 Choice 行を含む他行の指紋は不変（choice_marker が hover を指紋へ運ぶ・R4.4）。
/// あわせて非 Choice 行の `choice_marker` は常に 0・Choice 行は `hovered.map_or(0, |o| o+1)`。
#[test]
fn choice_marker_hover_switch_dirties_only_two_changed_lines() {
    let mode = WritingMode::HorizontalTb;
    // 4 行: [GlyphRun, Choice A, Choice B, GlyphRun]（ブロック軸 0/13/26/39）。
    let before = [
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, Some(0)), // A に hover
        choice_resident("えらぶБ", 26.0, None),
        glyph_resident("脚注", 39.0),
    ];
    let after = [
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, None),
        choice_resident("えらぶБ", 26.0, Some(0)), // hover を B へ切替
        glyph_resident("脚注", 39.0),
    ];
    let fp_before: Vec<_> = before.iter().map(|r| line_fingerprint(r, mode)).collect();
    let fp_after: Vec<_> = after.iter().map(|r| line_fingerprint(r, mode)).collect();

    // 変化したのは選択肢 2 行（index 1,2）のみ・他行は不変。
    assert_eq!(fp_before[0], fp_after[0], "非 Choice 見出し行は不変");
    assert_ne!(
        fp_before[1], fp_after[1],
        "hover が外れた選択肢行は指紋差分"
    );
    assert_ne!(
        fp_before[2], fp_after[2],
        "hover が乗った選択肢行は指紋差分"
    );
    assert_eq!(fp_before[3], fp_after[3], "非 Choice 脚注行は不変");

    // 非 Choice 行の choice_marker は常に 0・Choice 行は hovered.map_or(0, |o| o+1)。
    assert_eq!(fp_before[0].choice_marker, 0, "非 Choice は marker 0");
    assert_eq!(fp_before[1].choice_marker, 1, "hover ordinal 0 → 1");
    assert_eq!(fp_before[2].choice_marker, 0, "非 hover Choice → 0");
    assert_eq!(fp_before[3].choice_marker, 0, "非 Choice は marker 0");
    assert_eq!(fp_after[1].choice_marker, 0, "hover 解除 → 0");
    assert_eq!(fp_after[2].choice_marker, 1, "hover 付与 → 1");
}

/// before→after の hover 変化から `derive_dirty` を回し、(choice_marker が動いた行 index,
/// dirty 矩形, draw_lines) を返す共通口（task 6.2）。横書き・k=1.0・blit=0（スクロールなし）
/// ゆえ露出帯は生じず、dirty は**変化行の矩形のみ**になる——これにより「`derive_dirty` の
/// 出力行数が choice_marker の変化した行数と厳密一致し、全域ダーティにならない」（R4.4/7.5 の
/// Observable）を fingerprint レベルでなく**ダーティ導出の出力**レベルで検証できる。
///
/// 変化行判定は前後 canvas の行指紋差分（`committed_lines`＝derive_dirty が内部で使う唯一の
/// 根拠）で行い、hover 差分が確かに choice_marker 経由で指紋へ運ばれていることも同時に押さえる。
fn derive_dirty_for_hover(
    before: Vec<Resident>,
    after: Vec<Resident>,
) -> (Vec<usize>, Vec<PhysicalRect>, Vec<usize>) {
    let mode = WritingMode::HorizontalTb;
    let contract = ScaleContract::new(1.0, None);
    let surface = (400u32, 224u32);
    let before_canvas = ContentCanvas {
        residents: before,
        size: (400.0, 224.0),
    };
    let after_canvas = ContentCanvas {
        residents: after,
        size: (400.0, 224.0),
    };
    // prev_lines＝前回確定の行指紋（derive_dirty の変化行検出の唯一の根拠）。
    let prev = ScrollPlanner::committed_lines(&before_canvas, mode);
    let next = ScrollPlanner::committed_lines(&after_canvas, mode);
    // choice_marker（を含む指紋）が動いた行 index。hover 変化はこの集合に一致するのが期待。
    let changed: Vec<usize> = (0..prev.len()).filter(|&i| prev[i] != next[i]).collect();
    // blit=0（スクロールなし）＝露出帯なし・dirty は変化行の矩形のみ。
    let (dirty, draw) = ScrollPlanner::derive_dirty(
        &after_canvas,
        &window(0, 0.0),
        mode,
        &contract,
        (0, 0),
        surface,
        &prev,
    );
    (changed, dirty, draw)
}

/// hover SET（None → Some(o)）: `derive_dirty` の出力は付与された当該 Choice 行の矩形のみ＝
/// 出力行数が choice_marker の変化した行数（1）と厳密一致し、全域ダーティにならない（R4.4/7.5）。
#[test]
fn choice_marker_hover_set_derive_dirty_matches_only_the_set_line() {
    let before = vec![
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, None), // hover なし
        choice_resident("えらぶБ", 26.0, None),
        glyph_resident("脚注", 39.0),
    ];
    let after = vec![
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, Some(0)), // hover 付与（None → Some(0)）
        choice_resident("えらぶБ", 26.0, None),
        glyph_resident("脚注", 39.0),
    ];
    let (changed, dirty, draw) = derive_dirty_for_hover(before, after);
    // 変化した choice_marker は index 1 の 1 行のみ。
    assert_eq!(changed, vec![1], "変化した choice_marker 行は index 1 のみ");
    // Observable: derive_dirty の出力行数 == 変化した choice_marker 行数（1）。
    assert_eq!(dirty.len(), changed.len(), "dirty 行数 == 変化行数（1）");
    // 当該行矩形のみ（{0,13,40,10} をガード 1px 拡張）・全域（面全 {0,0,400,224}）でない。
    assert_eq!(
        dirty,
        vec![phys(0, 12, 41, 12)],
        "付与行 index 1 の矩形のみ・全域ダーティでない"
    );
    assert_eq!(draw, vec![1], "描画対象も付与行のみ");
}

/// hover CLEAR（Some(o) → None）: `derive_dirty` の出力は解除された当該 Choice 行の矩形のみ＝
/// 出力行数が choice_marker の変化した行数（1）と厳密一致し、全域ダーティにならない（R4.4/7.5）。
#[test]
fn choice_marker_hover_clear_derive_dirty_matches_only_the_cleared_line() {
    let before = vec![
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, Some(0)), // hover あり
        choice_resident("えらぶБ", 26.0, None),
        glyph_resident("脚注", 39.0),
    ];
    let after = vec![
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, None), // hover 解除（Some(0) → None）
        choice_resident("えらぶБ", 26.0, None),
        glyph_resident("脚注", 39.0),
    ];
    let (changed, dirty, draw) = derive_dirty_for_hover(before, after);
    assert_eq!(changed, vec![1], "変化した choice_marker 行は index 1 のみ");
    assert_eq!(dirty.len(), changed.len(), "dirty 行数 == 変化行数（1）");
    assert_eq!(
        dirty,
        vec![phys(0, 12, 41, 12)],
        "解除行 index 1 の矩形のみ・全域ダーティでない"
    );
    assert_eq!(draw, vec![1], "描画対象も解除行のみ");
}

/// hover SWITCH（Some(a) → Some(b)・異なる行）: `derive_dirty` の出力は旧・新の 2 行の矩形のみ＝
/// 出力行数が choice_marker の変化した行数（2）と厳密一致し、全域ダーティにならない（R4.4/7.5）。
/// 6.1 の同名テストは fingerprint 差分までを押さえるが、本テストは derive_dirty の**出力**を押さえる。
#[test]
fn choice_marker_hover_switch_derive_dirty_matches_only_the_two_switched_lines() {
    let before = vec![
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, Some(0)), // A に hover
        choice_resident("えらぶБ", 26.0, None),
        glyph_resident("脚注", 39.0),
    ];
    let after = vec![
        glyph_resident("見出し", 0.0),
        choice_resident("えらぶА", 13.0, None), // hover を A→B へ切替
        choice_resident("えらぶБ", 26.0, Some(0)),
        glyph_resident("脚注", 39.0),
    ];
    let (changed, dirty, draw) = derive_dirty_for_hover(before, after);
    // 変化したのは選択肢 2 行（旧 hover 行 index 1・新 hover 行 index 2）。
    assert_eq!(
        changed,
        vec![1, 2],
        "変化した choice_marker 行は index 1,2 の 2 行"
    );
    assert_eq!(dirty.len(), changed.len(), "dirty 行数 == 変化行数（2）");
    // 旧行（{0,13}）・新行（{0,26}）の 2 矩形のみ・全域ダーティでない。
    assert_eq!(
        dirty,
        vec![phys(0, 12, 41, 12), phys(0, 25, 41, 12)],
        "切替 2 行の矩形のみ・全域ダーティでない"
    );
    assert_eq!(draw, vec![1, 2], "描画対象も切替 2 行のみ");
}
