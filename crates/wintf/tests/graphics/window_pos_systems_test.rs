//! window_pos.rs 系システムのギャップテスト (W3b-T)
//!
//! - invalidate_dependent_components: 装置が無効なとき窓・見た目・描画面・画像・WUC を
//!   全部倒し、命令の列を外すこと。装置の資源が無いときの no-op 経路。
//! - apply_window_pos_changes: 積んだ SetWindowPosCommand を `drain_window_pos_commands()` で
//!   取り出し、飛ばす・積む・対象外の 3 つの分岐を確かめる。キューはスレッドごとなので、
//!   GPU の持ち主のスレッドでは包まない（包むと共有のスレッドに指令が残る）。
//!
//! 対象: crates/wintf/src/ecs/graphics/systems/window_pos.rs

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{
    BitmapSourceGraphics, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    apply_window_pos_changes, invalidate_dependent_components,
};

fn run_invalidate(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(invalidate_dependent_components);
    schedule.run(world);
}

fn run_apply_window_pos(world: &mut World) {
    let mut schedule = Schedule::default();
    // 指令のキューはスレッドごと。テストのスレッドで回して、同じスレッドで取り出す。
    schedule.set_executor(bevy_ecs::schedule::SingleThreadedExecutor::new());
    schedule.add_systems(apply_window_pos_changes);
    schedule.run(world);
}

/// テスト用の実 ID2D1Bitmap1 を作成する（CPU 読み出し不可・描画不可の最小構成）
fn create_test_bitmap(graphics: &GraphicsCore) -> ID2D1Bitmap1 {
    let dc = graphics.device_context().expect("device context");
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        dpiX: 96.0,
        dpiY: 96.0,
        ..Default::default()
    };
    let size = D2D_SIZE_U {
        width: 2,
        height: 2,
    };
    unsafe { dc.CreateBitmap(size, None, 0, &props) }.expect("test bitmap")
}

// ========================================================================
// invalidate_dependent_components
// ========================================================================

/// GraphicsCore 無効時に BitmapSourceGraphics が無効化される
#[test]
fn invalidate_invalidates_bitmap_source_when_graphics_invalid() {
    let mut graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let bitmap = create_test_bitmap(&graphics);

    let mut bsg = BitmapSourceGraphics::new();
    bsg.set_bitmap(bitmap);
    assert!(bsg.is_valid());

    graphics.invalidate();

    let mut world = World::new();
    world.insert_resource(graphics);
    let entity = world.spawn(bsg).id();

    run_invalidate(&mut world);

    let bsg = world.get::<BitmapSourceGraphics>(entity).unwrap();
    assert!(
        !bsg.is_valid(),
        "bitmap source should be invalidated when GraphicsCore is invalid"
    );
}

/// 無効化の確かめに使う数（見た目の部品に変化の印が立った数）
#[derive(Resource, Default)]
struct ChangedVisuals(usize);

fn count_changed_visuals(
    query: Query<(), Changed<wintf::ecs::VisualGraphics>>,
    mut counter: ResMut<ChangedVisuals>,
) {
    counter.0 = query.iter().count();
}

