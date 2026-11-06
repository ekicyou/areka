//! `fill_world_visuals` の純粋なテスト（装置なし・`World` だけ・要件 1.2〜1.11, 2.2, 4.1, 5.3, 5.4, 6.7）。

use super::draw_content_test_support::solid_rect_content;
use super::*;
use crate::ecs::common::MAX_ANCESTOR_DEPTH;
use crate::ecs::graphics::VisualGraphics;
use crate::ecs::layout::Arrangement;
use crate::ecs::test_support::capture_under_filter;
use crate::numerics::*;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::system::SystemState;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows_numerics::{Matrix3x2, Vector2};

const DEPTH_TEXT: &str = "木の深さが上限を越えた";
const OWNER_TEXT: &str = "節の下の描画面の持ち主は支えない";

fn rect4() -> D2D_RECT_F {
    D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 4.0,
        bottom: 4.0,
    }
}

fn arranged(x: f32, y: f32) -> Arrangement {
    Arrangement {
        offset: crate::ecs::Offset { x, y },
        ..Default::default()
    }
}

fn moved(x: f32, y: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        offset: Vector2 { X: x, Y: y },
        ..Transform2D::identity()
    })
}

fn scaled(x: f32, y: f32) -> VisualTransform {
    VisualTransform(Transform2D {
        scale: Vector2 { X: x, Y: y },
        ..Transform2D::identity()
    })
}

/// 根の子を `children_of_root` として 1 回埋める。
fn fill(
    world: &mut World,
    root: Entity,
    children_of_root: &[Entity],
) -> (bool, Vec<(Entity, usize)>) {
    let mut state = SystemState::<Query<FillData>>::new(world);
    let mut nodes = state.get_mut(world).unwrap();
    let mut visited = Vec::new();
    let changed = fill_world_visuals(
        root,
        Some(&Name::new("root")),
        &mut nodes,
        children_of_root,
        &mut visited,
    );
    (changed, visited)
}

fn kids(world: &World, e: Entity) -> Vec<Entity> {
    world
        .get::<Children>(e)
        .map(|c| c.to_vec())
        .unwrap_or_default()
}

fn wv(world: &World, e: Entity) -> WorldVisual {
    *world.get::<WorldVisual>(e).unwrap()
}

#[test]
fn two_levels_multiply_transform_then_offset() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((
            Visual::default(),
            moved(10.0, 20.0),
            arranged(1.0, 2.0),
            ChildOf(root),
        ))
        .id();
    let b = world
        .spawn((
            Visual::default(),
            scaled(2.0, 2.0),
            arranged(3.0, 0.0),
            ChildOf(a),
        ))
        .id();
    let (changed, visited) = {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    assert!(changed);
    assert_eq!(visited, vec![(a, 1), (b, 2)]);
    assert_eq!(wv(&world, a).world_mat, Matrix3x2::translation(11.0, 22.0));
    // 局所 = 拡大 × 平行移動(offset)（offset は拡大されない）× 親。
    let want = Matrix3x2 {
        M11: 2.0,
        M12: 0.0,
        M21: 0.0,
        M22: 2.0,
        M31: 14.0,
        M32: 22.0,
    };
    assert_eq!(wv(&world, b).world_mat, want);
    assert!(wv(&world, b).axis_aligned);
    // 同じ値なら書かない。
    let (changed, _) = {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    assert!(!changed);
}

#[test]
fn flip_gives_ordered_world_bounds() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let n = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            scaled(-1.0, 1.0),
            ChildOf(root),
        ))
        .id();
    {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    let w = wv(&world, n);
    assert!(w.axis_aligned);
    assert_eq!(w.content_world_aabb, Aabb::new(-4.0, 0.0, 0.0, 4.0));
}

