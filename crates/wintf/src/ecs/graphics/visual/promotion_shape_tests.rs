//! 自動で上げてよい形の見分けと自分の描画面の矩形の決めの純粋なテスト。
//! 走査の列を手で組み、拡大 1 ではにじみの余白が各辺 2（`ceil(2 × 1)`）・拡大 1.25 と 1.5 では 3 になる。

use super::promotion::Phase;
use super::promotion_shape::*;
use super::world_visual::{AncestorAcc, ClipClass, ScanEntry, ScanKind, TreeScan, WorldVisual};
use crate::numerics::*;
use bevy_ecs::prelude::Entity;
use windows_numerics::Matrix3x2;

fn ent(n: u32) -> Entity {
    Entity::from_raw_u32(n).unwrap()
}

/// 深さ `depth`・世界の位置 `(l, t)` で中身 `[l, r) × [t, b)`（DIP）の見える節。
pub(super) fn node(depth: usize, l: f32, t: f32, r: f32, b: f32) -> ScanEntry {
    let mut w = WorldVisual::from_world_mat(Matrix3x2::translation(l, t));
    w.content_world_aabb = Aabb::new(l, t, r, b);
    w.subtree_world_aabb = w.content_world_aabb;
    ScanEntry {
        entity: Entity::PLACEHOLDER,
        depth,
        parent: None,
        end: 0,
        kind: ScanKind::Node { phase: Phase::Node },
        visible: true,
        world: w,
        updated: false,
        own_clip: ClipClass::None,
        source_over: true,
        ancestors: AncestorAcc::ROOT,
    }
}

pub(super) fn owner(depth: usize, l: f32, t: f32, r: f32, b: f32) -> ScanEntry {
    ScanEntry {
        kind: ScanKind::Owner {
            parent_world: Matrix3x2::identity(),
        },
        ..node(depth, l, t, r, b)
    }
}

pub(super) fn phased(mut e: ScanEntry, phase: Phase) -> ScanEntry {
    e.kind = ScanKind::Node { phase };
    e
}

/// 深さの並びから親と部分木の終わりを埋め、実体の番号を振る。
pub(super) fn scan(mut entries: Vec<ScanEntry>) -> TreeScan {
    let n = entries.len();
    for i in 0..n {
        entries[i].entity = ent(i as u32 + 1);
        let d = entries[i].depth;
        entries[i].parent = (0..i).rev().find(|&j| entries[j].depth < d);
        entries[i].end = (i + 1..n).find(|&j| entries[j].depth <= d).unwrap_or(n);
    }
    TreeScan { entries }
}

fn input(scan: &TreeScan, idx: usize, scale: f32) -> ShapeInput<'_> {
    ShapeInput {
        scan,
        idx,
        scale,
        host_px: (100, 100),
    }
}

fn origin(px: (i32, i32), dip: (f32, f32), size_px: (u32, u32)) -> Origin {
    Origin { px, dip, size_px }
}

#[test]
fn plain_node_is_eligible_with_bleed_margin_rect() {
    let s = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0)]);
    assert_eq!(
        check_shape(&input(&s, 0, 1.0)),
        Ok(origin((8, 8), (8.0, 8.0), (14, 14)))
    );
}

#[test]
fn subtree_extent_includes_visible_descendants_only() {
    let mut hidden = node(2, 60.0, 60.0, 90.0, 90.0);
    hidden.visible = false;
    let s = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        node(2, 30.0, 30.0, 40.0, 40.0),
        hidden,
    ]);
    assert_eq!(
        check_shape(&input(&s, 0, 1.0)),
        Ok(origin((8, 8), (8.0, 8.0), (34, 34)))
    );
}

#[test]
fn later_content_overlap_margin_just_before_and_after() {
    // 候補の範囲 [8, 22)。後の兄弟の左 24.0 → 22（触れるだけ）、23.5 → 21（重なる）。
    let touch = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        node(1, 24.0, 10.0, 30.0, 20.0),
    ]);
    assert!(check_shape(&input(&touch, 0, 1.0)).is_ok());
    let hit = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        node(1, 23.5, 10.0, 30.0, 20.0),
    ]);
    assert_eq!(
        check_shape(&input(&hit, 0, 1.0)),
        Err(Ineligible::LaterContentOverlap)
    );
}

