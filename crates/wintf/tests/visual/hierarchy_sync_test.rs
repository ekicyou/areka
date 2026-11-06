//! Visual階層同期テスト (R6, R7)
//!
//! ChildOf変更を検知してVisual階層を同期するシステムをテストする。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::core::{Interface, Result};
use wintf::ecs::{Visual, VisualGraphics, visual_hierarchy_sync_system};

use super::common::{WucVisualFactory, wuc_children};
use wintf::ecs::GraphicsCore;

/// ChildOf追加時にVisual階層が同期されることを確認
#[test]
fn test_childof_addition_syncs_visual_hierarchy() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = GraphicsCore::new()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        // 親エンティティを作成
        let parent_visual = factory.create_visual()?;
        let parent_entity = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(parent_visual.clone()),
            ))
            .id();

        // 子エンティティを作成（親なし）
        let child_visual = factory.create_visual()?;
        let child_entity = world
            .spawn((Visual::default(), VisualGraphics::new(child_visual.clone())))
            .id();

        // ChildOf を追加
        world
            .entity_mut(child_entity)
            .insert(ChildOf(parent_entity));

        // 同期システムを実行
        let mut schedule = Schedule::default();
        schedule.add_systems(visual_hierarchy_sync_system);
        schedule.run(&mut world);

        // 子の VisualGraphics に親の Visual が控えられ、WUC の親の子になっていることを確認
        let child_vg = world.get::<VisualGraphics>(child_entity).unwrap();
        assert_eq!(
            child_vg.parent_visual().map(|v| v.as_raw()),
            Some(parent_visual.as_raw()),
            "parent_visual should be cached after sync"
        );
        assert_eq!(wuc_children(&parent_visual)?, vec![child_visual.as_raw()]);

        Ok(())
    })
}

/// VisualGraphicsにGPUリソースがないエンティティは階層同期をスキップすることを確認
/// Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
#[test]
fn test_entities_without_visual_graphics_are_skipped() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = GraphicsCore::new()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        // 親を作成（VisualGraphicsあり、GPUリソースあり）
        let parent_visual = factory.create_visual()?;
        let parent_entity = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(parent_visual.clone()),
            ))
            .id();

        // 子を作成（VisualGraphics::default()を明示追加、GPUリソースなし）
        // Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
        let child_entity = world
            .spawn((
                Visual::default(),
                VisualGraphics::default(),
                ChildOf(parent_entity),
            ))
            .id();

        // 同期システムを実行（パニックしないことを確認）
        let mut schedule = Schedule::default();
        schedule.add_systems(visual_hierarchy_sync_system);
        schedule.run(&mut world);

        // GPUリソースの無い子は同期されない（親の控えも、WUC の親の子も無い）
        let child_vg = world
            .get::<VisualGraphics>(child_entity)
            .expect("VisualGraphics component should exist (explicitly added)");
        assert!(
            child_vg.parent_visual().is_none(),
            "VisualGraphics without GPU resources should not be synced"
        );
        assert!(wuc_children(&parent_visual)?.is_empty());

        Ok(())
    })
}
