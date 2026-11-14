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

// ---- 平行移動の写し（5.1〜5.4・条件 S1〜S9） ----

/// `id`・`shape`・種類・親・部分木の終わりを決めた項（`exact`・`origin` は `b` から）。
fn node(
    id: u64,
    shape: u64,
    kind: EntryKind,
    parent: Option<u32>,
    end: u32,
    b: PxRect,
) -> DamageEntry {
    DamageEntry {
        shape,
        parent,
        end,
        ..entry(id, kind, b)
    }
}

/// 項を `(dx, dy)` だけ平行移動した今の項（`id` は位置で変わる＝新しい `id`）。
fn shifted(e: &DamageEntry, id: u64, dx: f32, dy: f32) -> DamageEntry {
    let mut n = e.clone();
    n.id = id;
    n.bounds = e.bounds.offset(dx.floor() as i32, dy.floor() as i32);
    if dx.fract() != 0.0 || dy.fract() != 0.0 {
        n.bounds.right += 1;
        n.bounds.bottom += 1;
    }
    n.exact = [
        e.exact[0] + dx,
        e.exact[1] + dy,
        e.exact[2] + dx,
        e.exact[3] + dy,
    ];
    n.origin = [e.origin[0] + dx, e.origin[1] + dy];
    n
}

/// 不透明度の層 [0] の中に、外の件 [1]・矩形の切り抜き E [2]（`e`）・E の中の件 2 つ [3][4]・外の件 [5]。
/// 今の列では E の中の件だけが `(dx, dy)` ずれ、外の件 [5] は描き換わる（id 5 → 6）。
fn clip_case(e: PxRect, dx: f32, dy: f32) -> (Vec<DamageEntry>, Vec<DamageEntry>) {
    let all = rect(0, 0, 100, 80);
    let layer = node(100, 100, EntryKind::Opacity, None, 6, all);
    let out1 = node(1, 1, item_kind(), Some(0), 2, rect(80, 0, 100, 10));
    let clip = node(2, 2, EntryKind::ClipRect, Some(0), 5, e);
    let c1 = node(
        20,
        20,
        item_kind(),
        Some(2),
        4,
        rect(e.left + 5, e.top + 5, e.left + 25, e.top + 15),
    );
    let c2 = node(
        21,
        21,
        item_kind(),
        Some(2),
        5,
        rect(e.left + 5, e.top + 20, e.left + 25, e.top + 30),
    );
    let out5 = node(5, 5, item_kind(), Some(0), 6, rect(70, 70, 90, 80));
    let out6 = node(6, 6, item_kind(), Some(0), 6, rect(70, 70, 90, 80));
    let prev = vec![
        layer.clone(),
        out1.clone(),
        clip.clone(),
        c1.clone(),
        c2.clone(),
        out5,
    ];
    let now = vec![
        layer,
        out1,
        clip,
        shifted(&c1, 30, dx, dy),
        shifted(&c2, 31, dx, dy),
        out6,
    ];
    (prev, now)
}

fn item_kind() -> EntryKind {
    EntryKind::Draw { unbounded: false }
}

/// 写しを使わず普通の範囲へ回った（E の中の前の件の範囲が描く矩形に入る）。
fn assert_falls_back(i: &PlanInput<'_>) {
    let p = partial(plan(i));
    assert!(p.shifts.is_empty(), "写しを受け入れた: {p:?}");
    assert_eq!((p.shift_area, p.exposed_area), (0, 0));
    assert_eq!(p.rects.len(), 1, "{p:?}");
    let sub = rect(15, 15, 35, 40);
    assert_eq!(p.rects[0].intersect(sub), sub, "{p:?}");
}

