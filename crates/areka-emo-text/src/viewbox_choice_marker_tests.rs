use super::test_support::{choice_resident, glyph_resident, phys, window};
use super::{DirtyRect, ScrollPlanner, line_fingerprint};
use crate::canvas::{ContentCanvas, Resident};
use crate::region::ScaleContract;
use crate::writing::WritingMode;

// ── 6.1 R4.4: 行指紋の hover 印（choice_marker）——hover 切替は当該行の指紋だけを変える ──

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
) -> (Vec<usize>, Vec<DirtyRect>, Vec<usize>) {
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
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], None), // hover なし
        choice_resident("えらぶБ", 26.0, &[], None),
        glyph_resident("脚注", 39.0, &[]),
    ];
    let after = vec![
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], Some(0)), // hover 付与（None → Some(0)）
        choice_resident("えらぶБ", 26.0, &[], None),
        glyph_resident("脚注", 39.0, &[]),
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
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], Some(0)), // hover あり
        choice_resident("えらぶБ", 26.0, &[], None),
        glyph_resident("脚注", 39.0, &[]),
    ];
    let after = vec![
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], None), // hover 解除（Some(0) → None）
        choice_resident("えらぶБ", 26.0, &[], None),
        glyph_resident("脚注", 39.0, &[]),
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
/// あわせて非 Choice 行の `choice_marker` は常に 0・Choice 行は `hovered.map_or(0, |o| o+1)`。
#[test]
fn choice_marker_hover_switch_derive_dirty_matches_only_the_two_switched_lines() {
    let before = vec![
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], Some(0)), // A に hover
        choice_resident("えらぶБ", 26.0, &[], None),
        glyph_resident("脚注", 39.0, &[]),
    ];
    let after = vec![
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], None), // hover を A→B へ切替
        choice_resident("えらぶБ", 26.0, &[], Some(0)),
        glyph_resident("脚注", 39.0, &[]),
    ];
    let markers = |rs: &[Resident]| -> Vec<u32> {
        rs.iter()
            .map(|r| line_fingerprint(r, WritingMode::HorizontalTb).choice_marker)
            .collect()
    };
    assert_eq!(
        markers(&before),
        vec![0, 1, 0, 0],
        "hover ordinal 0 → 1・他は 0"
    );
    assert_eq!(markers(&after), vec![0, 0, 1, 0], "hover を B へ切替");
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
