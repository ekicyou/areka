//! 装置の失いからの立ち直りのテスト（audit-wintf-graphics-world 1・6 番）
//!
//! 作る係は本番では装置が使える回だけ回すが、使う人が係を直に登録したときの守りとして、
//! 装置（`GraphicsCore`）の無い World で回しても落ちないことを固定する。
//! 立ち直りは本物の登録（`EcsWorld`）を、表示しない実の窓と四角の部品で通して確かめる。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::core::Interface;
use wintf::ecs::layout::{BoxSize, BoxStyle, Dimension, GlobalArrangement};
use wintf::ecs::widget::brushes::{Brush, Brushes};
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::world::EcsWorld;
use wintf::ecs::world::FrameCount;
use wintf::ecs::{
    GraphicsCommandList, GraphicsCore, HasGraphicsResources, Point, SizeI, SurfaceCreationStats,
    SurfaceGraphics, SurfaceGraphicsDirty, Visual, VisualGraphics, Window, WindowGraphics,
    WindowHandle, WindowPos, WucGraphicsResource, deferred_surface_creation_system,
    init_window_graphics, visual_resource_management_system,
};

/// 装置を入れない World で見た目の資源の係を直に回しても落ちず、見た目は作られない
#[test]
fn visual_resource_system_without_graphics_core_does_not_panic() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    let entity = world
        .spawn((Visual::default(), VisualGraphics::default()))
        .id();

    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(visual_resource_management_system);
    schedule.run(&mut world);

    assert!(
        !world.get::<VisualGraphics>(entity).unwrap().is_valid(),
        "without GraphicsCore no visual must be created"
    );
}

