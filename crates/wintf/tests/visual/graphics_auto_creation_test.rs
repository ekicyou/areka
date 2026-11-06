//! VisualGraphics 自動作成テスト (R2)
//!
//! visual_resource_management_system は VisualGraphics が存在するエンティティにGPUリソースを作成し、
//! SurfaceGraphics は作成しないことをテストする。
//!
//! Phase 2: on_visual_add は VisualGraphics を自動挿入しなくなったため、
//! テスト内で明示的に VisualGraphics::default() を挿入する。

use bevy_ecs::prelude::*;
use windows::core::{Interface, Result};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{
    GraphicsCore, SurfaceGraphics, Visual, VisualGraphics, WucGraphicsResource,
    visual_resource_management_system,
};

/// テスト用のワールドをセットアップするヘルパー関数（GPU の持ち主のスレッドの中で呼ぶ）
fn setup_world_with_graphics() -> Result<World> {
    let graphics = GraphicsCore::new()?;
    let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
    let wuc_resource = WucGraphicsResource::new(d2d)?;
    let mut world = World::new();
    world.insert_resource(graphics);
    world.insert_resource(wuc_resource);
    world.insert_resource(FrameCount(1));
    Ok(world)
}

/// Visual追加時に VisualGraphics が存在すればGPUリソースが作成されることを確認
/// Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
#[test]
fn test_visual_triggers_visual_graphics_creation() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let mut world = setup_world_with_graphics()?;

        // Visual + VisualGraphics を持つ Entity を作成
        // Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
        let entity = world
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();

        // この時点では VisualGraphics の GPU リソースはまだない
        assert!(!world.get::<VisualGraphics>(entity).unwrap().is_valid());

        // システムを実行すると作成される
        let mut schedule = Schedule::default();
        schedule.add_systems(visual_resource_management_system);
        schedule.run(&mut world);

        // VisualGraphics の GPU リソースが作成されていることを確認
        assert!(
            world.get::<VisualGraphics>(entity).unwrap().is_valid(),
            "VisualGraphics should get GPU resources for entity with Visual"
        );

        Ok(())
    })
}

/// Visual追加時に SurfaceGraphics は即時作成されないことを確認（R2 AC#5, R5で遅延作成）
#[test]
fn test_visual_does_not_trigger_immediate_surface_creation() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let mut world = setup_world_with_graphics()?;

        // Visual を持つ Entity を作成
        let entity = world.spawn(Visual::default()).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(visual_resource_management_system);
        schedule.run(&mut world);

        assert!(
            world.get::<SurfaceGraphics>(entity).is_none(),
            "SurfaceGraphics should NOT be created immediately (deferred creation)"
        );

        Ok(())
    })
}

/// VisualGraphics が既に存在する場合は再作成されないことを確認
#[test]
fn test_visual_graphics_not_recreated_if_exists() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let mut world = setup_world_with_graphics()?;

        let entity = world
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();

        // 最初のシステム実行
        let mut schedule = Schedule::default();
        schedule.add_systems(visual_resource_management_system);
        schedule.run(&mut world);

        let first = world
            .get::<VisualGraphics>(entity)
            .and_then(|vg| vg.visual())
            .expect("First VisualGraphics should be valid")
            .as_raw();

        // 2回目の実行。変化の印を立て、作り直しを止めるのが有効の判定であることを試す
        world
            .get_mut::<VisualGraphics>(entity)
            .unwrap()
            .set_changed();
        schedule.run(&mut world);

        let second = world
            .get::<VisualGraphics>(entity)
            .and_then(|vg| vg.visual())
            .expect("Second VisualGraphics should be valid")
            .as_raw();
        assert_eq!(first, second, "VisualGraphics should not be recreated");

        Ok(())
    })
}

/// 複数の Entity に対して VisualGraphics がGPUリソース付きで作成されることを確認
#[test]
fn test_multiple_entities_get_visual_graphics() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let mut world = setup_world_with_graphics()?;

        let entity1 = world
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();
        let entity2 = world
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();
        let entity3 = world
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(visual_resource_management_system);
        schedule.run(&mut world);

        assert!(
            world.get::<VisualGraphics>(entity1).unwrap().is_valid(),
            "Entity1 should have VisualGraphics"
        );
        assert!(
            world.get::<VisualGraphics>(entity2).unwrap().is_valid(),
            "Entity2 should have VisualGraphics"
        );
        assert!(
            world.get::<VisualGraphics>(entity3).unwrap().is_valid(),
            "Entity3 should have VisualGraphics"
        );

        Ok(())
    })
}

/// GraphicsCore が無効（invalidate済み）の場合は VisualGraphics のGPUリソースが作成されないことを確認
/// Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
/// （合成器は有効なまま入れて、合成器が無いことでなく GraphicsCore の無効で止まることを見る）
#[test]
fn test_no_visual_graphics_with_invalid_graphics_core() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let mut world = setup_world_with_graphics()?;
        world.resource_mut::<GraphicsCore>().invalidate();

        let entity = world
            .spawn((Visual::default(), VisualGraphics::default()))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(visual_resource_management_system);
        schedule.run(&mut world);

        // ただし、GPUリソースは作成されない（GraphicsCoreが無効なため）
        assert!(
            !world.get::<VisualGraphics>(entity).unwrap().is_valid(),
            "VisualGraphics should not have GPU resources with invalid GraphicsCore"
        );

        Ok(())
    })
}
