//! 比べの項の純粋なテスト（COM 無し・要件 7.1, 7.2）。ここでは物理ピクセルの整数の矩形の演算。

use super::diff::*;

fn r(left: i32, top: i32, right: i32, bottom: i32) -> PxRect {
    PxRect {
        left,
        top,
        right,
        bottom,
    }
}

#[test]
fn empty_is_zero_width_or_height_or_inverted() {
    assert!(PxRect::default().is_empty());
    assert!(r(5, 0, 5, 10).is_empty());
    assert!(r(0, 5, 10, 5).is_empty());
    assert!(r(10, 0, 0, 10).is_empty());
    assert!(!r(0, 0, 1, 1).is_empty());
}

#[test]
fn intersect_overlap_touch_and_disjoint() {
    assert_eq!(r(0, 0, 10, 10).intersect(r(5, 3, 20, 7)), r(5, 3, 10, 7));
    // 辺が接するだけ・離れている・空との交わりは空（空の形は 1 つ）
    assert_eq!(
        r(0, 0, 10, 10).intersect(r(10, 0, 20, 10)),
        PxRect::default()
    );
    assert_eq!(
        r(0, 0, 10, 10).intersect(r(30, 30, 40, 40)),
        PxRect::default()
    );
    assert_eq!(r(0, 0, 10, 10).intersect(r(3, 3, 3, 8)), PxRect::default());
}

#[test]
fn union_is_bounding_box_and_ignores_empty() {
    assert_eq!(r(0, 0, 10, 10).union(r(20, -5, 30, 5)), r(0, -5, 30, 10));
    assert_eq!(r(1, 2, 3, 4).union(PxRect::default()), r(1, 2, 3, 4));
    assert_eq!(r(100, 100, 100, 200).union(r(1, 2, 3, 4)), r(1, 2, 3, 4));
    assert!(PxRect::default().union(r(7, 7, 7, 9)).is_empty());
}

#[test]
fn area_counts_pixels_and_empty_is_zero() {
    assert_eq!(r(0, 0, 10, 20).area(), 200);
    assert_eq!(r(-5, -5, 5, 5).area(), 100);
    assert_eq!(r(10, 0, 0, 10).area(), 0);
    // i32 の端から端でも u64 に収まる
    assert_eq!(r(i32::MIN, 0, i32::MAX, 2).area(), 2 * (u32::MAX as u64));
}

#[test]
fn offset_moves_all_edges() {
    assert_eq!(r(0, 0, 10, 10).offset(3, -4), r(3, -4, 13, 6));
    assert_eq!(r(0, 0, 10, 10).offset(0, 0), r(0, 0, 10, 10));
    assert_eq!(r(0, 0, 10, 10).offset(-3, 4).offset(3, -4), r(0, 0, 10, 10));
}
