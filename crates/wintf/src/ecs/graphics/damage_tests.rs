//! 合成の計画の純粋なテスト（COM 無し・要件 7.2, 7.4）。ここでは全部の合成の理由の綴り（3.4）。

use super::damage::*;

#[test]
fn full_reason_spellings_are_the_seven_record_words() {
    let all = [
        (FullReason::First, "first"),
        (FullReason::Recreated, "recreated"),
        (FullReason::RootList, "root_list"),
        (FullReason::Scale, "scale"),
        (FullReason::NoPrev, "no_prev"),
        (FullReason::Undecidable, "undecidable"),
        (FullReason::CoversSurface, "covers_surface"),
    ];
    for (reason, word) in all {
        assert_eq!(reason.as_str(), word);
    }
}

#[test]
fn damage_record_target() {
    assert_eq!(DAMAGE_TARGET, "wintf::damage");
}

// ---- 描画面 1 枚の計画（3.1〜3.4・2.1・2.5・2.8・4.1） ----

use super::visual::{DamageEntry, EntryKind, PxRect};

const SURFACE: (u32, u32) = (100, 80);

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PxRect {
    PxRect {
        left,
        top,
        right,
        bottom,
    }
}

fn entry(id: u64, kind: EntryKind, b: PxRect) -> DamageEntry {
    DamageEntry {
        id,
        shape: id,
        kind,
        parent: None,
        end: 0,
        bounds: b,
        exact: [b.left as f32, b.top as f32, b.right as f32, b.bottom as f32],
        linear: [1.0, 0.0, 0.0, 1.0],
        origin: [b.left as f32, b.top as f32],
    }
}

fn item(id: u64, b: PxRect) -> DamageEntry {
    entry(id, EntryKind::Draw { unbounded: false }, b)
}

/// 前の合成と同じ根・拡大で写し終えた、木の在る描画面の入力。
fn input<'a>(prev: &'a [DamageEntry], now: &'a [DamageEntry]) -> PlanInput<'a> {
    PlanInput {
        fresh: None,
        same_root: true,
        same_scale: true,
        root_present: false,
        prev: Some(prev),
        now: Some(now),
        presented: true,
        surface: SURFACE,
    }
}

fn partial(p: ComposePlan) -> PartialPlan {
    match p {
        ComposePlan::Partial(p) => p,
        other => panic!("範囲だけでない: {other:?}"),
    }
}

#[test]
fn fresh_container_is_full_first_or_recreated_before_every_other_check() {
    let a = [item(1, rect(0, 0, 10, 10))];
    for reason in [FullReason::First, FullReason::Recreated] {
        let i = PlanInput {
            fresh: Some(reason),
            same_root: false,
            same_scale: false,
            prev: None,
            now: None,
            ..input(&a, &a)
        };
        assert_eq!(plan(&i), ComposePlan::Full(reason));
    }
}

#[test]
fn root_list_change_is_full_root_list_before_scale() {
    let a = [item(1, rect(0, 0, 10, 10))];
    let i = PlanInput {
        same_root: false,
        same_scale: false,
        ..input(&a, &a)
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::RootList));
}

#[test]
fn scale_change_is_full_scale() {
    let a = [item(1, rect(0, 0, 10, 10))];
    let i = PlanInput {
        same_scale: false,
        ..input(&a, &a)
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::Scale));
}

#[test]
fn undecidable_now_is_full_undecidable_even_without_prev() {
    let a = [item(1, rect(0, 0, 10, 10))];
    let i = PlanInput {
        now: None,
        ..input(&a, &a)
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::Undecidable));
    let i = PlanInput {
        now: None,
        prev: None,
        ..input(&a, &a)
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::Undecidable));
}

#[test]
fn missing_prev_is_full_no_prev() {
    let a = [item(1, rect(0, 0, 10, 10))];
    let i = PlanInput {
        prev: None,
        ..input(&a, &a)
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::NoPrev));
}

