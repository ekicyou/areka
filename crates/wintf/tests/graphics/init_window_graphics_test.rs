//! W3a-T: init_wuc_resource・init_window_graphics（systems/init.rs）のヘッドレステスト
//!
//! WindowGraphics 本体の作成は DesktopWindowTarget（実 HWND 必須）のため
//! ヘッドレスでは検証不能だが、前段の分岐はデバイス非依存に検証できる:
//! - ウィンドウ存在時の WucGraphicsResource 遅延初期化（窓の係から切り出した init_wuc_resource）
//! - ウィンドウ不在時の早期リターン（リソース未作成）
//! - 無効 HWND での CreateDesktopWindowTarget 失敗時のエラー経路（パニックなし）

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::window::{Window, WindowHandle};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{
    GraphicsCore, HasGraphicsResources, WindowGraphics, WucGraphicsResource, init_window_graphics,
    init_wuc_resource,
};

fn setup() -> (World, Schedule) {
    // init_wuc_resource は内部で WucGraphicsResource::new（DQTAT_COM_NONE）を呼ぶため
    // COM を MTA 初期化しておく。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(FrameCount(1));

    // 本番の GraphicsSetup と同じ順（WUC の資源 → 窓）
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems((init_wuc_resource, init_window_graphics).chain());
    (world, schedule)
}

fn spawn_window(world: &mut World) -> Entity {
    let entity = world
        .spawn((
            Window::default(),
            WindowHandle {
                hwnd: HWND(std::ptr::null_mut()),
                instance: HINSTANCE(std::ptr::null_mut()),
            },
            HasGraphicsResources,
        ))
        .id();
    world.flush();
    entity
}

#[test]
fn lazily_initializes_wuc_resource_when_window_exists() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut world, mut schedule) = setup();
        spawn_window(&mut world);

        assert!(world.get_resource::<WucGraphicsResource>().is_none());
        schedule.run(&mut world);

        let resource = world
            .get_resource::<WucGraphicsResource>()
            .expect("ウィンドウ検出時に WucGraphicsResource が遅延初期化される");
        assert!(resource.is_valid());
    });
}

#[test]
fn does_not_initialize_wuc_resource_without_windows() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut world, mut schedule) = setup();

        schedule.run(&mut world);

        assert!(
            world.get_resource::<WucGraphicsResource>().is_none(),
            "ウィンドウが無ければ WucGraphicsResource は作成されない"
        );
    });
}

#[test]
fn invalid_hwnd_fails_gracefully_without_window_graphics() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut world, mut schedule) = setup();
        let entity = spawn_window(&mut world);

        // 1回目: WucGraphicsResource を命令で入れ、同じ回の窓の係が CreateDesktopWindowTarget
        // (null HWND) に失敗 → error ログ + 継続。2回目も同じ（パニックなし）
        schedule.run(&mut world);
        schedule.run(&mut world);

        assert!(
            world.get::<WindowGraphics>(entity).is_none(),
            "無効 HWND では WindowGraphics は挿入されない（パニックなし）"
        );
    });
}

#[test]
fn invalid_graphics_core_defers_initialization() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut world, mut schedule) = setup();
        world.resource_mut::<GraphicsCore>().invalidate();
        spawn_window(&mut world);

        schedule.run(&mut world);

        assert!(
            world.get_resource::<WucGraphicsResource>().is_none(),
            "GraphicsCore 無効時は早期リターン"
        );
    });
}
