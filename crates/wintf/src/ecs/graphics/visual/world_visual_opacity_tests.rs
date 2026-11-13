//! `fill_world_visuals` の節の不透明度のテスト（装置なし・`World` だけ・要件 2.2, 2.6, 2.7, 2.8）。

use super::draw_content_test_support::solid_rect_content;
use super::world_visual_tests::{Filler, kids, rect4, version};
use super::*;
use crate::ecs::test_support::capture_under_filter;
use bevy_ecs::hierarchy::ChildOf;

const NAN_TEXT: &str = "節の不透明度が NaN";

/// 根の下に 節 `n` → 子 `child` を作る（どちらも描く中身つき）。
fn node_with_child(world: &mut World, opacity: f32) -> (Entity, Entity, Entity) {
    let root = world.spawn(Visual::default()).id();
    let n = world
        .spawn((
            Visual {
                opacity,
                ..Default::default()
            },
            solid_rect_content(rect4()),
            Name::new("opacity-node"),
            ChildOf(root),
        ))
        .id();
    let child = world
        .spawn((Visual::default(), solid_rect_content(rect4()), ChildOf(n)))
        .id();
    (root, n, child)
}

#[test]
fn zero_opacity_node_and_descendants_are_not_visited_but_versions_follow() {
    let mut world = World::new();
    let (root, n, child) = node_with_child(&mut world, 0.0);
    let mut filler = Filler::new(&mut world);
    let top = kids(&world, root);
    let (_, visited) = filler.run_with(&mut world, root, false, &top, false);
    assert!(
        visited.is_empty(),
        "不透明度 0 の部分木は訪れない: {visited:?}"
    );

    let v_child = version(&world, child);
    world
        .get_mut::<VisualDrawContent>(child)
        .unwrap()
        .set_changed();
    let (changed, visited) = filler.run_with(&mut world, root, false, &top, false);
    assert!(visited.is_empty());
    assert!(changed);
    assert_eq!(
        version(&world, child),
        v_child + 1,
        "隠れている間も版を追う"
    );

    // 不透明度の変化も描き直しになる（2.7）。
    let v_n = version(&world, n);
    world.get_mut::<Visual>(n).unwrap().opacity = 0.5;
    let (changed, visited) = filler.run_with(&mut world, root, false, &top, false);
    assert!(changed);
    assert_eq!(version(&world, n), v_n + 1);
    assert_eq!(visited, vec![(n, 1), (child, 2)]);
}

#[test]
fn opacity_below_zero_clamps_to_zero_and_is_not_visited() {
    let mut world = World::new();
    let (root, _, _) = node_with_child(&mut world, -0.5);
    let top = kids(&world, root);
    let (_, visited) = Filler::new(&mut world).run_with(&mut world, root, false, &top, false);
    assert!(visited.is_empty(), "{visited:?}");
}

#[test]
fn non_one_opacity_does_not_warn() {
    let mut world = World::new();
    let (root, n, _) = node_with_child(&mut world, 0.5);
    let mut filler = Filler::new(&mut world);
    let log = capture_under_filter("warn", || {
        filler.run(&mut world, root);
        world.get_mut::<Visual>(n).unwrap().opacity = 2.0;
        filler.run(&mut world, root);
    });
    assert!(!log.contains("fill_world_visuals"), "{log}");
}

#[test]
fn nan_opacity_is_drawn_as_one_and_warns_once_per_entity() {
    let mut world = World::new();
    let (root, n, child) = node_with_child(&mut world, f32::NAN);
    let mut filler = Filler::new(&mut world);
    let mut visited = Vec::new();
    let log = capture_under_filter("warn", || {
        let top = kids(&world, root);
        visited = filler.run_with(&mut world, root, false, &top, false).1;
        filler.run(&mut world, root);
    });
    assert_eq!(visited, vec![(n, 1), (child, 2)], "NaN は 1.0 として訪れる");
    assert_eq!(log.matches(NAN_TEXT).count(), 1, "{log}");
    assert!(log.contains("opacity-node"), "実体名が要る: {log}");
}
