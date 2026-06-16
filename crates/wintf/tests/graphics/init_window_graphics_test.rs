//! W3a-T: init_window_graphics（systems/init.rs）のヘッドレステスト
//!
//! WindowGraphics 本体の作成は IDCompositionTarget（実 HWND 必須）のため
//! ヘッドレスでは検証不能だが、前段の分岐はデバイス非依存に検証できる:
//! - DComp ウィンドウ存在時の DCompGraphicsResource 遅延初期化
//! - DComp ウィンドウ不在時の早期リターン（リソース未作成）
//! - 無効 HWND での CreateTargetForHwnd 失敗時のエラー経路（パニックなし）

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use wintf::ecs::window::{CompositionMode, Window, WindowHandle};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{
    DCompGraphicsResource, GraphicsCore, HasGraphicsResources, WindowGraphics,
    init_window_graphics,
};
use windows::Win32::Foundation::{HINSTANCE, HWND};

fn setup() -> (World, Schedule) {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(FrameCount(1));

    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(init_window_graphics);
    (world, schedule)
}

fn spawn_window(world: &mut World, mode: CompositionMode) -> Entity {
    let entity = world
        .spawn((
            Window {
                composition_mode: mode,
                ..Default::default()
            },
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
fn lazily_initializes_dcomp_resource_when_dcomp_window_exists() {
    let (mut world, mut schedule) = setup();
    spawn_window(&mut world, CompositionMode::DComp);

    assert!(world.get_resource::<DCompGraphicsResource>().is_none());
    schedule.run(&mut world);

    let resource = world
        .get_resource::<DCompGraphicsResource>()
        .expect("DComp ウィンドウ検出時に DCompGraphicsResource が遅延初期化される");
    assert!(resource.is_valid());
}

#[test]
fn does_not_initialize_dcomp_resource_without_dcomp_windows() {
    let (mut world, mut schedule) = setup();
    spawn_window(&mut world, CompositionMode::ULW);

    schedule.run(&mut world);

    assert!(
        world.get_resource::<DCompGraphicsResource>().is_none(),
        "ULW ウィンドウのみなら DCompGraphicsResource は作成されない"
    );
}

#[test]
fn invalid_hwnd_fails_gracefully_without_window_graphics() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(&mut world, CompositionMode::DComp);

    // 1回目: DCompGraphicsResource 遅延初期化（このフレームは return）
    schedule.run(&mut world);
    // 2回目: CreateTargetForHwnd(null HWND) が失敗 → error ログ + 継続
    schedule.run(&mut world);

    assert!(
        world.get::<WindowGraphics>(entity).is_none(),
        "無効 HWND では WindowGraphics は挿入されない（パニックなし）"
    );
}

#[test]
fn invalid_graphics_core_defers_initialization() {
    let (mut world, mut schedule) = setup();
    world.resource_mut::<GraphicsCore>().invalidate();
    spawn_window(&mut world, CompositionMode::DComp);

    schedule.run(&mut world);

    assert!(
        world.get_resource::<DCompGraphicsResource>().is_none(),
        "GraphicsCore 無効時は早期リターン"
    );
}
