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

// ---- 項の規則（要件 1.4・1.5・2.1・3.3・5.3）。素の値を受ける関数を COM 無しで確かめる ----

use super::super::MixMode;
use super::command::*;
use crate::numerics::Aabb;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

const LINEAR: D2D1_INTERPOLATION_MODE = D2D1_INTERPOLATION_MODE_LINEAR;
const NEAREST: D2D1_INTERPOLATION_MODE = D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR;
const OVER: MixMode = MixMode::Composite(D2D1_COMPOSITE_MODE_SOURCE_OVER);
const SURFACE: (u32, u32) = (200, 100);

fn entity(raw: u32) -> Entity {
    Entity::from_raw_u32(raw).expect("テスト用 entity index は有効")
}

fn root() -> Parent {
    Parent::default()
}

/// 原点の 10×10 の中身を `mat` で置いた件。
fn item(
    hash: u64,
    mat: Matrix3x2,
    interp: D2D1_INTERPOLATION_MODE,
    mix: MixMode,
    parent: &Parent,
    scale: (f32, f32),
) -> Option<DamageEntry> {
    let aabb = Aabb::new(0.0, 0.0, 10.0, 10.0).transform(mat);
    item_entry(
        hash,
        entity(1),
        1,
        mat,
        aabb,
        interp,
        mix,
        scale,
        parent,
        SURFACE,
    )
}

fn layer(l: Layer, aabb: Aabb, parent: &Parent) -> DamageEntry {
    layer_entry(l, entity(2), 1, aabb, (1.0, 1.0), parent).unwrap()
}

#[test]
fn item_moving_out_of_rect_clip_is_another_id() {
    let clip = layer(Layer::ClipRect, Aabb::new(0.0, 0.0, 50.0, 50.0), &root());
    let inside = root().inner(0, &clip);
    let mat = Matrix3x2::translation(5.0, 5.0);
    let a = item(7, mat, LINEAR, OVER, &inside, (1.0, 1.0)).unwrap();
    let b = item(7, mat, LINEAR, OVER, &root(), (1.0, 1.0)).unwrap();
    assert_ne!(a.id, b.id);
    assert_ne!(a.shape, b.shape);
    assert_eq!(a.parent, Some(0));
    assert_eq!(b.parent, None);
}

#[test]
fn opacity_change_gives_layer_and_inner_items_another_id() {
    let aabb = Aabb::new(0.0, 0.0, 50.0, 50.0);
    let l1 = layer(Layer::Opacity(0.5), aabb, &root());
    let l2 = layer(Layer::Opacity(0.6), aabb, &root());
    assert_ne!(l1.id, l2.id);
    assert_ne!(l1.shape, l2.shape);
    let mat = Matrix3x2::identity();
    let a = item(7, mat, LINEAR, OVER, &root().inner(0, &l1), (1.0, 1.0)).unwrap();
    let b = item(7, mat, LINEAR, OVER, &root().inner(0, &l2), (1.0, 1.0)).unwrap();
    assert_ne!(a.id, b.id);
    // 同じ材料からは同じ項（1.6）
    assert_eq!(l1, layer(Layer::Opacity(0.5), aabb, &root()));
}

#[test]
fn clips_cut_item_bounds_but_opacity_layer_does_not() {
    let small = Aabb::new(0.0, 0.0, 8.0, 8.0);
    let mat = Matrix3x2::identity();
    let o = layer(Layer::Opacity(0.5), small, &root());
    assert_eq!(o.bounds, r(0, 0, 8, 8));
    let in_o = item(1, mat, NEAREST, OVER, &root().inner(0, &o), (1.0, 1.0)).unwrap();
    assert_eq!(in_o.bounds, r(0, 0, 10, 10));
    let c = layer(Layer::ClipRect, small, &root());
    let in_c = item(1, mat, NEAREST, OVER, &root().inner(0, &c), (1.0, 1.0)).unwrap();
    assert_eq!(in_c.bounds, r(0, 0, 8, 8));
    let g = layer(
        Layer::ClipGeometry(Matrix3x2::identity()),
        Aabb::new(2.0, 2.0, 30.0, 30.0),
        &root(),
    );
    let in_g = item(1, mat, NEAREST, OVER, &root().inner(0, &g), (1.0, 1.0)).unwrap();
    assert_eq!(in_g.bounds, r(2, 2, 10, 10));
    // 入れ子は鎖の交わりで切る
    let both = root().inner(0, &c).inner(1, &g);
    let in_both = item(1, mat, NEAREST, OVER, &both, (1.0, 1.0)).unwrap();
    assert_eq!(in_both.bounds, r(2, 2, 8, 8));
    assert_eq!(in_both.parent, Some(1));
}

