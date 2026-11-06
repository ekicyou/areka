//! 描画の木の配線の実 GPU のテスト（wintf-visual-tree 7.4・要件 1.8, 1.9, 3.4）
//!
//! 本物の登録（`EcsWorld` の `PreRenderSurface`）を実の装置と WUC の資源で回す。窓は HWND の無い
//! `Window` の部品だけ（画面に出さない）。装置の失いの立ち直りは表示しない実の窓で通す
//! （`device_lost_recovery_test.rs` と同じ形）。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::core::Interface;
use windows_numerics::Matrix3x2;
use wintf::ecs::layout::{
    Arrangement, BoxSize, BoxStyle, Dimension, GlobalArrangement, Size as LayoutSize,
};
use wintf::ecs::world::{EcsWorld, PreRenderSurface};
use wintf::ecs::{
    DrawTree, GraphicsCommandList, GraphicsCore, HasGraphicsResources, OwnSurface, Point, Rect,
    SizeI, SurfaceCreationStats, SurfaceGraphics, SurfaceGraphicsDirty, Visual, VisualDrawContent,
    VisualGraphics, VisualTransform, Window, WindowHandle, WindowPos, WucGraphicsResource,
};
use wintf::numerics::Aabb;

/// 4x4 の矩形を描く中身（命令の列は閉じただけ・組む係は中身の矩形しか見ない）
fn content(core: &GraphicsCore) -> VisualDrawContent {
    let dc = core.device_context().expect("device context");
    let list = unsafe { dc.CreateCommandList() }.expect("command list");
    unsafe { list.Close() }.expect("close");
    VisualDrawContent::new(
        Aabb(D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 4.0,
            bottom: 4.0,
        }),
        list,
    )
}

/// 実の装置と WUC の資源を入れ、HWND の無い窓の下に 32x32 の持ち主を置く
fn setup(extra: impl Bundle) -> (EcsWorld, Entity) {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    world.insert_resource(core);
    world.insert_resource(wuc);
    let window = world.spawn(Window::default()).id();
    let size = LayoutSize {
        width: 32.0,
        height: 32.0,
    };
    let owner = world
        .spawn((
            Visual::default(),
            Arrangement {
                size,
                ..Default::default()
            },
            ChildOf(window),
            extra,
        ))
        .id();
    world.flush();
    // レイアウトは回さないので、描画面の大きさは配置の結果で与える
    *world.get_mut::<GlobalArrangement>(owner).unwrap() = GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: 32.0,
            bottom: 32.0,
        },
    };
    world.run_schedule(PreRenderSurface);
    (ecs, owner)
}

fn add_node(world: &mut World, parent: Entity) -> Entity {
    let c = content(world.resource::<GraphicsCore>());
    world.spawn((Visual::default(), c, ChildOf(parent))).id()
}

fn tree_len(world: &World, root: Entity) -> usize {
    world
        .get::<DrawTree>(root)
        .expect("DrawTree")
        .commands
        .len()
}

fn surface_valid(world: &World, e: Entity) -> bool {
    world.get::<SurfaceGraphics>(e).unwrap().is_valid()
}

/// 節を足すと根の部品が入り描画面が作られ、全部外すと根の部品が消え描画面が片づく（1.8）
#[test]
fn draw_tree_enters_with_nodes_and_leaves_when_all_removed() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, owner) = setup(());
        let world = ecs.world_mut();
        assert!(world.get::<DrawTree>(owner).is_none());
        assert!(
            !surface_valid(world, owner),
            "中身の無い持ち主に描画面は無い"
        );

        let a = add_node(world, owner);
        let b = add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        assert!(tree_len(world, owner) > 0, "節を足すと根の部品が入る");
        assert!(
            surface_valid(world, owner),
            "木だけの根にも描画面が作られる"
        );
        assert!(
            world.get::<VisualGraphics>(a).is_none(),
            "節は描画面を持たない"
        );

        let deleted = world.resource::<SurfaceCreationStats>().deleted_count;
        world.despawn(a);
        world.despawn(b);
        world.run_schedule(PreRenderSurface);
        assert!(
            world.get::<DrawTree>(owner).is_none(),
            "全部外すと根の部品が消える"
        );
        assert!(!surface_valid(world, owner), "描画面が片づく");
        assert_eq!(
            world.resource::<SurfaceCreationStats>().deleted_count,
            deleted + 1
        );
        world.clear_entities();
    });
}

