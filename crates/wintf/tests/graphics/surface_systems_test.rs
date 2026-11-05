//! W3a-T: Surface 系 ECS システムのヘッドレステスト
//!
//! 実 WUC デバイスを用い、以下を Schedule 経由で実行して検証する。
//! - deferred_surface_creation_system: 作成・スキップ・同サイズ no-op・リサイズ + 統計
//! - cleanup_surface_on_commandlist_removed: CommandList 削除時の Surface クリア + 統計
//! - render_surface: Changed<SurfaceGraphicsDirty> 駆動の自己描画（begin_draw/end_draw）
//!
//! WUC 移行: commit_composition は廃止（要件 7.1・WUC の暗黙リフレクション）。
//! 旧 commit_composition テスト群は削除した。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::UI::Composition::Visual;
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::D2D1_DEVICE_CONTEXT_OPTIONS_NONE;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::Interface;
use windows_numerics::Matrix3x2;
use wintf::com::d2d::{D2D1CommandListExt, D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::ecs::layout::GlobalArrangement;
use wintf::ecs::world::FrameCount;
use wintf::ecs::{
    GraphicsCommandList, GraphicsCore, Rect, SurfaceCreationStats, SurfaceGraphics,
    SurfaceGraphicsDirty, VisualGraphics, WucGraphicsResource,
    cleanup_surface_on_commandlist_removed, deferred_surface_creation_system, mark_dirty_surfaces,
    render_surface,
};

fn setup_world() -> World {
    // WucGraphicsResource::new は DQTAT_COM_NONE を使うため COM 初期化済みスレッドを要求する。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let d2d = core.d2d_device().expect("d2d device");
    let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource 作成失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc_resource);
    world.insert_resource(SurfaceCreationStats::default());
    world.insert_resource(FrameCount(1));
    world
}

fn single_system_schedule<M>(
    system: impl IntoScheduleConfigs<bevy_ecs::system::ScheduleSystem, M>,
) -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(system);
    schedule
}

fn make_global_arrangement(width: f32, height: f32) -> GlobalArrangement {
    GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        },
    }
}

/// 実 WUC ビジュアル付きの Surface 生成対象エンティティをスポーンする
fn spawn_surface_target(world: &mut World, width: f32, height: f32) -> Entity {
    let visual: Visual = {
        let resource = world.resource::<WucGraphicsResource>();
        let compositor = resource.compositor().expect("compositor");
        compositor
            .CreateSpriteVisual()
            .expect("CreateSpriteVisual")
            .cast()
            .expect("cast to Visual")
    };
    world
        .spawn((
            VisualGraphics::new(visual),
            GraphicsCommandList::empty(),
            make_global_arrangement(width, height),
            SurfaceGraphics::default(),
            SurfaceGraphicsDirty::default(),
        ))
        .id()
}

fn stats(world: &World) -> SurfaceCreationStats {
    world.resource::<SurfaceCreationStats>().clone()
}

// ==========================================================================
// deferred_surface_creation_system
// ==========================================================================

#[test]
fn deferred_creation_creates_surface_with_physical_size() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);

        let mut schedule = single_system_schedule(deferred_surface_creation_system);
        schedule.run(&mut world);

        let surface = world.get::<SurfaceGraphics>(entity).unwrap();
        assert!(surface.is_valid(), "Surface が作成される");
        assert_eq!(
            surface.size,
            (100, 50),
            "GlobalArrangement.bounds の物理サイズ"
        );

        let s = stats(&world);
        assert_eq!(s.created_count, 1, "作成統計が記録される");
        assert_eq!(s.skipped_count, 0);
        assert_eq!(s.resize_count, 0);

        let dirty = world.get::<SurfaceGraphicsDirty>(entity).unwrap();
        assert_eq!(
            dirty.requested_frame, 1,
            "dirty の Changed トリガー用に +1 される"
        );
    });
}

#[test]
fn deferred_creation_ceils_fractional_bounds() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.5, 49.2);

        let mut schedule = single_system_schedule(deferred_surface_creation_system);
        schedule.run(&mut world);

        let surface = world.get::<SurfaceGraphics>(entity).unwrap();
        assert_eq!(
            surface.size,
            (101, 50),
            "小数点以下は切り上げ（物理ピクセル）"
        );
    });
}

#[test]
fn deferred_creation_skips_invalid_size_and_records_stat() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 0.0, 0.0);

        let mut schedule = single_system_schedule(deferred_surface_creation_system);
        schedule.run(&mut world);

        let surface = world.get::<SurfaceGraphics>(entity).unwrap();
        assert!(!surface.is_valid(), "サイズ 0 では Surface を作成しない");
        let s = stats(&world);
        assert_eq!(s.skipped_count, 1, "スキップ統計が記録される");
        assert_eq!(s.created_count, 0);
    });
}

