//! `build` の不透明度の層の純粋なテスト（工場だけ・装置なし・要件 2.1, 2.3, 2.4, 2.5, 6.3）。

use super::super::*;
use super::builder_tests::*;
use crate::numerics::*;

/// 不透明度つき・切り抜きなしの節。
fn faded(opacity: f32) -> Visual {
    let mut v = Visual::default();
    v.set_opacity(opacity);
    v
}

fn push_opacity(opacity: f32, l: f32, t: f32, r: f32, b: f32) -> String {
    format!("push_opacity {} {:?}", opacity, Aabb::new(l, t, r, b))
}

#[test]
fn opacity_layer_wraps_clip_draw_and_pops_in_reverse() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let mut v = clipped();
    v.set_opacity(0.5);
    let g = group(&mut world, root, v, at(10.0, 10.0, 20.0, 20.0));
    let c = leaf(&mut world, g, 0.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(
        shape(&out),
        vec![
            push_opacity(0.5, 10.0, 10.0, 14.0, 14.0),
            push_rect(10.0, 10.0, 30.0, 30.0),
            draw(c, 10.0, 10.0, 14.0, 14.0),
            "pop_rect".into(),
            "pop_opacity".into(),
        ]
    );
}

#[test]
fn opaque_node_emits_no_layer() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let g = group(&mut world, root, faded(1.0), at(0.0, 0.0, 0.0, 0.0));
    let c = leaf(&mut world, g, 0.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(shape(&out), vec![draw(c, 0.0, 0.0, 4.0, 4.0)]);
}

#[test]
fn root_opacity_emits_no_layer() {
    let mut world = World::new();
    let root = world.spawn(faded(0.5)).id();
    let c = leaf(&mut world, root, 0.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(shape(&out), vec![draw(c, 0.0, 0.0, 4.0, 4.0)]);
}

#[test]
fn nested_and_sibling_layers_close_at_depth_returns() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let g = group(&mut world, root, faded(0.5), at(0.0, 0.0, 0.0, 0.0));
    let a = group(&mut world, g, faded(0.25), at(0.0, 0.0, 0.0, 0.0));
    let ca = leaf(&mut world, a, 0.0, 0.0);
    let b = group(&mut world, g, faded(0.75), at(10.0, 0.0, 0.0, 0.0));
    let cb = leaf(&mut world, b, 0.0, 0.0);
    let after = leaf(&mut world, root, 20.0, 0.0); // 深さが 3 → 1 へ戻る
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(
        shape(&out),
        vec![
            push_opacity(0.5, 0.0, 0.0, 14.0, 4.0),
            push_opacity(0.25, 0.0, 0.0, 4.0, 4.0),
            draw(ca, 0.0, 0.0, 4.0, 4.0),
            "pop_opacity".into(),
            push_opacity(0.75, 10.0, 0.0, 14.0, 4.0),
            draw(cb, 10.0, 0.0, 14.0, 4.0),
            "pop_opacity".into(),
            "pop_opacity".into(),
            draw(after, 20.0, 0.0, 24.0, 4.0),
        ]
    );
}

#[test]
fn layer_range_is_subtree_bounds_within_outer_clip() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let clip = group(&mut world, root, clipped(), at(0.0, 0.0, 10.0, 10.0));
    let g = group(&mut world, clip, faded(0.5), at(0.0, 0.0, 0.0, 0.0));
    let c = leaf(&mut world, g, 8.0, 0.0);
    let out = flatten(&mut world, root, INFINITE);
    assert_eq!(
        shape(&out),
        vec![
            push_rect(0.0, 0.0, 10.0, 10.0),
            push_opacity(0.5, 8.0, 0.0, 10.0, 4.0),
            draw(c, 8.0, 0.0, 12.0, 4.0),
            "pop_opacity".into(),
            "pop_rect".into(),
        ]
    );
}

#[test]
fn empty_layer_range_culls_the_subtree() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    // 部分木が viewport の外。
    let out_of_view = group(&mut world, root, faded(0.5), at(100.0, 0.0, 0.0, 0.0));
    leaf(&mut world, out_of_view, 0.0, 0.0);
    // 中身の無い部分木（外接矩形が空）。
    let g = group(&mut world, root, faded(0.5), at(0.0, 0.0, 0.0, 0.0));
    group(&mut world, g, Visual::default(), at(0.0, 0.0, 0.0, 0.0));
    let kept = leaf(&mut world, root, 0.0, 0.0);
    let out = flatten(&mut world, root, Aabb::new(0.0, 0.0, 50.0, 50.0));
    assert_eq!(shape(&out), vec![draw(kept, 0.0, 0.0, 4.0, 4.0)]);
}
