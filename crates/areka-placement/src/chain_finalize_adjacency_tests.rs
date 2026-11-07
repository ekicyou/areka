// =============================================================================
// 隣り合いの判定と記憶の印の語の檻（dpi-realign-remembered-chain 要件 1.2・2.1・2.2）
//
// 判定は World を持たない純関数ゆえ、物理 px の矩形を直接与える。
// 前の窓（n−1）を x=1000・幅 400 に固定し、後の窓（n）の位置で隙間を作る。
// =============================================================================

use super::*;

const PREV: ChainRect = ChainRect {
    x: 1000,
    y: 500,
    w: 400,
    h: 600,
};

/// 前の窓の左に、隙間 `gap`（負は重なり）で置いた幅 300 の窓。
fn left_of_prev(gap: i32) -> ChainRect {
    ChainRect {
        x: PREV.x - gap - 300,
        y: PREV.y,
        w: 300,
        h: PREV.h,
    }
}

/// 前の窓の右に、隙間 `gap`（負は重なり）で置いた幅 300 の窓。
fn right_of_prev(gap: i32) -> ChainRect {
    ChainRect {
        x: PREV.x + PREV.w + gap,
        y: PREV.y,
        w: 300,
        h: PREV.h,
    }
}

#[test]
fn gap_within_tolerance_is_adjacent_on_both_sides() {
    assert_eq!(ADJACENT_TOLERANCE_PX, 2);
    for gap in -2..=2 {
        assert_eq!(
            judge_adjacency(PREV, left_of_prev(gap)),
            Adjacency::Left { gap },
            "左の隙間 {gap}"
        );
        assert_eq!(
            judge_adjacency(PREV, right_of_prev(gap)),
            Adjacency::Right { gap },
            "右の隙間 {gap}"
        );
    }
}

#[test]
fn gap_beyond_tolerance_is_apart() {
    assert_eq!(
        judge_adjacency(PREV, left_of_prev(3)),
        Adjacency::Apart { gap: 3 }
    );
    assert_eq!(
        judge_adjacency(PREV, right_of_prev(3)),
        Adjacency::Apart { gap: 3 }
    );
}

#[test]
fn overlap_beyond_tolerance_is_overlapped() {
    assert_eq!(
        judge_adjacency(PREV, left_of_prev(-3)),
        Adjacency::Overlapped { gap: -3 }
    );
    assert_eq!(
        judge_adjacency(PREV, right_of_prev(-3)),
        Adjacency::Overlapped { gap: -3 }
    );
}

/// ぱすたの形（組の窓が同じ左上）は重なり（2.2）。幅が同じでも違っても横に並べない。
#[test]
fn same_top_left_is_overlapped() {
    let same = ChainRect { ..PREV };
    assert!(matches!(
        judge_adjacency(PREV, same),
        Adjacency::Overlapped { .. }
    ));
    let narrower = ChainRect { w: 200, ..PREV };
    assert!(matches!(
        judge_adjacency(PREV, narrower),
        Adjacency::Overlapped { .. }
    ));
}

/// 縦の範囲（[y, y+h)）が重ならない組は、横の隙間が 0 でも離れ（1.2）。
#[test]
fn vertically_disjoint_pair_is_apart() {
    let below = ChainRect {
        y: PREV.y + PREV.h,
        ..left_of_prev(0)
    };
    assert!(matches!(
        judge_adjacency(PREV, below),
        Adjacency::Apart { .. }
    ));
    let above = ChainRect {
        y: PREV.y - PREV.h,
        ..right_of_prev(0)
    };
    assert!(matches!(
        judge_adjacency(PREV, above),
        Adjacency::Apart { .. }
    ));
    // 1 px でも縦が重なれば横に並んでいる。
    let barely = ChainRect {
        y: PREV.y + PREV.h - 1,
        ..left_of_prev(0)
    };
    assert_eq!(judge_adjacency(PREV, barely), Adjacency::Left { gap: 0 });
}

/// 極端な座標でも panic しない（飽和演算）。
#[test]
fn extreme_coordinates_do_not_panic() {
    let far = ChainRect {
        x: i32::MAX,
        y: i32::MAX,
        w: i32::MAX,
        h: i32::MAX,
    };
    let near = ChainRect {
        x: i32::MIN,
        y: i32::MIN,
        w: i32::MAX,
        h: i32::MAX,
    };
    let _ = judge_adjacency(far, near);
    let _ = judge_adjacency(near, far);
}

#[test]
fn mark_words_round_trip() {
    for (mark, word) in [
        (AdjacencyMark::Left, "left"),
        (AdjacencyMark::Right, "right"),
        (AdjacencyMark::Separate, "none"),
    ] {
        assert_eq!(mark.as_str(), word);
        assert_eq!(AdjacencyMark::parse(word), Some(mark));
    }
}

#[test]
fn unknown_mark_word_is_none() {
    for word in ["", "Left", "LEFT", "separate", "apart", " left", "up"] {
        assert_eq!(AdjacencyMark::parse(word), None, "{word:?}");
    }
}

#[test]
fn mark_from_adjacency() {
    assert_eq!(
        AdjacencyMark::from_adjacency(Adjacency::Left { gap: 1 }),
        AdjacencyMark::Left
    );
    assert_eq!(
        AdjacencyMark::from_adjacency(Adjacency::Right { gap: -2 }),
        AdjacencyMark::Right
    );
    assert_eq!(
        AdjacencyMark::from_adjacency(Adjacency::Apart { gap: 3 }),
        AdjacencyMark::Separate
    );
    assert_eq!(
        AdjacencyMark::from_adjacency(Adjacency::Overlapped { gap: -3 }),
        AdjacencyMark::Separate
    );
    // 判定から印への写しを通して、離れ・重なりの組が印 none になる。
    assert_eq!(
        AdjacencyMark::from_adjacency(judge_adjacency(PREV, left_of_prev(0))).as_str(),
        "left"
    );
    assert_eq!(
        AdjacencyMark::from_adjacency(judge_adjacency(PREV, right_of_prev(5))).as_str(),
        "none"
    );
}