/// 装置を入れない World で描画面を作る係を直に回しても落ちず、描画面は作られない
#[test]
fn surface_creation_system_without_graphics_core_does_not_panic() {
    let mut world = World::new();
    world.insert_resource(SurfaceCreationStats::default());
    let entity = world
        .spawn((
            VisualGraphics::default(),
            GraphicsCommandList::empty(),
            GlobalArrangement::default(),
            SurfaceGraphics::default(),
            SurfaceGraphicsDirty::default(),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(deferred_surface_creation_system);
    schedule.run(&mut world);

    assert!(
        !world.get::<SurfaceGraphics>(entity).unwrap().is_valid(),
        "without GraphicsCore no surface must be created"
    );
}

/// 装置を入れない World で窓の係を直に回しても落ちず、窓の部品は作られない
#[test]
fn window_graphics_system_without_graphics_core_does_not_panic() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    let entity = world
        .spawn((
            Window::default(),
            WindowHandle {
                hwnd: HWND::default(),
                instance: HINSTANCE::default(),
            },
            HasGraphicsResources,
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(init_window_graphics);
    schedule.run(&mut world);

    assert!(
        world.get::<WindowGraphics>(entity).is_none(),
        "without GraphicsCore no window graphics must be created"
    );
}

/// WUC の資源が無い回に見た目の係が回っても印を食べず、資源が入った次の回に見た目が作られる
#[test]
fn frame_without_wuc_does_not_eat_visual_marks() {
    crate::common::on_gpu_owner_thread(move || {
        // 窓が無いので WUC の資源は作られない（装置は 1 回目に作られる）
        let mut ecs = EcsWorld::new();
        let entity = ecs
            .world_mut()
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();
        ecs.try_tick_world();
        let world = ecs.world_mut();
        assert!(world.resource::<GraphicsCore>().is_valid());
        assert!(world.get_resource::<WucGraphicsResource>().is_none());
        assert!(!world.get::<VisualGraphics>(entity).unwrap().is_valid());

        let wuc = {
            let d2d = world.resource::<GraphicsCore>().d2d_device().expect("d2d");
            WucGraphicsResource::new(d2d).expect("WucGraphicsResource")
        };
        world.insert_resource(wuc);
        ecs.try_tick_world();

        assert!(
            ecs.world()
                .get::<VisualGraphics>(entity)
                .unwrap()
                .is_valid(),
            "the visual mark from the frame without WUC must still be seen"
        );
        ecs.world_mut().clear_entities();
    });
}

/// 見た目・描画面・命令の列・合成器（古い物は控えで生かしておき、番地の使い回しを防ぐ）
struct Parts {
    visual: windows::UI::Composition::Visual,
    surface: windows::UI::Composition::CompositionDrawingSurface,
    command_list: windows::Win32::Graphics::Direct2D::ID2D1CommandList,
    compositor: windows::UI::Composition::Compositor,
}

/// 全部が有効なら部品を返す
fn valid_parts(world: &World, entity: Entity) -> Option<Parts> {
    let wg = world.get::<WindowGraphics>(entity)?;
    let vg = world.get::<VisualGraphics>(entity)?;
    let sg = world.get::<SurfaceGraphics>(entity)?;
    let cl = world.get::<GraphicsCommandList>(entity)?;
    let wuc = world.get_resource::<WucGraphicsResource>()?;
    // 窓の的は同じ HWND に 1 つしか持てず古い物を控えで生かせないので、作り直しは世代の番号で見る
    if !(wg.is_valid() && vg.is_valid() && sg.is_valid() && wuc.is_valid()) {
        return None;
    }
    Some(Parts {
        visual: vg.visual()?.clone(),
        surface: sg.surface()?.clone(),
        command_list: cl.command_list()?.clone(),
        compositor: wuc.compositor()?.clone(),
    })
}

/// 描画面が有効で、見た目に筆が付いたままか（空にされていないか）
fn surface_attached(world: &World, entity: Entity) -> bool {
    let sg = world.get::<SurfaceGraphics>(entity).unwrap();
    let vg = world.get::<VisualGraphics>(entity).unwrap();
    sg.is_valid()
        && vg
            .visual()
            .and_then(|v| v.cast::<windows::UI::Composition::SpriteVisual>().ok())
            .is_some_and(|s| s.Brush().is_ok())
}

/// 隠した実の窓と四角の部品で装置を無効にすると、3 回以内に全部が新しい物になり世代は 1。
/// その後さらに 2 回回しても描画面は空にならない。
#[test]
fn device_loss_recovers_window_visual_surface_command_list_and_compositor() {
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

        let mut ecs = EcsWorld::new();
        let entity = ecs
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
                // 窓の大きさはレイアウトが決める（窓の通知を回さないので箱の大きさで与える）
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(32.0)),
                        height: Some(Dimension::Px(32.0)),
                    }),
                    ..Default::default()
                },
                Rectangle::new(),
                Brushes::with_foreground(Brush::RED.as_color().unwrap()),
            ))
            .id();

        let mut before = None;
        for _ in 0..5 {
            ecs.try_tick_world();
            before = valid_parts(ecs.world(), entity);
            if before.is_some() {
                break;
            }
        }
        let before = before.expect("all parts must become valid before the loss");
        let generation = |ecs: &EcsWorld| {
            ecs.world()
                .get::<WindowGraphics>(entity)
                .unwrap()
                .generation()
        };
        assert_eq!(generation(&ecs), 0);

        ecs.world_mut().resource_mut::<GraphicsCore>().invalidate();
        let mut after = None;
        for _ in 0..3 {
            ecs.try_tick_world();
            after = valid_parts(ecs.world(), entity);
            if after.is_some() {
                break;
            }
        }
        let after = after.expect("all parts must be recreated within 3 runs");
        assert_ne!(before.visual.as_raw(), after.visual.as_raw(), "visual");
        assert_ne!(before.surface.as_raw(), after.surface.as_raw(), "surface");
        assert_ne!(
            before.command_list.as_raw(),
            after.command_list.as_raw(),
            "command list"
        );
        assert_ne!(
            before.compositor.as_raw(),
            after.compositor.as_raw(),
            "compositor"
        );
        assert_eq!(generation(&ecs), 1);

        for _ in 0..2 {
            ecs.try_tick_world();
        }
        assert!(
            surface_attached(ecs.world(), entity),
            "the recreated surface must not be emptied"
        );

        drop((before, after));
        ecs.world_mut().clear_entities();
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    });
}