#[test]
fn damage_covering_surface_is_full_covers_surface() {
    let prev = [item(1, rect(0, 0, 10, 10))];
    // 描画面より大きい足された項は描画面で切ると全面に等しい
    let now = [item(1, rect(0, 0, 10, 10)), item(2, rect(-5, -5, 200, 200))];
    assert_eq!(
        plan(&input(&prev, &now)),
        ComposePlan::Full(FullReason::CoversSurface)
    );
}

#[test]
fn unchanged_is_skip_when_presented_and_present_only_when_not() {
    let a = [
        item(1, rect(0, 0, 10, 10)),
        entry(2, EntryKind::ClipRect, rect(5, 5, 50, 50)),
    ];
    assert_eq!(plan(&input(&a, &a)), ComposePlan::Skip);
    let i = PlanInput {
        presented: false,
        ..input(&a, &a)
    };
    assert_eq!(plan(&i), ComposePlan::PresentOnly);
}

#[test]
fn partial_is_bounding_rect_of_removed_prev_and_added_moved_now() {
    let a = item(1, rect(10, 10, 20, 20));
    let b = item(2, rect(30, 5, 40, 15));
    let c = item(3, rect(60, 60, 70, 70));
    let gone = item(4, rect(2, 40, 8, 50));
    let prev = [a.clone(), b.clone(), c.clone(), gone];
    // a は最長増加部分列に入らず動いた（前 [a b c] → 今 [b c a]）。新しい項は描画面の右下をはみ出す
    let new = item(5, rect(90, 70, 120, 90));
    let outside_rect = item(6, rect(0, 0, 1, 1));
    let now_a = item(1, rect(10, 10, 20, 20));
    // 動かなかった切り抜きの層は描画面の全面を覆うが、変わっていないので範囲に入らない
    let clip = entry(7, EntryKind::ClipRect, rect(0, 0, 100, 80));
    let now = [
        outside_rect.clone(),
        b.clone(),
        c.clone(),
        now_a,
        new,
        clip.clone(),
    ];
    let prev = [vec![outside_rect], prev.to_vec(), vec![clip]].concat();
    let p = partial(plan(&input(&prev, &now)));
    // 消えた (2,40)-(8,50)・動いた (10,10)-(20,20)・足された (90,70)-(120,90) を描画面で切る
    let want = rect(2, 10, 100, 80);
    assert_eq!(p.rects, vec![want]);
    assert!(p.shifts.is_empty());
    assert_eq!(p.area, want.area());
    assert_eq!(p.present_rect, want);
    assert_eq!(p.shift_area, 0);
    assert_eq!(p.exposed_area, 0);
    // 矩形と交わる件は b・c・a・新しい項（(0,0)-(1,1) の件と切り抜きの層は数えない）
    assert_eq!(p.items_redrawn, 4);
}

#[test]
fn change_only_outside_surface_is_skip_or_present_only() {
    let prev = [
        item(1, rect(0, 0, 10, 10)),
        item(2, rect(-30, -30, -10, -10)),
    ];
    let now = [item(1, rect(0, 0, 10, 10)), item(3, rect(150, 10, 160, 20))];
    assert_eq!(plan(&input(&prev, &now)), ComposePlan::Skip);
    let i = PlanInput {
        presented: false,
        ..input(&prev, &now)
    };
    assert_eq!(plan(&i), ComposePlan::PresentOnly);
}

#[test]
fn surface_without_tree_skips_when_same_and_is_full_when_root_scale_or_container_differ() {
    let none: [DamageEntry; 0] = [];
    let base = PlanInput {
        root_present: true,
        ..input(&none, &none)
    };
    assert_eq!(plan(&base), ComposePlan::Skip);
    let i = PlanInput {
        presented: false,
        ..base
    };
    assert_eq!(plan(&i), ComposePlan::PresentOnly);
    let i = PlanInput {
        same_root: false,
        ..base
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::RootList));
    let i = PlanInput {
        same_scale: false,
        ..base
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::Scale));
    let i = PlanInput {
        fresh: Some(FullReason::Recreated),
        ..base
    };
    assert_eq!(plan(&i), ComposePlan::Full(FullReason::Recreated));
}