/// 節の部品が外れた回に（他に変化が無くても）木が組み直される（1.9）
#[test]
fn draw_tree_rebuilt_when_node_part_removed() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, owner) = setup(());
        let world = ecs.world_mut();
        add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        let one = tree_len(world, owner);
        let b = add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        assert!(tree_len(world, owner) > one);

        // 恒等の変形の外れは節の値を何も変えない＝外れの記録だけが組み直させる
        world.entity_mut(b).insert(VisualTransform::default());
        world.run_schedule(PreRenderSurface);
        world.entity_mut(b).remove::<VisualTransform>();
        world.increment_change_tick();
        let this_run = world.change_tick();
        world.run_schedule(PreRenderSurface);
        let rebuilt = world
            .entity(owner)
            .get_change_ticks::<DrawTree>()
            .unwrap()
            .changed;
        assert!(
            rebuilt.get() >= this_run.get(),
            "部品が外れた回に組み直されない"
        );

        world.entity_mut(b).remove::<VisualDrawContent>();
        world.run_schedule(PreRenderSurface);
        assert_eq!(tree_len(world, owner), one, "外れた中身が列に残った");
        world.clear_entities();
    });
}

/// 節が持ち主へ上がった回（目印か命令の列を足した）に、元の根の列から外れる（7.1 レビュー）
#[test]
fn promoted_node_drops_out_of_root_tree() {
    fn check(promote: impl Fn(&mut EntityWorldMut)) {
        let (mut ecs, owner) = setup(());
        let world = ecs.world_mut();
        add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        let one = tree_len(world, owner);
        let b = add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        assert!(tree_len(world, owner) > one);

        promote(&mut world.entity_mut(b));
        world.run_schedule(PreRenderSurface);
        assert!(
            world.get::<VisualGraphics>(b).is_some(),
            "上がった節は持ち主"
        );
        assert_eq!(
            tree_len(world, owner),
            one,
            "上がった節が元の根の列に残った"
        );
        world.clear_entities();
    }
    crate::common::on_gpu_owner_thread(move || {
        check(|e| {
            e.insert(OwnSurface);
        });
        check(|e| {
            e.insert(GraphicsCommandList::empty());
        });
    });
}

/// 中身を持つ節が持ち主へ上がったら（直下に節が無くても）自分の中身の 1 件で根の部品が入り、
/// 描画面が作られる。中身の差し替えで同一性が変わり、中身を外せば根の部品も描画面も消える（1.2b）
#[test]
fn promoted_node_with_own_content_gets_tree_and_surface() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, owner) = setup(());
        let world = ecs.world_mut();
        let b = add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        *world.get_mut::<GlobalArrangement>(b).unwrap() = GlobalArrangement {
            transform: Matrix3x2::identity(),
            bounds: Rect {
                left: 0.0,
                top: 0.0,
                right: 4.0,
                bottom: 4.0,
            },
        };

        world.entity_mut(b).insert(OwnSurface);
        world.run_schedule(PreRenderSurface);
        assert!(
            world.get::<DrawTree>(owner).is_none(),
            "上がった節は元の根の列から外れる"
        );
        assert_eq!(tree_len(world, b), 1, "自分の中身の 1 件");
        assert!(surface_valid(world, b), "上がった節に描画面が作られる");

        let hash = |w: &World| match &w.get::<DrawTree>(b).unwrap().commands[0] {
            wintf::ecs::DrawCommand::Draw(item) => item.hash,
            other => panic!("先頭が描く 1 件でない: {other:?}"),
        };
        let before = hash(world);
        let c = content(world.resource::<GraphicsCore>());
        world.entity_mut(b).insert(c);
        world.run_schedule(PreRenderSurface);
        assert_ne!(hash(world), before, "中身の差し替えで同一性が変わる");

        world.entity_mut(b).remove::<VisualDrawContent>();
        world.run_schedule(PreRenderSurface);
        assert!(
            world.get::<DrawTree>(b).is_none(),
            "中身を外せば根の部品が消える"
        );
        assert!(!surface_valid(world, b), "描画面が片づく");
        world.clear_entities();
    });
}

