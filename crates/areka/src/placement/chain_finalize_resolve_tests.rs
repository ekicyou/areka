// =============================================================================
// 組ごとの決めからの解きと記憶の印の決めの檻（dpi-realign-remembered-chain 要件 1.3・1.5・6.3・6.4・8.4）
//
// 解きは World を持たない純関数ゆえ、スコープ昇順の入力を直接与える。
// =============================================================================

use super::*;

fn slot(scope: usize, current_x: i32, width: i32, decision: PairDecision) -> ChainSlot {
    ChainSlot {
        scope,
        current_x,
        width,
        decision,
    }
}

/// 先頭（本体）の決めは読まないので何を入れてもよい。
const BODY: PairDecision = PairDecision::Keep(KeepReason::ExplicitlyPlaced);

/// 指示を当てた後の x の列（スコープ番号 = 添字）。
fn apply(slots: &[ChainSlot], moves: &[ChainMove]) -> Vec<i32> {
    let mut xs: Vec<i32> = slots.iter().map(|s| s.current_x).collect();
    for m in moves {
        xs[m.scope] = m.new_x;
    }
    xs
}

/// 左へ付ける: new_x = 前の x − 自分の幅（1.3）。
#[test]
fn join_left_places_flush_on_the_left() {
    let slots = [
        slot(0, 1000, 382, BODY),
        slot(1, 500, 336, PairDecision::JoinLeft),
    ];
    let moves = resolve_chain(&slots);
    assert_eq!(
        moves,
        vec![ChainMove {
            scope: 1,
            new_x: 664
        }]
    );
}

/// 右へ付ける: new_x = 前の x + 前の幅（1.3・左右を保つ）。
#[test]
fn join_right_places_flush_on_the_right() {
    let slots = [
        slot(0, 1000, 382, BODY),
        slot(1, 1800, 336, PairDecision::JoinRight),
    ];
    let moves = resolve_chain(&slots);
    assert_eq!(
        moves,
        vec![ChainMove {
            scope: 1,
            new_x: 1382
        }]
    );
}

/// 既に隣り合っていれば指示を出さない（6.4 の後段）。
#[test]
fn already_flush_yields_no_moves() {
    let slots = [
        slot(0, 1000, 382, BODY),
        slot(1, 664, 336, PairDecision::JoinLeft),
        slot(
            2,
            1382,
            300,
            PairDecision::Keep(KeepReason::NoRememberedMark),
        ),
    ];
    assert!(resolve_chain(&slots).is_empty());
}

/// 3 体の鎖: 2 の基準は 1 の解いた後の x（1.5・8.4）。
#[test]
fn three_chain_uses_previous_solved_x_as_base() {
    let slots = [
        slot(0, 1000, 382, BODY),
        slot(1, 236, 336, PairDecision::JoinLeft),
        slot(2, 0, 300, PairDecision::JoinLeft),
    ];
    let moves = resolve_chain(&slots);
    let xs = apply(&slots, &moves);
    assert_eq!(xs, vec![1000, 664, 364], "664 = 1000−336・364 = 664−300");
}

/// 3 体の鎖で右・左が混ざっても前の結果が次の基準になる。
#[test]
fn three_chain_mixed_directions() {
    let slots = [
        slot(0, 1000, 400, BODY),
        slot(1, 1500, 300, PairDecision::JoinRight),
        slot(2, 2000, 200, PairDecision::JoinRight),
    ];
    let xs = apply(&slots, &resolve_chain(&slots));
    assert_eq!(xs, vec![1000, 1400, 1700]);
}

/// 動かさない窓は今の x のまま、その x が次の基準になる（y は扱わない）。
#[test]
fn kept_window_current_x_is_next_base() {
    let slots = [
        slot(0, 1000, 400, BODY),
        slot(
            1,
            200,
            300,
            PairDecision::Keep(KeepReason::Apart { gap: 500 }),
        ),
        slot(2, 0, 100, PairDecision::JoinLeft),
    ];
    let moves = resolve_chain(&slots);
    assert!(moves.iter().all(|m| m.scope != 1), "動かさない窓に指示なし");
    assert_eq!(apply(&slots, &moves), vec![1000, 200, 100]);
}

/// 幅が 0 以下の窓は動かさず、今の x を次の基準にする。
#[test]
fn non_positive_width_is_kept_and_is_next_base() {
    let slots = [
        slot(0, 1000, 400, BODY),
        slot(1, 700, 0, PairDecision::JoinLeft),
        slot(2, 0, 200, PairDecision::JoinLeft),
    ];
    let moves = resolve_chain(&slots);
    assert_eq!(apply(&slots, &moves), vec![1000, 700, 500]);
}

/// 右へ付けるとき前の窓の幅が 0 以下なら式が立たないので動かさない。
#[test]
fn join_right_with_non_positive_previous_width_is_kept() {
    let slots = [
        slot(0, 1000, 0, BODY),
        slot(1, 1500, 300, PairDecision::JoinRight),
    ];
    assert!(resolve_chain(&slots).is_empty());
}

/// 空・1 体は動かす相手が居ない。極端な値でも panic しない。
#[test]
fn empty_single_and_extremes() {
    assert!(resolve_chain(&[]).is_empty());
    assert!(resolve_chain(&[slot(0, 1000, 400, BODY)]).is_empty());
    let moves = resolve_chain(&[
        slot(0, i32::MAX, i32::MAX, BODY),
        slot(1, 0, 10, PairDecision::JoinRight),
    ]);
    assert_eq!(moves[0].new_x, i32::MAX, "飽和して上限に張り付く");
}

