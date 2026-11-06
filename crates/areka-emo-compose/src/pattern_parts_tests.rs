//! `PatternState` の部品の欄（要件 5.3, 5.4, 5.10）。
//!
//! 部品のコマは一番上のコマと別の欄で運ぶ。等しさは派生のままなので、`ComposeKey` の等価判定は
//! 部品のコマを自動で含む。ここでは空の状態と部品の欄の走査・置換を固定する（挿入順・全部消した後・
//! 一番上と混ざらないことは `pattern_cell_tests.rs`）。

use super::*;
use crate::plan_test_support::frame;

/// 部品の欄を一度も使わない状態は空。
#[test]
fn empty_parts_are_empty() {
    // 既定値が空であることは pattern.rs の `default_is_empty`。
    let a = PatternState::default();
    assert_eq!(a.part(1400).count(), 0);
    assert_eq!(a.part_get(1400, 0), None);
}

/// 部品のコマだけを持つ状態は空でなく、部品のコマを持たない状態と区別される。
#[test]
fn part_frame_makes_state_non_empty_and_distinct() {
    let mut a = PatternState::default();
    a.set_part(1400, 0, frame(1401, 0, 0));
    assert!(!a.is_empty());
    assert_ne!(a, PatternState::default());
    // 一番上の欄は空のまま。
    assert_eq!(a.iter().count(), 0);
}

/// `part` は animation の番号の昇順で走査し、同じ鍵への二度目の `set_part` は置換する。
#[test]
fn part_iterates_ascending_and_set_part_replaces() {
    let mut s = PatternState::default();
    s.set_part(1400, 9, frame(1, 0, 0));
    s.set_part(1400, 2, frame(2, 0, 0));
    s.set_part(1400, 5, frame(3, 0, 0));
    s.set_part(1400, 5, frame(4, 0, 0));
    let got: Vec<(u32, u32)> = s.part(1400).map(|(id, f)| (id, f.surface_id)).collect();
    assert_eq!(got, vec![(2, 2), (5, 4), (9, 1)]);
    assert_eq!(s.part_get(1400, 5).map(|f| f.surface_id), Some(4));
    assert_eq!(s.part(1401).count(), 0);
}