#[test]
fn later_content_deeper_or_shallower_counts_earlier_does_not() {
    // 前に描く兄弟は重なっても数えない。
    let earlier = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        node(1, 12.0, 12.0, 18.0, 18.0),
    ]);
    assert!(check_shape(&input(&earlier, 1, 1.0)).is_ok());
    // 後の兄弟の子（深い）と、親が閉じた後の節（浅い）は数える。
    let deeper = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        node(1, 50.0, 50.0, 60.0, 60.0),
        node(2, 12.0, 12.0, 18.0, 18.0),
    ]);
    assert_eq!(
        check_shape(&input(&deeper, 0, 1.0)),
        Err(Ineligible::LaterContentOverlap)
    );
    let shallower = scan(vec![
        node(1, 50.0, 50.0, 60.0, 60.0),
        node(2, 10.0, 10.0, 20.0, 20.0),
        node(1, 12.0, 12.0, 18.0, 18.0),
    ]);
    assert_eq!(
        check_shape(&input(&shallower, 1, 1.0)),
        Err(Ineligible::LaterContentOverlap)
    );
}

#[test]
fn later_non_source_over_far_away_counts_as_overlap() {
    // SOURCE_OVER でない中身は D2D が軸に沿う切り抜き（無ければ描画面の全体）まで写すので、離れていても重なる。
    let mut far = node(1, 70.0, 70.0, 80.0, 80.0);
    let plain = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0), far.clone()]);
    assert!(check_shape(&input(&plain, 0, 1.0)).is_ok());
    far.source_over = false;
    let blend = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0), far.clone()]);
    assert_eq!(
        check_shape(&input(&blend, 0, 1.0)),
        Err(Ineligible::LaterContentOverlap)
    );
    // ほかの自動の持ち主の部分の中は、その持ち主の描画面に描くので数えない。
    let inside = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        phased(node(1, 70.0, 70.0, 80.0, 80.0), Phase::Active),
        ScanEntry { depth: 2, ..far },
    ]);
    assert!(check_shape(&input(&inside, 0, 1.0)).is_ok());
}

#[test]
fn later_content_inside_other_auto_owner_or_manual_owner_is_not_counted() {
    for phase in [Phase::Preparing, Phase::Active] {
        let s = scan(vec![
            node(1, 10.0, 10.0, 20.0, 20.0),
            phased(node(1, 12.0, 12.0, 18.0, 18.0), phase),
            node(2, 12.0, 12.0, 18.0, 18.0),
        ]);
        assert!(check_shape(&input(&s, 0, 1.0)).is_ok(), "{phase:?}");
    }
    let manual = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        owner(1, 12.0, 12.0, 18.0, 18.0),
    ]);
    assert!(check_shape(&input(&manual, 0, 1.0)).is_ok());
    // 自動の持ち主の部分が閉じた後の節は数える。
    let after = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        phased(node(1, 50.0, 50.0, 60.0, 60.0), Phase::Active),
        node(2, 50.0, 50.0, 60.0, 60.0),
        node(1, 12.0, 12.0, 18.0, 18.0),
    ]);
    assert_eq!(
        check_shape(&input(&after, 0, 1.0)),
        Err(Ineligible::LaterContentOverlap)
    );
}

#[test]
fn owner_inside_subtree_is_ineligible() {
    let s = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        owner(2, 12.0, 12.0, 18.0, 18.0),
    ]);
    assert_eq!(
        check_shape(&input(&s, 0, 1.0)),
        Err(Ineligible::ContainsOwner)
    );
}

