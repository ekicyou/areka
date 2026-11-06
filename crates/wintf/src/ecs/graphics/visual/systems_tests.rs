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

/// 根の部品（`DrawTree`）は根にだけ在る: 節へ降りた持ち主からは 3 部品と一緒に外れる。
#[test]
fn demoted_owner_loses_draw_tree() {
    let mut ecs = EcsWorld::new();
    let (_, node) = owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.entity_mut(node).insert(OwnSurface);
    world.run_schedule(PreRenderSurface);
    assert!(has_parts(world, node));
    world.entity_mut(node).insert(DrawTree {
        commands: vec![],
        unit_rect: None,
    });

    world.entity_mut(node).remove::<OwnSurface>();
    world.run_schedule(PreRenderSurface);
    assert!(has_none(world, node));
    assert!(
        world.get::<DrawTree>(node).is_none(),
        "降りた持ち主に古い木を残さない"
    );
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

/// 組み直す条件（`needs_rebuild`）の真理値表 16 通り: どれか 1 つでも真なら組み直す。
#[test]
fn needs_rebuild_truth_table() {
    for bits in 0u8..16 {
        let (changed, removed_any, core_changed, has_tree) =
            (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0);
        let expected = changed || removed_any || core_changed || !has_tree;
        assert_eq!(
            needs_rebuild(changed, removed_any, core_changed, has_tree),
            expected,
            "changed={changed} removed_any={removed_any} core_changed={core_changed} has_tree={has_tree}"
        );
    }
}

/// 装置が無い（無効な）回も節の版は上がり（変化を捨てない）、根の部品は入らない（組まない）。
#[test]
fn invalid_device_round_bumps_version_without_tree() {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let window = world.spawn(Window::default()).id();
    let owner = world.spawn((Visual::default(), ChildOf(window))).id();
    let node = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect()),
            ChildOf(owner),
        ))
        .id();
    world.run_schedule(PreRenderSurface);
    assert!(has_parts(world, owner));
    let before = world.get::<VisualVersion>(node).unwrap().0;

    world.get_mut::<Visual>(node).unwrap().is_visible = false;
    world.run_schedule(PreRenderSurface);
    assert!(
        world.get::<VisualVersion>(node).unwrap().0 > before,
        "無効な回も版は上がる"
    );
    assert!(
        world.get::<DrawTree>(owner).is_none(),
        "装置が無ければ組まない"
    );
}

/// どの根か分からない出入り（`TreeEdits::any`）: 節の昇格（`VisualGraphics` が入る）と降格
/// （`VisualGraphics` が外れる・`GraphicsCommandList` の外れでも起きる）を 1 回だけ拾う。
#[test]
fn tree_edits_catch_promotion_and_demotion() {
    let mut world = World::new();
    let any = world.register_system(|mut edits: TreeEdits| edits.any());
    let node = world
        .spawn((Visual::default(), WorldVisual::default()))
        .id();
    assert!(!world.run_system(any).unwrap(), "何も無ければ出入りなし");

    world.entity_mut(node).insert(VisualGraphics::default());
    assert!(world.run_system(any).unwrap(), "節の昇格を拾う");
    assert!(!world.run_system(any).unwrap(), "同じ昇格は 1 回だけ");

    world.entity_mut(node).remove::<VisualGraphics>();
    assert!(world.run_system(any).unwrap(), "降格を拾う");
    assert!(!world.run_system(any).unwrap(), "同じ降格は 1 回だけ");

    let owner = world.spawn(Visual::default()).id();
    world.entity_mut(owner).insert(VisualGraphics::default());
    assert!(
        !world.run_system(any).unwrap(),
        "節でない持ち主に入るのは昇格でない（根の部品が無ければどのみち組む）"
    );
}

/// 装置の失いの立ち直りの後に単位矩形が新しい工場の図形で作り直される（3.4・外のテストからは見えない欄）。
/// 立ち直りは `init_graphics_core` と同じく装置の資源を同じ場所で入れ替えて与える。
#[test]
fn unit_rect_recreated_after_device_loss_recovery() {
    use crate::ecs::graphics::{GraphicsCore, invalidate_dependent_components};
    use windows::core::Interface;

    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let window = world.spawn(Window::default()).id();
    let owner = world.spawn((Visual::default(), ChildOf(window))).id();
    let node = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect()),
            ChildOf(owner),
        ))
        .id();
    world.run_schedule(PreRenderSurface);
    let unit_rect = |w: &World| w.get::<DrawTree>(owner).unwrap().unit_rect.clone();
    let before = unit_rect(world).expect("unit_rect built");

    world.resource_mut::<GraphicsCore>().invalidate();
    let mut invalidate = Schedule::default();
    invalidate.add_systems(invalidate_dependent_components);
    invalidate.run(world);
    assert!(unit_rect(world).is_none(), "the loss drops unit_rect");

    *world.resource_mut::<GraphicsCore>() = GraphicsCore::new().expect("GraphicsCore");
    world.entity_mut(node).insert(solid_rect_content(rect()));
    world.run_schedule(PreRenderSurface);
    let after = unit_rect(world).expect("unit_rect recreated after recovery");
    // 古い物は控えで生かしてあるので番地は使い回されない
    assert_ne!(before.as_raw(), after.as_raw());
}
