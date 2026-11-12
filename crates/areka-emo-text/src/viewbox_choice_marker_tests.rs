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

/// 4 行（見出し・選択肢 A・選択肢 B・脚注）の選択肢 2 行の hover を `[A, B]` で与えた住人列。
fn hover_rows(hovered: [Option<usize>; 2]) -> Vec<Resident> {
    vec![
        glyph_resident("見出し", 0.0, &[]),
        choice_resident("えらぶА", 13.0, &[], hovered[0]),
        choice_resident("えらぶБ", 26.0, &[], hovered[1]),
        glyph_resident("脚注", 39.0, &[]),
    ]
}

/// hover の付与（None → Some）・解除（Some → None）・切替（A → B）のどれでも、`derive_dirty` の
/// 出力は choice_marker が変わった行の矩形だけ＝出力の矩形の数が変わった行の数と厳密一致し、
/// 全域ダーティにならない（R4.4/7.5）。行 i の矩形は {0, 13i, 40, 10} をガード 1px 広げたもの。
/// あわせて非 Choice 行の `choice_marker` は常に 0・Choice 行は `hovered.map_or(0, |o| o+1)`。
#[test]
fn choice_marker_hover_change_derive_dirty_matches_only_the_changed_lines() {
    let row1 = phys(0, 12, 41, 12);
    let row2 = phys(0, 25, 41, 12);
    let markers = |rs: &[Resident]| -> Vec<u32> {
        rs.iter()
            .map(|r| line_fingerprint(r, WritingMode::HorizontalTb).choice_marker)
            .collect()
    };
    // (名前, 前の hover, 後の hover, 後の印, 変わった行, 矩形)。
    let cases = [
        (
            "付与",
            [None, None],
            [Some(0), None],
            [0, 1, 0, 0],
            vec![1],
            vec![row1],
        ),
        (
            "解除",
            [Some(0), None],
            [None, None],
            [0, 0, 0, 0],
            vec![1],
            vec![row1],
        ),
        (
            "切替",
            [Some(0), None],
            [None, Some(0)],
            [0, 0, 1, 0],
            vec![1, 2],
            vec![row1, row2],
        ),
    ];
    for (name, before, after, after_markers, lines, rects) in cases {
        assert_eq!(markers(&hover_rows(after)), after_markers, "{name}: 後の印");
        let (changed, dirty, draw) = derive_dirty_for_hover(hover_rows(before), hover_rows(after));
        assert_eq!(changed, lines, "{name}: choice_marker が変わった行");
        assert_eq!(
            dirty, rects,
            "{name}: 変わった行の矩形だけ・全域ダーティでない"
        );
        assert_eq!(draw, lines, "{name}: 描画対象も変わった行だけ");
    }
}