#[test]
fn ancestor_layer_or_rotated_rect_clip_is_ineligible() {
    let mut layered = node(1, 10.0, 10.0, 20.0, 20.0);
    layered.ancestors.layer = true;
    assert_eq!(
        check_shape(&input(&scan(vec![layered]), 0, 1.0)),
        Err(Ineligible::AncestorLayer)
    );
    let mut rotated = node(1, 10.0, 10.0, 20.0, 20.0);
    rotated.ancestors.rect_clip_rotated = true;
    assert_eq!(
        check_shape(&input(&scan(vec![rotated]), 0, 1.0)),
        Err(Ineligible::AncestorLayer)
    );
}

#[test]
fn non_source_over_in_visible_subtree_is_ineligible() {
    let mut blend = node(2, 12.0, 12.0, 18.0, 18.0);
    blend.source_over = false;
    let s = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0), blend.clone()]);
    assert_eq!(
        check_shape(&input(&s, 0, 1.0)),
        Err(Ineligible::NonSourceOver)
    );
    blend.visible = false;
    let hidden = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0), blend]);
    assert!(check_shape(&input(&hidden, 0, 1.0)).is_ok());
}

#[test]
fn host_surface_fit_margin_just_before_and_after() {
    // 左上: 2.0 → 0（収まる）、1.5 → -1（はみ出す）。右下: 98.0 → 100、98.5 → 101。
    for (l, r, ok) in [
        (2.0, 50.0, true),
        (1.5, 50.0, false),
        (50.0, 98.0, true),
        (50.0, 98.5, false),
    ] {
        let s = scan(vec![node(1, l, l, r, r)]);
        let got = check_shape(&input(&s, 0, 1.0));
        if ok {
            assert!(got.is_ok(), "{l} {r}");
        } else {
            assert_eq!(got, Err(Ineligible::OutOfBounds), "{l} {r}");
        }
    }
}

#[test]
fn ancestor_rect_clip_fit_margin_just_before_and_after() {
    // 範囲の左上は 8。祖先の切り抜きの左上 8.0 と 7.5 は収まり、8.5（端数）は収まらない。
    for (c, ok) in [(8.0, true), (7.5, true), (8.5, false)] {
        let mut e = node(1, 10.0, 10.0, 20.0, 20.0);
        e.ancestors.rect_clip = Some(Aabb::new(c, c, 60.0, 60.0));
        let got = check_shape(&input(&scan(vec![e]), 0, 1.0));
        if ok {
            assert!(got.is_ok(), "{c}");
        } else {
            assert_eq!(got, Err(Ineligible::OutOfBounds), "{c}");
        }
    }
}

#[test]
fn rotation_scale_and_own_clip_are_not_skip_reasons() {
    let mut e = node(1, 30.0, 30.0, 50.0, 50.0);
    // 45 度の回転と 1.5 倍（中身の外接矩形は埋める係が出したものを使う）。
    let k = std::f32::consts::FRAC_1_SQRT_2 * 1.5;
    e.world = WorldVisual {
        world_mat: Matrix3x2 {
            M11: k,
            M12: k,
            M21: -k,
            M22: k,
            M31: 40.0,
            M32: 30.0,
        },
        axis_aligned: false,
        ..e.world
    };
    e.own_clip = ClipClass::Rounded;
    let mut child = node(2, 35.0, 35.0, 45.0, 45.0);
    child.own_clip = ClipClass::Geometry;
    let got = check_shape(&input(&scan(vec![e, child]), 0, 1.0));
    // 候補の余白は列の長さ 1.5 で ceil(3) = 3、子は 2。範囲 [27, 53)。
    assert_eq!(got, Ok(origin((27, 27), (27.0, 27.0), (26, 26))));
}

#[test]
fn hidden_or_owner_candidate_is_hidden() {
    let mut h = node(1, 10.0, 10.0, 20.0, 20.0);
    h.visible = false;
    assert_eq!(
        check_shape(&input(&scan(vec![h]), 0, 1.0)),
        Err(Ineligible::Hidden)
    );
    let o = scan(vec![owner(1, 10.0, 10.0, 20.0, 20.0)]);
    assert_eq!(check_shape(&input(&o, 0, 1.0)), Err(Ineligible::Hidden));
    assert_eq!(check_shape(&input(&o, 5, 1.0)), Err(Ineligible::Hidden));
}

