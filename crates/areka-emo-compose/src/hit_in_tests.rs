//! 領域の列を直に受ける形（`hit_region_in`・`hit_region_scaled_in`）の答えを固定する檻
//! （要件 4.2・2.8・タスク 5.1）。
//!
//! 6.2 で表示の入口は持ち込み済みの列（`SurfaceMaster.collisions` ではない列）で
//! 列を受ける形を呼ぶ。規則ごとの檻は `hit.rs` のテストにある。

use super::{RegionPriority, hit_region_in, hit_region_scaled_in};
use crate::plan_test_support::coll;
use crate::scale::ScaleRatio;

/// 列を受ける 2 つの形の答えをリテラルで固定する（重なり＝画家則・単独域・外・空の列・k=2 の縮約）。
#[test]
fn slice_forms_return_concrete_answers() {
    let list = vec![coll(1, 0, 0, 100, 100, "A"), coll(2, 50, 50, 150, 150, "B")];
    assert_eq!(
        hit_region_in(&list, 75, 75, RegionPriority::Painter),
        Some("B")
    );
    assert_eq!(
        hit_region_in(&list, 20, 20, RegionPriority::Painter),
        Some("A")
    );
    assert_eq!(
        hit_region_in(&list, 200, 200, RegionPriority::Painter),
        None
    );
    assert_eq!(hit_region_in(&[], 20, 20, RegionPriority::Painter), None);

    let k2 = ScaleRatio::new(192, 96).expect("k=2");
    let hit = hit_region_scaled_in(&list, 150, 150, k2, RegionPriority::Painter);
    assert_eq!(hit.region, Some("B"));
    assert_eq!(hit.surface_point, (75, 75));
    let hit = hit_region_scaled_in(&list, 40, 40, k2, RegionPriority::Painter);
    assert_eq!(hit.region, Some("A"));
    assert_eq!(hit.surface_point, (20, 20));
}