#[test]
fn deferred_creation_is_noop_when_size_unchanged() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);

        let mut schedule = single_system_schedule(deferred_surface_creation_system);
        schedule.run(&mut world);
        assert_eq!(stats(&world).created_count, 1);

        // 値を変えずに Changed のみ立てる → サイズ一致で何もしない
        world
            .get_mut::<GlobalArrangement>(entity)
            .unwrap()
            .set_changed();
        schedule.run(&mut world);

        let s = stats(&world);
        assert_eq!(s.created_count, 1, "再作成されない");
        assert_eq!(s.resize_count, 0, "リサイズ扱いにもならない");
        assert_eq!(s.skipped_count, 0);
    });
}

#[test]
fn deferred_creation_resizes_on_bounds_change() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);

        let mut schedule = single_system_schedule(deferred_surface_creation_system);
        schedule.run(&mut world);

        *world.get_mut::<GlobalArrangement>(entity).unwrap() = make_global_arrangement(200.0, 80.0);
        schedule.run(&mut world);

        let surface = world.get::<SurfaceGraphics>(entity).unwrap();
        assert_eq!(surface.size, (200, 80), "新サイズで再作成される");
        let s = stats(&world);
        assert_eq!(s.created_count, 1, "初回作成のみ");
        assert_eq!(s.resize_count, 1, "リサイズ統計が記録される");
    });
}

// ==========================================================================
// cleanup_surface_on_commandlist_removed
// ==========================================================================

#[test]
fn cleanup_clears_surface_when_commandlist_removed() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);

        let mut create = single_system_schedule(deferred_surface_creation_system);
        create.run(&mut world);
        assert!(world.get::<SurfaceGraphics>(entity).unwrap().is_valid());

        world.entity_mut(entity).remove::<GraphicsCommandList>();

        let mut cleanup = single_system_schedule(cleanup_surface_on_commandlist_removed);
        cleanup.run(&mut world);

        let surface = world.get::<SurfaceGraphics>(entity).unwrap();
        assert!(!surface.is_valid(), "Surface 内容がクリアされる");
        assert_eq!(surface.size, (0, 0), "サイズもリセット");
        assert!(
            world.get::<VisualGraphics>(entity).is_some(),
            "VisualGraphics は Visual 階層維持のため残る"
        );
        assert_eq!(stats(&world).deleted_count, 1, "削除統計が記録される");
    });
}

#[test]
fn cleanup_skips_already_invalid_surface() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        // Surface 未作成（invalid）のまま CommandList を除去
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);
        world.entity_mut(entity).remove::<GraphicsCommandList>();

        let mut cleanup = single_system_schedule(cleanup_surface_on_commandlist_removed);
        cleanup.run(&mut world);

        assert_eq!(
            stats(&world).deleted_count,
            0,
            "既に invalid な Surface は削除統計に計上されない"
        );
    });
}

/// 命令の列を外して付け直した回（立ち直りで無効化の後に描き直した回）に、
/// 作り直したばかりの描画面を片づけの係が空にしない
#[test]
fn cleanup_keeps_surface_recreated_after_commandlist_reinserted() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);
        world.entity_mut(entity).remove::<GraphicsCommandList>();
        world
            .entity_mut(entity)
            .insert(GraphicsCommandList::empty());

        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.add_systems(
            (
                deferred_surface_creation_system,
                cleanup_surface_on_commandlist_removed,
            )
                .chain(),
        );
        schedule.run(&mut world);

        assert!(
            world.get::<SurfaceGraphics>(entity).unwrap().is_valid(),
            "surface recreated in the same run must not be cleared"
        );
        assert_eq!(stats(&world).deleted_count, 0);
    });
}

/// 見た目を作り直した部品は、大きさが変わらなくても描画面も作り直す
#[test]
fn deferred_creation_recreates_surface_for_recreated_visual() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = spawn_surface_target(&mut world, 100.0, 50.0);
        let mut schedule = single_system_schedule(deferred_surface_creation_system);
        schedule.run(&mut world);
        assert_eq!(stats(&world).created_count, 1);

        // 立ち直り相当: 描画面は空になり、見た目は新しい物に入れ替わる
        world.get_mut::<SurfaceGraphics>(entity).unwrap().clear();
        let visual: Visual = {
            let compositor = world
                .resource::<WucGraphicsResource>()
                .compositor()
                .expect("compositor")
                .clone();
            compositor
                .CreateSpriteVisual()
                .expect("CreateSpriteVisual")
                .cast()
                .expect("cast to Visual")
        };
        *world.get_mut::<VisualGraphics>(entity).unwrap() = VisualGraphics::new(visual);
        schedule.run(&mut world);

        assert!(
            world.get::<SurfaceGraphics>(entity).unwrap().is_valid(),
            "surface must be recreated for the recreated visual"
        );
        assert_eq!(stats(&world).created_count, 2);
    });
}

// ==========================================================================
// mark_dirty_surfaces（11 番）
// ==========================================================================

fn set_transform(world: &mut World, entity: Entity, scale: f32, offset: f32) {
    world
        .get_mut::<GlobalArrangement>(entity)
        .unwrap()
        .transform = Matrix3x2 {
        M11: scale,
        M12: 0.0,
        M21: 0.0,
        M22: scale,
        M31: offset,
        M32: offset,
    };
}