#[test]
fn surface_rect_negative_coords_cut_by_host() {
    let s = scan(vec![node(1, -10.0, -10.0, 20.0, 20.0)]);
    assert_eq!(
        surface_rect(&input(&s, 0, 1.0)),
        Some(origin((0, 0), (0.0, 0.0), (22, 22)))
    );
    // 拡大 1.25: 物理 [-10, 10) × [5, 15)、余白 3 → [-13, 13) × [2, 18) を描画面で切る。
    let s = scan(vec![node(1, -8.0, 4.0, 8.0, 12.0)]);
    assert_eq!(
        surface_rect(&input(&s, 0, 1.25)),
        Some(origin((0, 2), (0.0, 1.6), (13, 16)))
    );
    // 右下も描画面で切る。
    let s = scan(vec![node(1, 90.0, 90.0, 120.0, 120.0)]);
    assert_eq!(
        surface_rect(&input(&s, 0, 1.0)),
        Some(origin((88, 88), (88.0, 88.0), (12, 12)))
    );
}

#[test]
fn surface_rect_cut_by_ancestor_rect_clip_outward() {
    // 範囲 [3, 22) を切り抜き [0, 9.5)（物理へ外向きに 10）で切る。
    let mut e = node(1, 5.0, 5.0, 20.0, 20.0);
    e.ancestors.rect_clip = Some(Aabb::new(0.0, 0.0, 9.5, 9.5));
    assert_eq!(
        surface_rect(&input(&scan(vec![e]), 0, 1.0)),
        Some(origin((3, 3), (3.0, 3.0), (7, 7)))
    );
}

#[test]
fn surface_rect_with_scale_keeps_dip_from_px() {
    // 拡大 1.5: 物理 [15, 30)、余白 3 → [12, 33)。dip = 12 / 1.5 = 8。
    let s = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0)]);
    assert_eq!(
        check_shape(&input(&s, 0, 1.5)),
        Ok(origin((12, 12), (8.0, 8.0), (21, 21)))
    );
}

#[test]
fn regrow_only_when_extent_overflows_the_rect() {
    let s = scan(vec![node(1, 10.0, 10.0, 20.0, 20.0)]);
    let cur = surface_rect(&input(&s, 0, 1.0)).unwrap();
    // 同じ・縮む → 決め直さない。
    assert_eq!(regrow(&input(&s, 0, 1.0), &cur), None);
    let shrunk = scan(vec![node(1, 12.0, 12.0, 18.0, 18.0)]);
    assert_eq!(regrow(&input(&shrunk, 0, 1.0), &cur), None);
    // 子が増えてはみ出す → 新しい範囲で決め直す。
    let grown = scan(vec![
        node(1, 10.0, 10.0, 20.0, 20.0),
        node(2, 30.0, 30.0, 40.0, 40.0),
    ]);
    assert_eq!(
        regrow(&input(&grown, 0, 1.0), &cur),
        Some(origin((8, 8), (8.0, 8.0), (34, 34)))
    );
    // 1 ピクセルだけはみ出しても決め直す。
    let edge = scan(vec![node(1, 10.0, 10.0, 20.5, 20.0)]);
    assert_eq!(
        regrow(&input(&edge, 0, 1.0), &cur),
        Some(origin((8, 8), (8.0, 8.0), (15, 14)))
    );
}

#[test]
fn fractional_origin_condition() {
    let at = |x: f32, y: f32| {
        let mut e = node(1, 0.0, 0.0, 1.0, 1.0);
        e.world.world_mat = Matrix3x2::translation(x, y);
        e
    };
    assert!(!fractional_origin(&at(10.0, 20.0), 1.0));
    assert!(fractional_origin(&at(10.3, 20.0), 1.0));
    assert!(fractional_origin(&at(10.0, 20.5), 1.0));
    // 拡大 1.25: 10 DIP → 12.5（端数）、8 DIP → 10（整数）。
    assert!(fractional_origin(&at(10.0, 8.0), 1.25));
    assert!(!fractional_origin(&at(8.0, 8.0), 1.25));
}