/// 記憶から戻したスコープの決め: 印の 3 通り＋印なし（6.3・6.4）。
#[test]
fn remembered_decision_covers_three_marks_and_none() {
    assert_eq!(
        remembered_decision(Some(AdjacencyMark::Left)),
        PairDecision::JoinLeft
    );
    assert_eq!(
        remembered_decision(Some(AdjacencyMark::Right)),
        PairDecision::JoinRight
    );
    assert_eq!(
        remembered_decision(Some(AdjacencyMark::Separate)),
        PairDecision::Keep(KeepReason::RememberedSeparate)
    );
    assert_eq!(
        remembered_decision(None),
        PairDecision::Keep(KeepReason::NoRememberedMark)
    );
}

/// 動かさない理由の固定語（ログの語・7.1 の判別に使う）。
#[test]
fn keep_reason_fixed_words() {
    let cases = [
        (KeepReason::Apart { gap: 9 }, "apart"),
        (KeepReason::Overlapped { gap: -9 }, "overlapped"),
        (KeepReason::StickyGroup { scope: 1 }, "sticky-group"),
        (KeepReason::Dragging { scope: 1 }, "dragging"),
        (KeepReason::ScriptMovedAfterArm { scope: 1 }, "script-moved"),
        (KeepReason::Unreadable { scope: 1 }, "unreadable"),
        (KeepReason::ExplicitlyPlaced, "explicitly-placed"),
        (KeepReason::RememberedSeparate, "remembered-separate"),
        (KeepReason::NoRememberedMark, "no-remembered-mark"),
        (KeepReason::NonPositiveWidth, "non-positive-width"),
    ];
    for (reason, word) in cases {
        assert_eq!(reason.as_str(), word);
    }
}

/// 判断の語: 付ける向きは固定語、動かさないは理由の語（7.1）。
#[test]
fn pair_decision_words() {
    assert_eq!(PairDecision::JoinLeft.as_str(), "adjacent-left");
    assert_eq!(PairDecision::JoinRight.as_str(), "adjacent-right");
    assert_eq!(
        PairDecision::Keep(KeepReason::StickyGroup { scope: 1 }).as_str(),
        "sticky-group"
    );
}

// -----------------------------------------------------------------------------
// 起動の並べ直しの組ごとの決め（design BootRemembered の 5 行・要件 6.3〜6.5）
// -----------------------------------------------------------------------------

fn state(default_x: Option<i32>) -> ScopeChainState {
    ScopeChainState {
        scope: 1,
        current_x: 500,
        width: 336,
        default_x,
    }
}

const MARKS: [Option<AdjacencyMark>; 4] = [
    Some(AdjacencyMark::Left),
    Some(AdjacencyMark::Right),
    Some(AdjacencyMark::Separate),
    None,
];

/// 記憶に位置が無く既定のままなら左へ付ける。組・覚えた印は読まない（1 行目）。
#[test]
fn boot_untouched_scope_joins_left() {
    for mark in MARKS {
        for sticky in [false, true] {
            assert_eq!(
                boot_pair_decision(&state(Some(500)), false, sticky, mark),
                PairDecision::JoinLeft
            );
        }
    }
}

/// 記憶に位置が無く、既定から動かされたか台本の印があれば動かさない（2 行目）。
#[test]
fn boot_moved_or_script_placed_scope_without_memory_is_kept() {
    let explicit = PairDecision::Keep(KeepReason::ExplicitlyPlaced);
    for mark in MARKS {
        assert_eq!(
            boot_pair_decision(&state(Some(800)), false, false, mark),
            explicit
        );
        assert_eq!(
            boot_pair_decision(&state(Some(500)), true, false, mark),
            explicit
        );
        assert_eq!(
            boot_pair_decision(&state(Some(800)), true, true, mark),
            explicit
        );
    }
}

/// 記憶から戻した窓に台本の印があれば、組や印より先に動かさない（3 行目）。
#[test]
fn boot_restored_scope_with_script_marker_is_explicitly_placed() {
    for mark in MARKS {
        for sticky in [false, true] {
            assert_eq!(
                boot_pair_decision(&state(None), true, sticky, mark),
                PairDecision::Keep(KeepReason::ExplicitlyPlaced)
            );
        }
    }
}

/// 記憶から戻した窓が組に入っていれば、印より先に動かさない（4 行目）。
#[test]
fn boot_restored_scope_in_sticky_group_is_kept() {
    for mark in MARKS {
        assert_eq!(
            boot_pair_decision(&state(None), false, true, mark),
            PairDecision::Keep(KeepReason::StickyGroup { scope: 1 })
        );
    }
}

/// 記憶から戻した、上のどれでもない窓は覚えた印で決める（5 行目）。
#[test]
fn boot_restored_scope_follows_the_remembered_mark() {
    let cases = [
        (Some(AdjacencyMark::Left), PairDecision::JoinLeft),
        (Some(AdjacencyMark::Right), PairDecision::JoinRight),
        (
            Some(AdjacencyMark::Separate),
            PairDecision::Keep(KeepReason::RememberedSeparate),
        ),
        (None, PairDecision::Keep(KeepReason::NoRememberedMark)),
    ];
    for (mark, expected) in cases {
        assert_eq!(
            boot_pair_decision(&state(None), false, false, mark),
            expected
        );
        assert_eq!(remembered_decision(mark), expected);
    }
}