#[test]
fn subtree_is_union_of_content_and_children_in_preorder() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let p = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            ChildOf(root),
        ))
        .id();
    let c1 = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect4()),
            arranged(10.0, 10.0),
            ChildOf(p),
        ))
        .id();
    // 中身の無い節は和に何も足さない。
    let c2 = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(p)))
        .id();
    let (_, visited) = {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    assert_eq!(visited, vec![(p, 1), (c1, 2), (c2, 2)]);
    assert_eq!(
        wv(&world, p).world_mat,
        Matrix3x2::identity(),
        "変形なしは恒等"
    );
    assert_eq!(
        wv(&world, p).content_world_aabb,
        Aabb::new(0.0, 0.0, 4.0, 4.0)
    );
    assert_eq!(
        wv(&world, p).subtree_world_aabb,
        Aabb::new(0.0, 0.0, 14.0, 14.0)
    );
    assert_eq!(
        wv(&world, c1).subtree_world_aabb,
        Aabb::new(10.0, 10.0, 14.0, 14.0)
    );
    assert!(wv(&world, c2).subtree_world_aabb.is_empty());
}

#[test]
fn invisible_node_skips_its_subtree() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((Visual::default(), moved(1.0, 0.0), ChildOf(root)))
        .id();
    let hidden = world
        .spawn((
            Visual {
                is_visible: false,
                ..Default::default()
            },
            moved(2.0, 0.0),
            ChildOf(a),
        ))
        .id();
    let under = world
        .spawn((Visual::default(), moved(3.0, 0.0), ChildOf(hidden)))
        .id();
    let (_, visited) = {
        let top = kids(&world, root);
        fill(&mut world, root, &top)
    };
    assert_eq!(visited, vec![(a, 1)]);
    assert_eq!(wv(&world, hidden).world_mat, Matrix3x2::identity());
    assert_eq!(wv(&world, under).world_mat, Matrix3x2::identity());
}

#[test]
fn depth_over_limit_stops_with_one_error() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let mut parent = root;
    for _ in 0..MAX_ANCESTOR_DEPTH + 5 {
        parent = world
            .spawn((Visual::default(), WorldVisual::default(), ChildOf(parent)))
            .id();
    }
    let top = kids(&world, root);
    let mut visited = Vec::new();
    let log = capture_under_filter("error", || visited = fill(&mut world, root, &top).1);
    assert_eq!(visited.len(), MAX_ANCESTOR_DEPTH);
    assert_eq!(visited.last().unwrap().1, MAX_ANCESTOR_DEPTH);
    assert_eq!(log.matches(DEPTH_TEXT).count(), 1, "{log}");
    assert!(log.contains("root"), "根の実体名が要る: {log}");
}

#[test]
fn parent_cycle_does_not_hang() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let a = world
        .spawn((Visual::default(), WorldVisual::default()))
        .id();
    let b = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(a)))
        .id();
    world.entity_mut(a).insert(ChildOf(b));
    let mut visited = Vec::new();
    let log = capture_under_filter("error", || visited = fill(&mut world, root, &[a]).1);
    assert_eq!(visited.len(), MAX_ANCESTOR_DEPTH);
    assert_eq!(log.matches(DEPTH_TEXT).count(), 1, "{log}");
}

#[test]
fn owner_under_node_is_cut_with_error_but_not_under_root() {
    let mut world = World::new();
    let root = world.spawn(Visual::default()).id();
    let node = world
        .spawn((Visual::default(), WorldVisual::default(), ChildOf(root)))
        .id();
    // 根の直下の持ち主は今の WUC の木の子なので記録しない。
    world.spawn((Visual::default(), Name::new("root-owner"), ChildOf(root)));
    let owner = world
        .spawn((
            Visual::default(),
            WorldVisual::default(),
            VisualGraphics::default(),
            Name::new("deep-owner"),
            ChildOf(node),
        ))
        .id();
    world.spawn((Visual::default(), WorldVisual::default(), ChildOf(owner)));
    let top = kids(&world, root);
    let mut visited = Vec::new();
    let log = capture_under_filter("error", || visited = fill(&mut world, root, &top).1);
    assert_eq!(visited, vec![(node, 1)]);
    assert_eq!(log.matches(OWNER_TEXT).count(), 1, "{log}");
    assert!(log.contains("deep-owner"), "実体名が要る: {log}");
    assert!(!log.contains("root-owner"), "{log}");
}
