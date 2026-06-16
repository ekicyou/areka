//! W3a-T: compositor_init_system の ECS 駆動ヘッドレステスト
//!
//! 実 GraphicsCore（D3D11/D2D デバイス）を用い、compositor_init_system を
//! Schedule 経由で実行して WindowD3D11Compositor の作成・スキップ・リサイズ・
//! デバイスロスト復旧の各分岐を検証する。
//! HWND は不要（compositor_init_system は WindowHandle の存在のみ要求し中身を使わない）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use wintf::ecs::compositor::WindowD3D11Compositor;
use wintf::ecs::compositor_systems::compositor_init_system;
use wintf::ecs::window::{CompositionMode, Window, WindowHandle, WindowPos};
use wintf::ecs::{GraphicsCore, HasGraphicsResources, SizeI};
use windows::Win32::Foundation::{HINSTANCE, HWND};

/// GraphicsCore 入りワールドと compositor_init_system のみのシングルスレッド Schedule
fn setup() -> (World, Schedule) {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let mut world = World::new();
    world.insert_resource(core);

    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(compositor_init_system);
    (world, schedule)
}

/// ダミー HWND の WindowHandle（システムはハンドル値を参照しない）
fn dummy_handle() -> WindowHandle {
    WindowHandle {
        hwnd: HWND(std::ptr::null_mut()),
        instance: HINSTANCE(std::ptr::null_mut()),
    }
}

fn window_pos_with_size(width: i32, height: i32) -> WindowPos {
    WindowPos {
        size: Some(SizeI { width, height }),
        ..Default::default()
    }
}

fn spawn_window(world: &mut World, mode: CompositionMode, pos: WindowPos) -> Entity {
    let entity = world
        .spawn((
            Window {
                composition_mode: mode,
                ..Default::default()
            },
            dummy_handle(),
            pos,
            HasGraphicsResources,
        ))
        .id();
    world.flush();
    entity
}

#[test]
fn ulw_window_with_valid_size_gets_compositor() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(160, 120));

    schedule.run(&mut world);

    let compositor = world
        .get::<WindowD3D11Compositor>(entity)
        .expect("ULW ウィンドウに WindowD3D11Compositor が作成される");
    assert!(compositor.is_valid(), "作成直後は有効");
    assert_eq!(compositor.cached_size(), (160, 120), "WindowPos.size が反映される");
    assert_eq!(compositor.generation(), 0, "初期 generation は 0");
}

#[test]
fn dcomp_window_is_skipped() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(
        &mut world,
        CompositionMode::DComp,
        window_pos_with_size(160, 120),
    );

    schedule.run(&mut world);

    assert!(
        world.get::<WindowD3D11Compositor>(entity).is_none(),
        "DComp モードの Window には Compositor を作成しない"
    );
}

#[test]
fn window_without_size_is_skipped() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(
        &mut world,
        CompositionMode::ULW,
        WindowPos {
            size: None,
            ..Default::default()
        },
    );

    schedule.run(&mut world);

    assert!(
        world.get::<WindowD3D11Compositor>(entity).is_none(),
        "WindowPos.size == None の Window はスキップされる"
    );
}

#[test]
fn window_with_zero_size_is_skipped() {
    let (mut world, mut schedule) = setup();
    let zero_w = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(0, 100));
    let zero_h = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(100, 0));

    schedule.run(&mut world);

    assert!(
        world.get::<WindowD3D11Compositor>(zero_w).is_none(),
        "幅 0 はスキップ"
    );
    assert!(
        world.get::<WindowD3D11Compositor>(zero_h).is_none(),
        "高さ 0 はスキップ"
    );
}

#[test]
fn invalid_graphics_core_defers_creation() {
    let (mut world, mut schedule) = setup();
    world.resource_mut::<GraphicsCore>().invalidate();
    let entity = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(64, 64));

    schedule.run(&mut world);

    assert!(
        world.get::<WindowD3D11Compositor>(entity).is_none(),
        "GraphicsCore 無効時は何も作成しない（早期リターン）"
    );
}

#[test]
fn window_pos_size_change_triggers_resize_with_generation_increment() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(100, 100));

    schedule.run(&mut world);
    assert_eq!(
        world.get::<WindowD3D11Compositor>(entity).unwrap().cached_size(),
        (100, 100)
    );

    // WindowPos.size を変更 → Changed<WindowPos> で resize 分岐に入る
    world.get_mut::<WindowPos>(entity).unwrap().size = Some(SizeI {
        width: 300,
        height: 250,
    });
    schedule.run(&mut world);

    let compositor = world.get::<WindowD3D11Compositor>(entity).unwrap();
    assert_eq!(compositor.cached_size(), (300, 250), "新サイズで再作成される");
    assert_eq!(compositor.generation(), 1, "resize で generation がインクリメント");
    assert!(compositor.is_valid());
}

#[test]
fn same_size_change_does_not_resize() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(100, 100));

    schedule.run(&mut world);

    // 値を変えず Changed フラグのみ立てる → cached_size 一致で何もしない
    world.get_mut::<WindowPos>(entity).unwrap().set_changed();
    schedule.run(&mut world);

    let compositor = world.get::<WindowD3D11Compositor>(entity).unwrap();
    assert_eq!(compositor.generation(), 0, "サイズ不変なら resize しない");
    assert_eq!(compositor.cached_size(), (100, 100));
}

#[test]
fn device_lost_recreates_compositor_with_generation_carryover() {
    let (mut world, mut schedule) = setup();
    let entity = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(128, 96));

    schedule.run(&mut world);

    // デバイスロストをシミュレート: compositor 無効化 + HasGraphicsResources 変更通知
    world
        .get_mut::<WindowD3D11Compositor>(entity)
        .unwrap()
        .invalidate();
    world
        .get_mut::<HasGraphicsResources>(entity)
        .unwrap()
        .set_changed();

    schedule.run(&mut world);

    let compositor = world.get::<WindowD3D11Compositor>(entity).unwrap();
    assert!(compositor.is_valid(), "デバイスロスト後に再作成される");
    assert_eq!(
        compositor.generation(),
        1,
        "旧 generation(0) を引き継ぎ +1 される"
    );
    assert_eq!(compositor.cached_size(), (128, 96));
}

// ==========================================================================
// W3a-V: 負サイズ入力の特性化テスト
// ==========================================================================

/// W3a-V: 負の WindowPos.size は `as u32` でラップして巨大サイズとなり、
/// WindowD3D11Compositor::new 内の D2D CreateBitmap が Err となる。
/// システムは error ログのみで panic せず、compositor 未作成のままとなる。
#[test]
fn negative_window_pos_size_does_not_create_compositor_and_does_not_panic() {
    let (mut world, mut schedule) = setup();
    let neg_w = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(-1, 100));
    let neg_h = spawn_window(&mut world, CompositionMode::ULW, window_pos_with_size(100, -2));

    schedule.run(&mut world);

    assert!(
        world.get::<WindowD3D11Compositor>(neg_w).is_none(),
        "負の幅（u32 ラップで巨大値）は作成失敗で compositor 未作成"
    );
    assert!(
        world.get::<WindowD3D11Compositor>(neg_h).is_none(),
        "負の高さ（u32 ラップで巨大値）は作成失敗で compositor 未作成"
    );
}
