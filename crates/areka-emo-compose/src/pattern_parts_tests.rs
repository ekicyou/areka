//! `PatternState` の部品の欄（task 2.2・要件 5.3, 5.4, 5.10）。
//!
//! 部品のコマは一番上のコマと別の欄で運ぶ。等しさは派生のままなので、`ComposeKey` の等価判定は
//! 部品のコマを自動で含む。ここでは欄の等しさの安定（空・挿入順・全部消した後）と、同じ番号の
//! サーフェスの一番上のコマと部品のコマが混ざらないことを固定する。

use super::*;

/// テスト用のコマ（`Overlay`・オフセットで区別できるようにする）。
fn frame(surface_id: u32, x: i64) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x,
        y: 0,
    }
}

/// 部品の欄を一度も使わない状態どうしは等しく、空。
#[test]
fn empty_parts_are_equal_and_empty() {
    let a = PatternState::default();
    let b = PatternState::default();
    assert_eq!(a, b);
    assert!(a.is_empty());
    assert_eq!(a.part(1400).count(), 0);
    assert_eq!(a.part_get(1400, 0), None);
}

/// 部品のコマだけを持つ状態は空でなく、部品のコマを持たない状態と区別される。
#[test]
fn part_frame_makes_state_non_empty_and_distinct() {
    let mut a = PatternState::default();
    a.set_part(1400, 0, frame(1401, 0));
    assert!(!a.is_empty());
    assert_ne!(a, PatternState::default());
    // 一番上の欄は空のまま。
    assert_eq!(a.iter().count(), 0);
}

/// 部品・animation のどちらの順で入れても、同じコマの集合なら等しい。
#[test]
fn parts_equal_regardless_of_insertion_order() {
    let mut a = PatternState::default();
    a.set_part(1400, 0, frame(1401, 1));
    a.set_part(1400, 3, frame(1402, 2));
    a.set_part(10, 1, frame(11, 3));
    a.set(5, frame(20, 4));

    let mut b = PatternState::default();
    b.set(5, frame(20, 4));
    b.set_part(10, 1, frame(11, 3));
    b.set_part(1400, 3, frame(1402, 2));
    b.set_part(1400, 0, frame(1401, 1));

    assert_eq!(a, b);
}

/// `part` は animation の番号の昇順で走査し、同じ鍵への二度目の `set_part` は置換する。
#[test]
fn part_iterates_ascending_and_set_part_replaces() {
    let mut s = PatternState::default();
    s.set_part(1400, 9, frame(1, 0));
    s.set_part(1400, 2, frame(2, 0));
    s.set_part(1400, 5, frame(3, 0));
    s.set_part(1400, 5, frame(4, 0));
    let got: Vec<(u32, u32)> = s.part(1400).map(|(id, f)| (id, f.surface_id)).collect();
    assert_eq!(got, vec![(2, 2), (5, 4), (9, 1)]);
    assert_eq!(s.part_get(1400, 5).map(|f| f.surface_id), Some(4));
    assert_eq!(s.part(1401).count(), 0);
}

/// 全部消した後は、部品の欄を一度も使わない状態と等しい（空の内側の表を残さない）。
#[test]
fn clear_parts_equals_never_used() {
    let mut a = PatternState::default();
    a.set(7, frame(70, 0));
    a.set_part(1400, 0, frame(1401, 0));
    a.set_part(10, 1, frame(11, 0));
    a.clear_parts();

    let mut b = PatternState::default();
    b.set(7, frame(70, 0));

    assert_eq!(a, b);
    // 一番上の欄は残る。
    assert_eq!(a.get(7).map(|f| f.surface_id), Some(70));

    let mut c = PatternState::default();
    c.set_part(1400, 0, frame(1401, 0));
    c.clear_parts();
    assert_eq!(c, PatternState::default());
    assert!(c.is_empty());
}

/// 同じ番号のサーフェスの一番上のコマと部品のコマは混ざらない（要件 5.10）。
///
/// animation 0 を一番上の欄と「部品 0」の欄の両方へ置いても、互いの読み書きは相手に届かない。
#[test]
fn top_level_and_part_frames_do_not_mix() {
    let mut s = PatternState::default();
    s.set(0, frame(100, 1));
    s.set_part(0, 0, frame(200, 2));

    assert_eq!(s.get(0).map(|f| f.x), Some(1));
    assert_eq!(s.part_get(0, 0).map(|f| f.x), Some(2));
    assert_eq!(s.iter().count(), 1);
    assert_eq!(s.part(0).count(), 1);

    // 一番上を消しても部品は残る。
    s.remove(0);
    assert_eq!(s.get(0), None);
    assert_eq!(s.part_get(0, 0).map(|f| f.x), Some(2));
    assert!(!s.is_empty());

    // 部品を消しても一番上は残る。
    let mut t = PatternState::default();
    t.set(0, frame(100, 1));
    t.set_part(0, 0, frame(200, 2));
    t.clear_parts();
    assert_eq!(t.get(0).map(|f| f.x), Some(1));
    assert_eq!(t.part_get(0, 0), None);

    // 一番上に同じコマを置いた状態と、部品に置いた状態は等しくない。
    let mut top = PatternState::default();
    top.set(0, frame(100, 0));
    let mut part = PatternState::default();
    part.set_part(0, 0, frame(100, 0));
    assert_ne!(top, part);
}