#[test]
fn non_source_over_item_spreads_to_rect_clip_or_whole_surface() {
    let mat = Matrix3x2::translation(5.0, 5.0);
    let copy = MixMode::Composite(D2D1_COMPOSITE_MODE_SOURCE_COPY);
    let a = item(1, mat, LINEAR, copy, &root(), (1.0, 1.0)).unwrap();
    assert_eq!(a.kind, EntryKind::Draw { unbounded: true });
    assert_eq!(a.bounds, r(0, 0, 200, 100));
    let c = layer(Layer::ClipRect, Aabb::new(1.5, 2.0, 40.0, 30.2), &root());
    let b = item(1, mat, LINEAR, copy, &root().inner(0, &c), (1.0, 1.0)).unwrap();
    assert_eq!(b.bounds, r(1, 2, 40, 31));
    // 図形の切り抜きでは切らない（軸に沿う矩形の切り抜きまで広がる）
    let g = layer(
        Layer::ClipGeometry(Matrix3x2::identity()),
        Aabb::new(0.0, 0.0, 12.0, 12.0),
        &root(),
    );
    let in_g = item(1, mat, LINEAR, copy, &root().inner(0, &g), (1.0, 1.0)).unwrap();
    assert_eq!(in_g.bounds, r(0, 0, 200, 100));
    // ブレンドと範囲の外の重ね方は SOURCE_OVER と同じく自分の範囲
    let blend = MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY);
    let bl = item(1, mat, NEAREST, blend, &root(), (1.0, 1.0)).unwrap();
    assert_eq!(bl.kind, EntryKind::Draw { unbounded: false });
    assert_eq!(bl.bounds, r(5, 5, 15, 15));
    let oor = MixMode::Composite(D2D1_COMPOSITE_MODE(99));
    let o = item(1, mat, NEAREST, oor, &root(), (1.0, 1.0)).unwrap();
    assert_eq!(o.kind, EntryKind::Draw { unbounded: false });
    assert_eq!(o.bounds, r(5, 5, 15, 15));
}

#[test]
fn margin_per_interpolation_and_nearest_is_zero() {
    let exact = [10.25, 10.0, 20.25, 20.0];
    let id = [1.0, 0.0, 0.0, 1.0];
    assert_eq!(
        item_bounds(exact, id, NEAREST, OVER, &root(), SURFACE),
        Some(r(10, 10, 21, 20))
    );
    for interp in [
        D2D1_INTERPOLATION_MODE_LINEAR,
        D2D1_INTERPOLATION_MODE_CUBIC,
        D2D1_INTERPOLATION_MODE_MULTI_SAMPLE_LINEAR,
        D2D1_INTERPOLATION_MODE_ANISOTROPIC,
        D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
    ] {
        assert_eq!(
            item_bounds(exact, id, interp, OVER, &root(), SURFACE),
            Some(r(8, 8, 23, 22)),
            "{interp:?}"
        );
    }
    // 拡大 1.5 倍: s = 1.5、余白 ceil(3) = 3
    let mat = Matrix3x2::translation(10.25, 10.0);
    let e = item(1, mat, LINEAR, OVER, &root(), (1.5, 1.5)).unwrap();
    assert_eq!(e.exact, [15.375, 15.0, 30.375, 30.0]);
    assert_eq!(e.linear, [1.5, 0.0, 0.0, 1.5]);
    assert_eq!(e.origin, [15.375, 15.0]);
    assert_eq!(e.bounds, r(12, 12, 34, 33));
    // 縮小（s < 1）でも s = 1 として 2
    let half = item(1, Matrix3x2::identity(), LINEAR, OVER, &root(), (0.5, 0.5)).unwrap();
    assert_eq!(half.bounds, r(-2, -2, 7, 7));
    // 回転とせん断は列の長さの大きい方: 列 (M11, M21) = (0, 3) で s = 3、余白 6
    let shear = [0.0, 1.0, 3.0, 0.0];
    assert_eq!(
        item_bounds(
            [0.0, 0.0, 10.0, 10.0],
            shear,
            LINEAR,
            OVER,
            &root(),
            SURFACE
        ),
        Some(r(-6, -6, 16, 16))
    );
    // 補間が違う件は別の shape
    let n = item(1, mat, NEAREST, OVER, &root(), (1.5, 1.5)).unwrap();
    assert_ne!(n.shape, e.shape);
}

