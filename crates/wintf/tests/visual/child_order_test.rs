//! Visual階層の兄弟順序テスト
//!
//! 異なるアーキタイプ（コンポーネント構成）を持つ兄弟エンティティでも、
//! `Children` コンポーネントの順序に基づいて Visual 階層が同期されることを検証する。
//! (Requirements: 3.3)

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use windows::core::{Interface, Result};
use wintf::ecs::{Visual, VisualGraphics, visual_hierarchy_sync_system};

use super::common::{WucVisualFactory, expected_wuc_order, wuc_children};
use wintf::ecs::GraphicsCore;

/// テスト用マーカーコンポーネント（アーキタイプを変えるために使用）
#[derive(Component, Default)]
struct ExtraMarkerA;

/// テスト用マーカーコンポーネント（別のアーキタイプを作る）
#[derive(Component, Default)]
struct ExtraMarkerB;

/// 異なるアーキタイプを持つ兄弟エンティティの一部を未同期状態にした場合、
/// visual_hierarchy_sync_system 実行後に全子の parent_visual が親の Visual を指し、
/// WUC の子の並びが Children の順を写していることを検証。
#[test]
fn test_different_archetype_siblings_visual_hierarchy_sync() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = GraphicsCore::new()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        // 親エンティティ（VisualGraphics あり）
        let parent_visual = factory.create_visual()?;
        let parent_entity = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(parent_visual.clone()),
            ))
            .id();

        // child_a: 通常のアーキタイプ（Visual + VisualGraphics + ChildOf）
        let child_a_visual = factory.create_visual()?;
        let child_a = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(child_a_visual.clone()),
                ChildOf(parent_entity),
            ))
            .id();

        // child_b: ExtraMarkerA 付き（異なるアーキタイプ）
        let child_b_visual = factory.create_visual()?;
        let child_b = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(child_b_visual.clone()),
                ExtraMarkerA,
                ChildOf(parent_entity),
            ))
            .id();

        // child_c: ExtraMarkerB 付き（さらに別のアーキタイプ）
        let child_c_visual = factory.create_visual()?;
        let child_c = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(child_c_visual.clone()),
                ExtraMarkerB,
                ChildOf(parent_entity),
            ))
            .id();

        // Visual 同期システムを実行
        let mut schedule = Schedule::default();
        schedule.add_systems(visual_hierarchy_sync_system);
        schedule.run(&mut world);

        // Children の順序を確認（spawn 順であること）
        let children = world
            .get::<Children>(parent_entity)
            .expect("Parent should have Children");
        let children_order: Vec<Entity> = children.iter().collect();
        assert_eq!(
            children_order,
            vec![child_a, child_b, child_c],
            "Children should reflect spawn order"
        );

        // 全子の parent_visual が親の Visual を指すこと
        for (name, entity) in [
            ("child_a", child_a),
            ("child_b", child_b),
            ("child_c", child_c),
        ] {
            let vg = world.get::<VisualGraphics>(entity).unwrap();
            assert_eq!(
                vg.parent_visual().map(|v| v.as_raw()),
                Some(parent_visual.as_raw()),
                "{name} should have parent_visual cached after sync"
            );
        }

        // WUC の子の並びが Children の順を写すこと
        assert_eq!(
            wuc_children(&parent_visual)?,
            expected_wuc_order(&[&child_a_visual, &child_b_visual, &child_c_visual])
        );

        Ok(())
    })
}

/// Children に含まれるが VisualGraphics を持たない子エンティティが
/// 安全にスキップされることを検証（他のエンティティの同期には影響しない）。
#[test]
fn test_children_without_visual_graphics_are_safely_skipped() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = GraphicsCore::new()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        let parent_visual = factory.create_visual()?;
        let parent_entity = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(parent_visual.clone()),
            ))
            .id();

        // child_a: VisualGraphics あり
        let child_a_visual = factory.create_visual()?;
        let child_a = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(child_a_visual.clone()),
                ChildOf(parent_entity),
            ))
            .id();

        // child_no_vg: VisualGraphics なし（ChildOf のみ。Visual なし → VisualGraphics 自動挿入もなし）
        let _child_no_vg = world.spawn(ChildOf(parent_entity)).id();

        // child_c: VisualGraphics あり
        let child_c_visual = factory.create_visual()?;
        let child_c = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(child_c_visual.clone()),
                ChildOf(parent_entity),
            ))
            .id();

        // パニックせずにシステムが実行できること
        let mut schedule = Schedule::default();
        schedule.add_systems(visual_hierarchy_sync_system);
        schedule.run(&mut world);

        // VisualGraphics を持つ子は正しく同期されていること
        for (name, entity) in [("child_a", child_a), ("child_c", child_c)] {
            let vg = world.get::<VisualGraphics>(entity).unwrap();
            assert_eq!(
                vg.parent_visual().map(|v| v.as_raw()),
                Some(parent_visual.as_raw()),
                "{name} should have parent_visual"
            );
        }
        assert_eq!(
            wuc_children(&parent_visual)?,
            expected_wuc_order(&[&child_a_visual, &child_c_visual])
        );

        Ok(())
    })
}

/// 5つの兄弟（交互に異なるアーキタイプ）の Visual 同期テスト
#[test]
fn test_many_siblings_alternating_archetypes_visual_hierarchy() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let graphics = GraphicsCore::new()?;
        let factory = WucVisualFactory::new(&graphics)?;

        let mut world = World::new();
        world.insert_resource(graphics);

        let parent_visual = factory.create_visual()?;
        let parent_entity = world
            .spawn((
                Visual::default(),
                VisualGraphics::new(parent_visual.clone()),
            ))
            .id();

        // 交互に異なるアーキタイプの子を spawn
        let mut child_entities = Vec::new();
        let mut child_visuals = Vec::new();
        for i in 0..5 {
            let child_visual = factory.create_visual()?;
            let entity = if i % 2 == 0 {
                world
                    .spawn((
                        Visual::default(),
                        VisualGraphics::new(child_visual.clone()),
                        ExtraMarkerA,
                        ChildOf(parent_entity),
                    ))
                    .id()
            } else {
                world
                    .spawn((
                        Visual::default(),
                        VisualGraphics::new(child_visual.clone()),
                        ChildOf(parent_entity),
                    ))
                    .id()
            };
            child_entities.push(entity);
            child_visuals.push(child_visual);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(visual_hierarchy_sync_system);
        schedule.run(&mut world);

        // Children の順序が spawn 順であること
        let children = world.get::<Children>(parent_entity).unwrap();
        let children_order: Vec<Entity> = children.iter().collect();
        assert_eq!(children_order, child_entities);

        // 全子の parent_visual が親の Visual を指すこと
        for (i, entity) in child_entities.iter().enumerate() {
            let vg = world.get::<VisualGraphics>(*entity).unwrap();
            assert_eq!(
                vg.parent_visual().map(|v| v.as_raw()),
                Some(parent_visual.as_raw()),
                "child_{i} should have parent_visual after sync"
            );
        }
        let refs: Vec<&_> = child_visuals.iter().collect();
        assert_eq!(wuc_children(&parent_visual)?, expected_wuc_order(&refs));

        Ok(())
    })
}
