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

/// 持ち主の `VisualTransform` は WUC では効かない: 変わった回に 1 度だけ `warn!` を残す（1.2c）。
#[test]
fn owner_transform_warns_once_per_change() {
    use log_capture_kit::{LineFormat, capture_lines};
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let window = world.spawn(Window::default()).id();
    let owner = world
        .spawn((
            Visual::default(),
            VisualTransform::default(),
            OwnSurface,
            ChildOf(window),
        ))
        .id();
    let warns = |world: &mut World| {
        let ((), lines) = capture_lines(LineFormat::LevelFields, || {
            world.run_schedule(PreRenderSurface)
        });
        lines
            .iter()
            .filter(|l| l.starts_with("level=WARN") && l.contains("持ち主の VisualTransform"))
            .count()
    };
    assert_eq!(warns(world), 1, "入った回に 1 度");
    assert_eq!(warns(world), 0, "変わらなければ残さない");
    world.get_mut::<VisualTransform>(owner).unwrap().0.rotation = 0.5;
    assert_eq!(warns(world), 1, "変わった回にまた 1 度");
}

/// 装置を入れ、窓の下の持ち主と描く中身つきの節を作って 1 回組む（根の部品が入るまで）。
fn built_owner_and_node(ecs: &mut EcsWorld) -> (Entity, Entity) {
    use crate::ecs::graphics::GraphicsCore;
    let world = ecs.world_mut();
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore"));
    let window = world.spawn(Window::default()).id();
    // 根の大きさが 0 だと描く 1 件が見える範囲の外で間引かれるので、中身より広くする
    let arrangement = crate::ecs::layout::Arrangement {
        size: crate::ecs::Size {
            width: 8.0,
            height: 8.0,
        },
        ..Default::default()
    };
    let owner = world
        .spawn((Visual::default(), arrangement, ChildOf(window)))
        .id();
    let node = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect()),
            ChildOf(owner),
        ))
        .id();
    world.run_schedule(PreRenderSurface);
    assert!(world.get::<DrawTree>(owner).is_some(), "最初の回に組む");
    (owner, node)
}

/// 根の列（`DrawTree`）と描き直しの印（`SurfaceGraphicsDirty`）が最後に変わった時点。
fn change_ticks(world: &World, owner: Entity) -> (u32, u32) {
    let e = world.entity(owner);
    (
        e.get_ref::<DrawTree>().unwrap().last_changed().get(),
        e.get_ref::<SurfaceGraphicsDirty>()
            .unwrap()
            .last_changed()
            .get(),
    )
}

/// 回す前の時点から列が組み直され、印も立ったか（どちらも新しい時点で書かれたか）。
fn assert_rebuilt_and_marked(world: &mut World, owner: Entity, what: &str) {
    let (tree0, dirty0) = change_ticks(world, owner);
    world.run_schedule(PreRenderSurface);
    let (tree1, dirty1) = change_ticks(world, owner);
    assert_ne!(tree1, tree0, "{what}: 列を組み直す");
    assert_ne!(dirty1, dirty0, "{what}: 描き直しの印が立つ");
}

/// 何も変えない回は組み直さず印も立てない（後の 2 つのテストの比べの基準）。
#[test]
fn unchanged_round_neither_rebuilds_nor_marks() {
    let mut ecs = EcsWorld::new();
    let (owner, _) = built_owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.run_schedule(PreRenderSurface);
    let before = change_ticks(world, owner);
    world.run_schedule(PreRenderSurface);
    assert_eq!(change_ticks(world, owner), before, "変化が無ければ書かない");
}

/// 描く中身の混ぜ方を変えた回に列が組み直され、新しい混ぜ方と同一性で描き直しの印が立つ（1.8）。
#[test]
fn mix_mode_change_rebuilds_and_marks_dirty() {
    use windows::Win32::Graphics::Direct2D::Common::D2D1_BLEND_MODE_MULTIPLY;
    let mut ecs = EcsWorld::new();
    let (owner, node) = built_owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.run_schedule(PreRenderSurface);
    let item = |w: &World| match &w.get::<DrawTree>(owner).unwrap().commands[..] {
        [DrawCommand::Draw(d)] => (d.hash, d.mix_mode),
        other => panic!("描く 1 件だけのはず: {other:?}"),
    };
    let (hash0, mix0) = item(world);
    assert_eq!(mix0, MixMode::default());

    world.get_mut::<VisualDrawContent>(node).unwrap().mix_mode =
        MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY);
    assert_rebuilt_and_marked(world, owner, "混ぜ方の変化");
    let (hash1, mix1) = item(world);
    assert_eq!(mix1, MixMode::Blend(D2D1_BLEND_MODE_MULTIPLY));
    assert_ne!(hash1, hash0, "同一性も変わる");
}

/// 節の不透明度を変えた回に列が組み直され、不透明度の層が入って描き直しの印が立つ（2.7）。
#[test]
fn node_opacity_change_rebuilds_and_marks_dirty() {
    let mut ecs = EcsWorld::new();
    let (owner, node) = built_owner_and_node(&mut ecs);
    let world = ecs.world_mut();
    world.run_schedule(PreRenderSurface);
    let layers = |w: &World| -> Vec<f32> {
        w.get::<DrawTree>(owner)
            .unwrap()
            .commands
            .iter()
            .filter_map(|c| match c {
                DrawCommand::PushOpacity(l) => Some(l.opacity),
                _ => None,
            })
            .collect()
    };
    assert!(layers(world).is_empty(), "不透明なら層は無い");

    world.get_mut::<Visual>(node).unwrap().opacity = 0.5;
    assert_rebuilt_and_marked(world, owner, "不透明度の変化");
    assert_eq!(layers(world), vec![0.5]);

    world.get_mut::<Visual>(node).unwrap().opacity = 0.25;
    assert_rebuilt_and_marked(world, owner, "不透明度の再変化");
    assert_eq!(layers(world), vec![0.25]);
}
