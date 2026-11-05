//! 窓へ後からつないだ部品のテスト（4 番）
//!
//! 窓の外で作った部品の木を、後から窓の下へ付けると、根と子孫に見た目の部品が入る。
//! 窓の外へ付けたままなら入らない。本物の登録を通すため `EcsWorld` の
//! `PreRenderSurface` だけを回す（窓を作る係は回さない＝実の窓は立たない）。
//! 同じスケジュールの係が `GraphicsCore` を要るので、実の GPU で作って入れる。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::world::{EcsWorld, PreRenderSurface};
use wintf::ecs::{
    GraphicsCore, SurfaceGraphics, SurfaceGraphicsDirty, Visual, VisualGraphics, Window,
};

/// 窓の外で根と子の木を作る（`Visual` のフックはこの時点では見た目の部品を入れない）。
fn spawn_detached_tree(world: &mut World) -> (Entity, Entity) {
    let root = world.spawn(Visual::default()).id();
    let child = world.spawn((Visual::default(), ChildOf(root))).id();
    world.flush();
    assert!(world.get::<VisualGraphics>(root).is_none());
    assert!(world.get::<VisualGraphics>(child).is_none());
    (root, child)
}

fn has_graphics_parts(world: &World, entity: Entity) -> bool {
    world.get::<VisualGraphics>(entity).is_some()
        && world.get::<SurfaceGraphics>(entity).is_some()
        && world.get::<SurfaceGraphicsDirty>(entity).is_some()
}

#[test]
fn test_tree_joined_to_window_gets_graphics_parts() {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let window = world.spawn(Window::default()).id();
    let (root, child) = spawn_detached_tree(world);
    world.run_schedule(PreRenderSurface);

    world.entity_mut(root).insert(ChildOf(window));
    world.run_schedule(PreRenderSurface);

    assert!(
        has_graphics_parts(world, root),
        "窓へ付けた根に見た目の部品が入る"
    );
    assert!(has_graphics_parts(world, child), "窓へ付けた根の子にも入る");
}

#[test]
fn test_tree_joined_outside_window_gets_nothing() {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let other = world.spawn_empty().id();
    let (root, child) = spawn_detached_tree(world);

    world.entity_mut(root).insert(ChildOf(other));
    world.run_schedule(PreRenderSurface);

    assert!(world.get::<VisualGraphics>(root).is_none());
    assert!(world.get::<VisualGraphics>(child).is_none());
}