#[test]
fn translation_only_keeps_shape() {
    let a = item(
        1,
        Matrix3x2::translation(3.0, 4.0),
        LINEAR,
        OVER,
        &root(),
        (1.0, 1.0),
    )
    .unwrap();
    let b = item(
        2,
        Matrix3x2::translation(30.5, -4.0),
        LINEAR,
        OVER,
        &root(),
        (1.0, 1.0),
    )
    .unwrap();
    assert_eq!(a.shape, b.shape);
    assert_ne!(a.id, b.id);
    let c1 = layer(Layer::ClipRect, Aabb::new(0.0, 0.0, 10.0, 10.0), &root());
    let c2 = layer(Layer::ClipRect, Aabb::new(5.0, 5.0, 25.0, 25.0), &root());
    assert_eq!(c1.shape, c2.shape);
    assert_ne!(c1.id, c2.id);
    // 層が動いただけなら中の件も同じ shape
    let mat = Matrix3x2::identity();
    let ia = item(1, mat, LINEAR, OVER, &root().inner(0, &c1), (1.0, 1.0)).unwrap();
    let ib = item(1, mat, LINEAR, OVER, &root().inner(0, &c2), (1.0, 1.0)).unwrap();
    assert_eq!(ia.shape, ib.shape);
    assert_ne!(ia.id, ib.id);
}

