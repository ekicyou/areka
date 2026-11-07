//! 持ち主か節かの規則（`wants_own_surface`）とフックの確かめ（要件 1.2a〜2c）。

use super::draw_content_test_support::solid_rect_content;
use super::*;
use crate::ecs::world::EcsWorld;
use crate::ecs::{GraphicsCommandList, Window};
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

#[test]
fn wants_own_surface_truth_table() {
    // (命令の列, 目印, 節) → 持ち主か
    let table = [
        (false, false, false, true), // 何も持たない Visual は持ち主（1.2b）
        (false, false, true, false), // 新しい描き方の部品だけ＝節（1.2a）
        (false, true, false, true),
        (false, true, true, true), // 目印で手動の昇格（1.2c）
        (true, false, false, true),
        (true, false, true, true), // 命令の列は持ち主（1.2b）
        (true, true, false, true),
        (true, true, true, true),
    ];
    for (cl, own, node, want) in table {
        assert_eq!(
            wants_own_surface(cl, own, node),
            want,
            "command_list={cl} own_surface={own} is_node={node}"
        );
    }
}

fn rect() -> D2D_RECT_F {
    D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 4.0,
        bottom: 4.0,
    }
}

fn has_parts(world: &World, e: Entity) -> bool {
    world.get::<VisualGraphics>(e).is_some()
        && world.get::<SurfaceGraphics>(e).is_some()
        && world.get::<SurfaceGraphicsDirty>(e).is_some()
}

/// フックは同じバンドルで入った部品（require で入る `WorldVisual` も）を見て決める。
#[test]
fn hook_sees_components_of_same_bundle() {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let window = world.spawn(Window::default()).id();
    let plain = world.spawn((Visual::default(), ChildOf(window))).id();
    let content_node = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect()),
            ChildOf(window),
        ))
        .id();
    let transform_node = world
        .spawn((
            Visual::default(),
            VisualTransform::default(),
            ChildOf(window),
        ))
        .id();
    let promoted = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect()),
            OwnSurface,
            ChildOf(window),
        ))
        .id();
    let with_list = world
        .spawn((
            Visual::default(),
            VisualTransform::default(),
            GraphicsCommandList::empty(),
            ChildOf(window),
        ))
        .id();
    world.flush();

    // 新しい描き方の 2 部品は世界の行列と見た目の版を必ず連れてくる。
    for e in [content_node, transform_node] {
        assert!(world.get::<WorldVisual>(e).is_some());
        assert!(world.get::<VisualVersion>(e).is_some());
    }
    assert!(
        has_parts(world, plain),
        "何も持たない Visual は持ち主のまま"
    );
    assert!(!has_parts(world, content_node), "描く中身だけ＝節");
    assert!(!has_parts(world, transform_node), "局所の変形だけ＝節");
    assert!(has_parts(world, promoted), "目印があれば持ち主");
    assert!(has_parts(world, with_list), "命令の列があれば持ち主");
    for e in [content_node, transform_node] {
        assert!(world.get::<VisualGraphics>(e).is_none());
        assert!(world.get::<SurfaceGraphics>(e).is_none());
        assert!(world.get::<SurfaceGraphicsDirty>(e).is_none());
    }
}