/// 命令の列を持つ根から木の列だけが外れたら、描画面を残して描き直しの印を立てる（7.2）
#[test]
fn cleanup_marks_dirty_when_only_draw_tree_removed() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, owner) = setup(GraphicsCommandList::empty());
        let world = ecs.world_mut();
        let node = add_node(world, owner);
        world.run_schedule(PreRenderSurface);
        assert!(world.get::<DrawTree>(owner).is_some());
        assert!(surface_valid(world, owner));

        world.despawn(node);
        world.increment_change_tick();
        let this_run = world.change_tick();
        world.run_schedule(PreRenderSurface);
        assert!(world.get::<DrawTree>(owner).is_none());
        assert!(
            surface_valid(world, owner),
            "命令の列が残る根の描画面は残る"
        );
        let dirty = world
            .entity(owner)
            .get_change_ticks::<SurfaceGraphicsDirty>()
            .unwrap()
            .changed;
        assert!(
            dirty.get() >= this_run.get(),
            "外れた木の絵を残さないよう描き直しの印が立つ"
        );
        world.clear_entities();
    });
}

/// 装置の失いの立ち直りの後に、木を持つ描画面が作り直され描き直される（3.4）
#[test]
fn device_loss_recovery_redraws_tree_surface() {
    crate::common::on_gpu_owner_thread(move || {
        use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};
        use windows::core::w;

        // 表示しない最上位の窓（WindowGraphics は実の HWND が要る）
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
        }
        .expect("hidden window");

        // 窓は命令の列を持たない（木だけの根）
        let mut ecs = EcsWorld::new();
        let root = ecs
            .world_mut()
            .spawn((
                Window::default(),
                WindowPos {
                    position: Some(Point { x: 0, y: 0 }),
                    size: Some(SizeI {
                        width: 32,
                        height: 32,
                    }),
                    ..Default::default()
                },
                WindowHandle {
                    hwnd,
                    instance: HINSTANCE::default(),
                },
                HasGraphicsResources,
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(32.0)),
                        height: Some(Dimension::Px(32.0)),
                    }),
                    ..Default::default()
                },
            ))
            .id();
        let mut node = None;
        let mut ready = false;
        for _ in 0..5 {
            ecs.try_tick_world();
            let world = ecs.world_mut();
            if node.is_none() && world.resource::<GraphicsCore>().is_valid() {
                node = Some(add_node(world, root));
            }
            ready = node.is_some()
                && surface_valid(world, root)
                && world
                    .get::<DrawTree>(root)
                    .is_some_and(|t| !t.commands.is_empty());
            if ready {
                break;
            }
        }
        assert!(ready, "the tree surface must be ready before the loss");
        let node = node.unwrap();
        let before = world_surface(ecs.world(), root);

        ecs.world_mut().resource_mut::<GraphicsCore>().invalidate();
        ecs.try_tick_world();
        assert!(
            ecs.world().get::<VisualDrawContent>(node).is_none(),
            "the loss drops the node content"
        );
        // 使い手が新しい装置で中身を記録し直す
        let world = ecs.world_mut();
        let c = content(world.resource::<GraphicsCore>());
        world.entity_mut(node).insert(c);
        let mut after = None;
        for _ in 0..3 {
            ecs.world_mut().increment_change_tick();
            let this_run = ecs.world_mut().change_tick();
            ecs.try_tick_world();
            let world = ecs.world();
            let redrawn = world
                .entity(root)
                .get_change_ticks::<SurfaceGraphicsDirty>()
                .unwrap()
                .changed
                .get()
                >= this_run.get();
            let tree_ok = world
                .get::<DrawTree>(root)
                .is_some_and(|t| !t.commands.is_empty());
            if surface_valid(world, root) && tree_ok && redrawn {
                after = Some(world_surface(world, root));
                break;
            }
        }
        let after = after.expect("the tree surface must be recreated and redrawn within 3 runs");
        assert_ne!(before.as_raw(), after.as_raw(), "surface recreated");
        assert!(
            ecs.world()
                .get::<VisualGraphics>(root)
                .and_then(|v| v.visual())
                .and_then(|v| v.cast::<windows::UI::Composition::SpriteVisual>().ok())
                .is_some_and(|s| s.Brush().is_ok()),
            "the recreated surface is attached"
        );

        drop((before, after));
        ecs.world_mut().clear_entities();
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    });
}

fn world_surface(world: &World, e: Entity) -> windows::UI::Composition::CompositionDrawingSurface {
    world
        .get::<SurfaceGraphics>(e)
        .unwrap()
        .surface()
        .unwrap()
        .clone()
}