#[test]
fn layer_physical_values_and_bounds() {
    let c = layer_entry(
        Layer::ClipRect,
        entity(2),
        1,
        Aabb::new(1.25, 2.0, 10.0, 10.75),
        (2.0, 2.0),
        &root(),
    )
    .unwrap();
    assert_eq!(c.kind, EntryKind::ClipRect);
    assert_eq!(c.exact, [2.5, 4.0, 20.0, 21.5]);
    assert_eq!(c.origin, [2.5, 4.0]);
    assert_eq!(c.linear, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(c.bounds, r(2, 4, 20, 22));
    let m = Matrix3x2::translation(3.0, 1.0);
    let g = layer_entry(
        Layer::ClipGeometry(m),
        entity(2),
        1,
        Aabb::new(3.0, 1.0, 13.0, 11.0),
        (2.0, 2.0),
        &root(),
    )
    .unwrap();
    assert_eq!(g.kind, EntryKind::ClipGeometry);
    assert_eq!(g.linear, [2.0, 0.0, 0.0, 2.0]);
    assert_eq!(g.origin, [6.0, 2.0]);
    assert_eq!(g.bounds, r(6, 2, 26, 22));
}

#[test]
fn non_finite_or_out_of_i32_is_undecidable() {
    let nan = Matrix3x2 {
        M31: f32::NAN,
        ..Matrix3x2::identity()
    };
    assert!(item(1, nan, LINEAR, OVER, &root(), (1.0, 1.0)).is_none());
    assert!(
        item(
            1,
            Matrix3x2::identity(),
            LINEAR,
            OVER,
            &root(),
            (f32::INFINITY, 1.0)
        )
        .is_none()
    );
    let far = Matrix3x2::translation(3.0e9, 0.0);
    assert!(item(1, far, NEAREST, OVER, &root(), (1.0, 1.0)).is_none());
    // i32 に収まる矩形でも、余白（200 倍で 400）を足すとあふれる
    let edge = [2.0e9, 0.0, 2.147_483_5e9, 1.0];
    let big = [200.0, 0.0, 0.0, 200.0];
    assert!(item_bounds(edge, big, NEAREST, OVER, &root(), SURFACE).is_some());
    assert!(item_bounds(edge, big, LINEAR, OVER, &root(), SURFACE).is_none());
    // 空の図形の外接矩形（裏返った無限）
    let empty = layer_entry(
        Layer::ClipGeometry(Matrix3x2::identity()),
        entity(2),
        1,
        Aabb::EMPTY,
        (1.0, 1.0),
        &root(),
    );
    assert!(empty.is_none());
    let nan_opacity = layer_entry(
        Layer::Opacity(f32::NAN),
        entity(2),
        1,
        Aabb::new(0.0, 0.0, 1.0, 1.0),
        (1.0, 1.0),
        &root(),
    );
    assert!(nan_opacity.is_none());
}

fn push_rect(raw: u32, aabb: Aabb) -> DrawCommand {
    DrawCommand::PushClipRect(ClipRect {
        entity: entity(raw),
        version: 1,
        world_aabb: aabb,
    })
}

fn push_opacity(raw: u32, opacity: f32, aabb: Aabb) -> DrawCommand {
    DrawCommand::PushOpacity(OpacityLayer {
        entity: entity(raw),
        version: 1,
        opacity,
        world_aabb: aabb,
    })
}

/// 件は COM の命令の列が要るので、層だけの列で `entries` の畳み方を確かめる（件の欄の渡し方は WARP のテスト）。
#[test]
fn entries_fold_layers_one_to_one_with_parent_and_end() {
    let a = Aabb::new(0.0, 0.0, 50.0, 50.0);
    let b = Aabb::new(10.0, 10.0, 20.0, 20.0);
    let list = [
        push_opacity(1, 0.5, a),
        push_rect(2, b),
        DrawCommand::PopClipRect,
        push_rect(3, a),
        DrawCommand::PopClipRect,
        DrawCommand::PopOpacity,
        push_rect(4, b),
        DrawCommand::PopClipRect,
    ];
    let e = entries(&list, (1.0, 1.0), SURFACE).unwrap();
    assert_eq!(e.len(), 4);
    let parents: Vec<_> = e.iter().map(|x| x.parent).collect();
    let ends: Vec<_> = e.iter().map(|x| x.end).collect();
    assert_eq!(parents, [None, Some(0), Some(0), None]);
    assert_eq!(ends, [3, 2, 3, 4]);
    // 規則の関数と同じ項
    let o = layer_entry(Layer::Opacity(0.5), entity(1), 1, a, (1.0, 1.0), &root()).unwrap();
    assert_eq!(e[0].id, o.id);
    assert_eq!(
        e[1].id,
        layer_entry(
            Layer::ClipRect,
            entity(2),
            1,
            b,
            (1.0, 1.0),
            &root().inner(0, &o)
        )
        .unwrap()
        .id
    );
    assert_eq!(entries(&list, (1.0, 1.0), SURFACE), Some(e));
    assert_eq!(entries(&[], (1.0, 1.0), SURFACE), Some(vec![]));
}

#[test]
fn entries_undecidable_when_any_value_is_not_finite_or_unbalanced() {
    let a = Aabb::new(0.0, 0.0, 50.0, 50.0);
    let bad = [
        push_rect(1, a),
        DrawCommand::PopClipRect,
        push_opacity(2, 0.5, Aabb::new(0.0, 0.0, f32::INFINITY, 1.0)),
        DrawCommand::PopOpacity,
    ];
    assert_eq!(entries(&bad, (1.0, 1.0), SURFACE), None);
    assert_eq!(entries(&[push_rect(1, a)], (1.0, 1.0), SURFACE), None);
    assert_eq!(
        entries(&[DrawCommand::PopClipRect], (1.0, 1.0), SURFACE),
        None
    );
}
