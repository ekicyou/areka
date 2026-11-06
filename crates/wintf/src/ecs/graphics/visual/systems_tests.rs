//! 後からの部品の出入りにも規則を効かせる係（`sync_surface_owner_parts`・要件 1.2a〜2c）。
//! 本物の登録を通すため `EcsWorld` の `PreRenderSurface` だけを回す（装置も実の窓も要らない）。

use super::draw_content_test_support::solid_rect_content;
use super::*;
use crate::ecs::world::{EcsWorld, PreRenderSurface};
use crate::ecs::{GraphicsCommandList, Window};
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

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

fn has_none(world: &World, e: Entity) -> bool {
    world.get::<VisualGraphics>(e).is_none()
        && world.get::<SurfaceGraphics>(e).is_none()
        && world.get::<SurfaceGraphicsDirty>(e).is_none()
}

/// 窓の下に何も持たない Visual（持ち主）と描く中身だけの Visual（節）を作る。
fn owner_and_node(ecs: &mut EcsWorld) -> (Entity, Entity) {
    let world = ecs.world_mut();
    let window = world.spawn(Window::default()).id();
    let owner = world.spawn((Visual::default(), ChildOf(window))).id();
    let node = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect()),
            ChildOf(window),
        ))
        .id();
    world.run_schedule(PreRenderSurface);
    assert!(has_parts(world, owner));
    assert!(has_none(world, node));
    (owner, node)
}

#[test]
fn owner_given_draw_content_later_becomes_node() {
    let mut ecs = EcsWorld::new();
    let (owner, _) = owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.entity_mut(owner).insert(solid_rect_content(rect()));
    world.run_schedule(PreRenderSurface);
    assert!(has_none(world, owner), "後から描く中身を入れた持ち主は節");
}

#[test]
fn node_given_own_surface_becomes_owner_and_back() {
    let mut ecs = EcsWorld::new();
    let (_, node) = owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.entity_mut(node).insert(OwnSurface);
    world.run_schedule(PreRenderSurface);
    assert!(has_parts(world, node), "目印を入れた節は持ち主");

    world.entity_mut(node).remove::<OwnSurface>();
    world.run_schedule(PreRenderSurface);
    assert!(has_none(world, node), "目印を外せば節へ戻る");
}

#[test]
fn node_given_command_list_becomes_owner_and_back() {
    let mut ecs = EcsWorld::new();
    let (_, node) = owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.entity_mut(node).insert(GraphicsCommandList::empty());
    world.run_schedule(PreRenderSurface);
    assert!(has_parts(world, node), "命令の列を入れた節は持ち主");

    world.entity_mut(node).remove::<GraphicsCommandList>();
    world.run_schedule(PreRenderSurface);
    assert!(has_none(world, node), "命令の列を外せば節へ戻る");
}

/// 装置の失い（`invalidate_dependent_components`）は `VisualDrawContent` を外し、
/// `WorldVisual` は残す。節のまま（3 部品が入らない）。
#[test]
fn node_losing_draw_content_on_device_loss_stays_node() {
    let mut ecs = EcsWorld::new();
    let (_, node) = owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.entity_mut(node).remove::<VisualDrawContent>();
    world.run_schedule(PreRenderSurface);
    assert!(world.get::<WorldVisual>(node).is_some());
    assert!(has_none(world, node), "失いの形で中身を外しても節のまま");
}

/// 窓の外で作った木（持ち主の根・持ち主の子・節の子・目印つきの節の孫）を後から窓へ付ける。
#[test]
fn tree_joined_to_window_later_follows_rule() {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let window = world.spawn(Window::default()).id();
    let root = world.spawn(Visual::default()).id();
    let owner_child = world.spawn((Visual::default(), ChildOf(root))).id();
    let node_child = world
        .spawn((Visual::default(), solid_rect_content(rect()), ChildOf(root)))
        .id();
    let promoted = world
        .spawn((
            Visual::default(),
            VisualTransform::default(),
            OwnSurface,
            ChildOf(node_child),
        ))
        .id();
    world.run_schedule(PreRenderSurface);
    for e in [root, owner_child, node_child, promoted] {
        assert!(has_none(world, e), "窓の外では入らない");
    }

    world.entity_mut(root).insert(ChildOf(window));
    world.run_schedule(PreRenderSurface);
    assert!(has_parts(world, root));
    assert!(has_parts(world, owner_child));
    assert!(has_none(world, node_child), "節は窓へ付いても節");
    assert!(has_parts(world, promoted), "目印つきは持ち主");
}