#[test]
fn clip_rect_candidate_accepts_shift_with_band_and_keeps_normal_range_outside() {
    let (prev, now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    let p = partial(plan(&input(&prev, &now)));
    // D = (10,10)-(55,60)・元はその 5 右・右の帯 (55,10)-(60,60)・外の描き換え (70,70)-(90,80)
    assert_eq!(
        p.shifts,
        vec![RegionShift {
            src: rect(15, 10, 60, 60),
            dst: (10, 10),
        }]
    );
    assert_eq!(p.rects, vec![rect(55, 10, 60, 60), rect(70, 70, 90, 80)]);
    assert_eq!(p.shift_area, 45 * 50);
    assert_eq!(p.exposed_area, 5 * 50);
    assert_eq!(p.area, 5 * 50 + 20 * 10);
    assert_eq!(p.present_rect, rect(10, 10, 90, 80));
    // 帯にも外の矩形にも交わる件は描き換わった外の件だけ（E の中の件は数えない）
    assert_eq!(p.items_redrawn, 1);
}

#[test]
fn whole_surface_candidate_accepts_shift_without_root_list() {
    let a = item(10, rect(10, 10, 30, 20));
    let b = item(11, rect(10, 30, 30, 40));
    let c = item(12, rect(10, 60, 30, 70));
    let prev = [a.clone(), b.clone(), c.clone()];
    // 全部が 20 上へ・下の帯へ新しい件が入る（前なら描画面の外 (10,85)-(30,95)）
    let now = [
        shifted(&a, 40, 0.0, -20.0),
        shifted(&b, 41, 0.0, -20.0),
        shifted(&c, 42, 0.0, -20.0),
        item(43, rect(10, 65, 30, 75)),
    ];
    let p = partial(plan(&input(&prev, &now)));
    assert_eq!(
        p.shifts,
        vec![RegionShift {
            src: rect(0, 20, 100, 80),
            dst: (0, 0),
        }]
    );
    assert_eq!(p.rects, vec![rect(0, 60, 100, 80)]);
    assert_eq!(p.shift_area, 100 * 60);
    assert_eq!(p.exposed_area, 100 * 20);
    assert_eq!(p.area, 100 * 20);
    assert_eq!(p.present_rect, rect(0, 0, 100, 80));
    assert_eq!(p.items_redrawn, 1);

    // S1: 根の命令の列を持てば描画面の全面の候補も写さない
    let i = PlanInput {
        root_present: true,
        ..input(&prev, &now)
    };
    let p = partial(plan(&i));
    assert!(p.shifts.is_empty());
    assert_eq!(p.rects, vec![rect(10, 10, 30, 75)]);
}

#[test]
fn s1_root_list_present_falls_back() {
    let (prev, now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    assert_falls_back(&PlanInput {
        root_present: true,
        ..input(&prev, &now)
    });
}

#[test]
fn s2_geometry_clip_in_parent_chain_falls_back() {
    let (mut prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    prev[0].kind = EntryKind::ClipGeometry;
    now[0].kind = EntryKind::ClipGeometry;
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s3_reordered_pairs_fall_back() {
    let (prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    now.swap(3, 4);
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s4_non_integer_or_uneven_shift_falls_back() {
    let (prev, now) = clip_case(rect(10, 10, 60, 60), -5.5, 0.0);
    assert_falls_back(&input(&prev, &now));
    let (prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    now[4] = shifted(&prev[4], 31, -4.0, 0.0);
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s5_linear_or_size_difference_falls_back() {
    let (prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    now[4].linear[3] = 1.5;
    assert_falls_back(&input(&prev, &now));
    let (prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    now[4].exact[2] += 1.0;
    now[4].bounds.right += 1;
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s6_non_source_over_item_falls_back() {
    let (mut prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    prev[3].kind = EntryKind::Draw { unbounded: true };
    now[3].kind = EntryKind::Draw { unbounded: true };
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s7_unpaired_item_staying_inside_the_clip_falls_back() {
    // 前にだけ在る件がずらしても切り抜きの中に残る
    let (mut prev, now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    prev[2].end = 6;
    prev[4].end = 5;
    prev.insert(
        5,
        node(22, 22, item_kind(), Some(2), 6, rect(30, 45, 50, 55)),
    );
    prev[0].end = 7;
    prev[6].end = 7;
    assert_falls_back(&input(&prev, &now));
    // 今にだけ在る件が露出した帯の外（写しの先）に入る
    let (prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    now[2].end = 6;
    now.insert(
        5,
        node(32, 32, item_kind(), Some(2), 6, rect(30, 45, 50, 55)),
    );
    now[0].end = 7;
    now[6].end = 7;
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s8_item_outside_subtree_overlapping_the_clip_falls_back() {
    let (mut prev, mut now) = clip_case(rect(10, 10, 60, 60), -5.0, 0.0);
    prev[1].bounds = rect(50, 0, 70, 20);
    now[1].bounds = rect(50, 0, 70, 20);
    assert_falls_back(&input(&prev, &now));
}

#[test]
fn s9_empty_destination_falls_back() {
    // 切り抜きは描画面の左へはみ出し、描画面の中の幅 10 より大きく動く（幅 60 の中では整数）
    let (prev, now) = clip_case(rect(-50, 10, 10, 60), -20.0, 0.0);
    let p = partial(plan(&input(&prev, &now)));
    assert!(p.shifts.is_empty(), "{p:?}");
    // E の中の件は描画面の外にしか描かないので、普通の範囲は外の描き換えだけ
    assert_eq!(p.rects, vec![rect(70, 70, 90, 80)]);
}