/// GraphicsCore 無効時に窓・見た目・描画面・画像・WUC が全部無効になり、命令の列が外れる。
/// 倒れた部品には二度と可変で触らない（2 回目の無効化で見た目の変化の印が立たない）。
#[test]
fn invalidate_invalidates_all_and_removes_command_lists() {
    crate::common::on_gpu_owner_thread(move || {
        use windows::Foundation::Size;
        use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
        use windows::UI::Composition::Visual;
        use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};
        use windows::core::{Interface, w};
        use wintf::ecs::world::FrameCount;
        use wintf::ecs::{
            GraphicsCommandList, HasGraphicsResources, SurfaceGraphics, VisualGraphics,
            WindowGraphics, WucGraphicsResource, init_window_graphics, init_wuc_resource,
        };

        // 表示しない最上位の窓（WindowGraphics は実の HWND が要る）
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                8,
                8,
                None,
                None,
                None,
                None,
            )
        }
        .expect("hidden window");

        let mut world = World::new();
        world.insert_resource(GraphicsCore::new().expect("GraphicsCore creation"));
        world.insert_resource(FrameCount(1));
        let window = world
            .spawn((
                Window::default(),
                WindowHandle {
                    hwnd,
                    instance: HINSTANCE::default(),
                },
                HasGraphicsResources,
            ))
            .id();
        let mut setup = Schedule::default();
        setup.set_executor(bevy_ecs::schedule::SingleThreadedExecutor::new());
        setup.add_systems((init_wuc_resource, init_window_graphics).chain());
        setup.run(&mut world);
        assert!(world.get::<WindowGraphics>(window).unwrap().is_valid());

        let wuc_resource = world.resource::<WucGraphicsResource>();
        let compositor = wuc_resource.compositor().expect("compositor").clone();
        let gd = wuc_resource
            .graphics_device()
            .expect("graphics_device")
            .clone();
        let visual: Visual = compositor
            .CreateSpriteVisual()
            .expect("visual")
            .cast()
            .expect("cast to Visual");
        let surface = gd
            .CreateDrawingSurface(
                Size {
                    Width: 4.0,
                    Height: 4.0,
                },
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )
            .expect("surface");
        let brush = compositor
            .CreateSurfaceBrushWithSurface(&surface)
            .expect("brush");
        let mut bsg = BitmapSourceGraphics::new();
        bsg.set_bitmap(create_test_bitmap(world.resource::<GraphicsCore>()));
        let entity = world
            .spawn((
                VisualGraphics::new(visual),
                SurfaceGraphics::new(surface, brush, (4, 4)),
                bsg,
                GraphicsCommandList::empty(),
            ))
            .id();

        // デバイスロストを模擬
        world.resource_mut::<GraphicsCore>().invalidate();
        world.init_resource::<ChangedVisuals>();
        let mut schedule = Schedule::default();
        schedule.set_executor(bevy_ecs::schedule::SingleThreadedExecutor::new());
        schedule.add_systems((invalidate_dependent_components, count_changed_visuals).chain());
        schedule.run(&mut world);

        assert!(!world.resource::<WucGraphicsResource>().is_valid());
        assert!(!world.get::<WindowGraphics>(window).unwrap().is_valid());
        assert!(!world.get::<VisualGraphics>(entity).unwrap().is_valid());
        assert!(!world.get::<SurfaceGraphics>(entity).unwrap().is_valid());
        assert!(
            !world
                .get::<BitmapSourceGraphics>(entity)
                .unwrap()
                .is_valid()
        );
        assert!(
            world.get::<GraphicsCommandList>(entity).is_none(),
            "command lists belong to the old device and must be removed"
        );

        schedule.run(&mut world);
        assert_eq!(
            world.resource::<ChangedVisuals>().0,
            0,
            "already-invalid parts must not be touched mutably again"
        );

        world.clear_entities();
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    });
}

/// GraphicsCore リソース不在時は早期リターン（Option<Res> = None 経路）
#[test]
fn invalidate_noop_without_graphics_resource() {
    let graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let bitmap = create_test_bitmap(&graphics);
    let mut bsg = BitmapSourceGraphics::new();
    bsg.set_bitmap(bitmap);
    assert!(bsg.is_valid());
    drop(graphics); // リソースとして挿入しない

    let mut world = World::new();
    let entity = world.spawn(bsg).id();

    run_invalidate(&mut world);

    let bsg = world.get::<BitmapSourceGraphics>(entity).unwrap();
    assert!(
        bsg.is_valid(),
        "missing GraphicsCore resource must be a no-op"
    );
}

// ========================================================================
// apply_window_pos_changes（積んだ指令を drain_window_pos_commands で取り出して確かめる）
// ========================================================================

/// CW_USEDEFAULT の WindowPos は飛ばす（指令を積まない）
#[test]
fn apply_window_pos_skips_cw_usedefault() {
    let mut world = World::new();
    world.spawn((
        Window::default(),
        WindowHandle {
            hwnd: HWND::default(),
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point {
                x: CW_USEDEFAULT,
                y: 0,
            }),
            size: None,
            ..Default::default()
        },
    ));

    run_apply_window_pos(&mut world);

    assert!(
        drain_window_pos_commands().is_empty(),
        "CW_USEDEFAULT must not enqueue a SetWindowPos command"
    );
}

/// 無効な HWND では座標の変換が失敗し、元の座標のまま 1 件積む
#[test]
fn apply_window_pos_fallback_on_invalid_hwnd() {
    let mut world = World::new();
    world.spawn((
        Window::default(),
        WindowHandle {
            hwnd: HWND::default(),
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point { x: 10, y: 20 }),
            size: Some(SizeI {
                width: 100,
                height: 50,
            }),
            ..Default::default()
        },
    ));

    run_apply_window_pos(&mut world);

    let commands = drain_window_pos_commands();
    assert_eq!(commands.len(), 1, "exactly one command must be enqueued");
    let cmd = &commands[0];
    assert_eq!(cmd.hwnd, HWND::default());
    assert_eq!(
        (cmd.x, cmd.y, cmd.width, cmd.height),
        (10, 20, 100, 50),
        "fallback must keep the original client coordinates"
    );
}

/// Window を持たないエンティティは With<Window> で対象外（指令を積まない）
#[test]
fn apply_window_pos_ignores_non_window_entities() {
    let mut world = World::new();
    world.spawn((
        WindowHandle {
            hwnd: HWND::default(),
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point { x: 1, y: 2 }),
            size: None,
            ..Default::default()
        },
    ));

    run_apply_window_pos(&mut world);

    assert!(
        drain_window_pos_commands().is_empty(),
        "entities without Window must not enqueue a command"
    );
}
