//! 領域の列を直に受ける形（`hit_region_in`・`hit_region_scaled_in`）が、同じ列を持つ
//! サーフェスを受ける今の 2 関数（`hit_region`・`hit_region_scaled`）と同じ答えを返すことの檻
//! （要件 4.2・2.8・タスク 5.1）。
//!
//! 6.2 で表示の入口は持ち込み済みの列（`SurfaceMaster.collisions` ではない列）で
//! 列を受ける形を呼ぶ。列を受ける形が今の 2 関数と同じ規則（閉区間・画家則・反転矩形・
//! 拡大率の縮約）で答えることを、複数の列・点・拡大率で固定する。

use super::{
    RegionPriority, ScaledHit, hit_region, hit_region_in, hit_region_scaled, hit_region_scaled_in,
};
use crate::normalized::SurfaceMaster;
use crate::scale::ScaleRatio;
use areka_parsers::shell::{Collision, CollisionName};

fn coll(index: u32, left: i64, top: i64, right: i64, bottom: i64, name: &str) -> Collision {
    Collision {
        index,
        left,
        top,
        right,
        bottom,
        name: CollisionName::new(name.to_string()),
    }
}

fn master_with(collisions: Vec<Collision>) -> SurfaceMaster {
    SurfaceMaster {
        id: 1000,
        elements: Vec::new(),
        collisions,
        animations: Vec::new(),
    }
}

/// 列の候補: 空・単独・重なり（画家則）・同じ名前の重なり・反転矩形・離れた 2 つ。
fn lists() -> Vec<Vec<Collision>> {
    vec![
        Vec::new(),
        vec![coll(0, 93, 62, 271, 130, "Head")],
        vec![
            coll(0, 93, 62, 271, 130, "Head"),
            coll(1, 133, 270, 229, 326, "Bust"),
        ],
        vec![coll(1, 0, 0, 100, 100, "A"), coll(2, 50, 50, 150, 150, "B")],
        vec![
            coll(1, 0, 0, 100, 100, "Hand"),
            coll(2, 50, 50, 150, 150, "Hand"),
        ],
        vec![
            coll(0, 100, 100, 0, 0, "InvBoth"),
            coll(1, 0, 100, 100, 0, "InvY"),
            coll(2, 40, 40, 60, 60, "Ok"),
        ],
        vec![
            coll(0, 30, 30, 70, 70, "Torso"),
            coll(1, 200, 200, 260, 260, "Foot"),
        ],
    ]
}

/// 点の候補: 領域内・境界・外側 1px・重なり域・負値・窓外・i64 極値。
const POINTS: &[(i64, i64)] = &[
    (0, 0),
    (50, 50),
    (75, 75),
    (100, 100),
    (101, 101),
    (150, 150),
    (151, 151),
    (180, 96),
    (180, 300),
    (93, 62),
    (92, 61),
    (271, 130),
    (272, 131),
    (186, 124),
    (543, 261),
    (116, 77),
    (230, 230),
    (460, 460),
    (-7, -13),
    (5000, 5000),
    (i64::MIN, 0),
    (0, i64::MAX),
];

fn scales() -> Vec<ScaleRatio> {
    vec![
        ScaleRatio::ONE,
        ScaleRatio::new(192, 96).expect("k=2"),
        ScaleRatio::new(120, 96).expect("k=5/4"),
        ScaleRatio::new(72, 96).expect("k=3/4"),
    ]
}

#[test]
fn hit_region_in_matches_hit_region_for_same_list() {
    for list in lists() {
        let m = master_with(list.clone());
        for &(x, y) in POINTS {
            assert_eq!(
                hit_region_in(&list, x, y, RegionPriority::Painter),
                hit_region(&m, x, y, RegionPriority::Painter),
                "列を受ける形とサーフェスを受ける形が食い違う: list={list:?} point=({x},{y})"
            );
        }
    }
}

#[test]
fn hit_region_scaled_in_matches_hit_region_scaled_for_same_list() {
    for list in lists() {
        let m = master_with(list.clone());
        for k in scales() {
            for &(x, y) in POINTS {
                let got: ScaledHit<'_> =
                    hit_region_scaled_in(&list, x, y, k, RegionPriority::Painter);
                let want = hit_region_scaled(&m, x, y, k, RegionPriority::Painter);
                assert_eq!(
                    got, want,
                    "列を受ける拡大率つきの形が食い違う: list={list:?} k={k:?} point=({x},{y})"
                );
            }
        }
    }
}

/// 非空虚性: 上の一致が「両方 None」だけで成立していないことを、リテラルの答えで固定する。
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