/// 位置だけ変えても描き直しの印は立たず、拡大率を変えると立つ（GPU 不要）
#[test]
fn mark_dirty_ignores_move_only_and_marks_on_scale_change() {
    let mut world = World::new();
    world.insert_resource(FrameCount(1));
    let entity = world
        .spawn((
            SurfaceGraphics::default(),
            SurfaceGraphicsDirty::default(),
            GraphicsCommandList::empty(),
            make_global_arrangement(100.0, 50.0),
        ))
        .id();
    let mut schedule = single_system_schedule(mark_dirty_surfaces);
    schedule.run(&mut world);
    assert_eq!(
        world
            .get::<SurfaceGraphicsDirty>(entity)
            .unwrap()
            .requested_frame,
        1
    );

    world.resource_mut::<FrameCount>().0 = 5;
    set_transform(&mut world, entity, 1.0, 30.0);
    schedule.run(&mut world);
    assert_eq!(
        world
            .get::<SurfaceGraphicsDirty>(entity)
            .unwrap()
            .requested_frame,
        1,
        "moving only must not request a redraw"
    );

    world.resource_mut::<FrameCount>().0 = 7;
    set_transform(&mut world, entity, 2.0, 30.0);
    schedule.run(&mut world);
    assert_eq!(
        world
            .get::<SurfaceGraphicsDirty>(entity)
            .unwrap()
            .requested_frame,
        7,
        "changing the scale must request a redraw"
    );
}

// ==========================================================================
// render_surface
// ==========================================================================

/// 内容入りの CommandList を記録する
fn make_filled_command_list(core: &GraphicsCore, w: f32, h: f32) -> GraphicsCommandList {
    let device = core.d2d_device().expect("d2d device");
    let dc = device
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("recording dc");
    let cl = device.create_command_list().expect("command list");
    unsafe {
        dc.SetTarget(&cl);
        dc.BeginDraw();
    }
    let brush = dc
        .create_solid_color_brush(
            &D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            None,
        )
        .expect("brush");
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: w,
            bottom: h,
        },
        &brush,
    );
    unsafe {
        dc.EndDraw(None, None).expect("EndDraw");
    }
    cl.close().expect("close");
    GraphicsCommandList::new(cl)
}

/// 実 WUC CompositionDrawingSurface + SurfaceBrush を作成する
fn create_real_surface(world: &World, w: u32, h: u32) -> SurfaceGraphics {
    let resource = world.resource::<WucGraphicsResource>();
    let compositor = resource.compositor().expect("compositor");
    let gd = resource.graphics_device().expect("graphics_device");
    let surface = gd
        .CreateDrawingSurface(
            Size {
                Width: w as f32,
                Height: h as f32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )
        .expect("CreateDrawingSurface");
    let brush = compositor
        .CreateSurfaceBrushWithSurface(&surface)
        .expect("CreateSurfaceBrushWithSurface");
    SurfaceGraphics::new(surface, brush, (w, h))
}

#[test]
fn render_surface_draws_dirty_surface_without_error() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let surface = create_real_surface(&world, 64, 64);
        let cmd = {
            let core = world.resource::<GraphicsCore>();
            make_filled_command_list(core, 64.0, 64.0)
        };
        let entity = world
            .spawn((
                surface,
                make_global_arrangement(64.0, 64.0),
                SurfaceGraphicsDirty::default(),
                cmd,
            ))
            .id();

        // スポーン直後は Changed<SurfaceGraphicsDirty> が立っている → 描画対象
        let mut schedule = single_system_schedule(render_surface);
        schedule.run(&mut world);

        // begin_draw → SetTransform → clear → DrawImage → end_draw が完走
        // （BeginDraw/EndDraw 失敗時は error! ログだが、不整合があれば後続 COM 呼び出しで panic する）
        assert!(
            world.get::<SurfaceGraphics>(entity).unwrap().is_valid(),
            "描画後も Surface は有効なまま"
        );
    });
}

#[test]
fn render_surface_skips_invalid_surface_without_error() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let entity = world
            .spawn((
                SurfaceGraphics::default(), // invalid
                make_global_arrangement(64.0, 64.0),
                SurfaceGraphicsDirty::default(),
            ))
            .id();

        let mut schedule = single_system_schedule(render_surface);
        schedule.run(&mut world);

        assert!(
            !world.get::<SurfaceGraphics>(entity).unwrap().is_valid(),
            "invalid Surface はスキップされる（パニックなし）"
        );
    });
}

// ==========================================================================
// commit_composition — WUC 移行で廃止（要件 7.1）
// ==========================================================================
// 旧テスト（commit_composition_without_dcomp_resource_is_noop /
// commit_composition_commits_valid_device / commit_composition_skips_invalidated_resource）は、
// commit_composition システムそのものが削除されたため撤去した。WUC は暗黙リフレクションで
// 反映するため明示 commit の概念がなく、これらのテストは対象を失った。
