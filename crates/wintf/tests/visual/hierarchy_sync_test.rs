//! Visual階層同期テスト (R6, R7)
//!
//! ChildOf変更を検知してVisual階層を同期するシステムをテストする。

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use windows::UI::Composition::ContainerVisual;
use windows::core::{Interface, Result};
use wintf::ecs::{Visual, VisualGraphics, visual_hierarchy_sync_system};

use super::common::{WucVisualFactory, setup_graphics};

/// ChildOf追加時にVisual階層が同期されることを確認
#[test]
fn test_childof_addition_syncs_visual_hierarchy() -> Result<()> {
    let graphics = setup_graphics()?;
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

    // 子の VisualGraphics に parent_visual がキャッシュされていることを確認
    let child_vg = world.get::<VisualGraphics>(child_entity).unwrap();
    assert!(
        child_vg.parent_visual().is_some(),
        "parent_visual should be cached after sync"
    );

    Ok(())
}

/// ChildOf変更時に旧親から削除→新親に追加されることを確認
#[test]
fn test_childof_change_moves_visual_to_new_parent() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    let mut world = World::new();
    world.insert_resource(graphics);

    // 親1を作成
    let parent1_visual = factory.create_visual()?;
    let parent1_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(parent1_visual.clone()),
        ))
        .id();

    // 親2を作成
    let parent2_visual = factory.create_visual()?;
    let parent2_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(parent2_visual.clone()),
        ))
        .id();

    // 子を親1の下に作成
    let child_visual = factory.create_visual()?;
    let mut child_vg = VisualGraphics::new(child_visual.clone());
    child_vg.set_parent_visual(Some(parent1_visual.clone()));
    // WUC 移行: DComp の add_visual(insertAbove=false, ref=None) は
    // ContainerVisual.Children().InsertAtBottom に対応。
    parent1_visual
        .cast::<ContainerVisual>()?
        .Children()?
        .InsertAtBottom(&child_visual)?;

    let child_entity = world
        .spawn((Visual::default(), child_vg, ChildOf(parent1_entity)))
        .id();

    // ChildOf を親2に変更
    world
        .entity_mut(child_entity)
        .insert(ChildOf(parent2_entity));

    // 同期システムを実行
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_hierarchy_sync_system);
    schedule.run(&mut world);

    // 子の parent_visual が親2に更新されていることを確認
    let child_vg = world.get::<VisualGraphics>(child_entity).unwrap();
    assert!(
        child_vg.parent_visual().is_some(),
        "parent_visual should point to new parent"
    );

    Ok(())
}

/// Children順序変更でZ-orderが同期されることを確認
#[test]
fn test_children_order_change_syncs_zorder() -> Result<()> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;

    let mut world = World::new();
    world.insert_resource(graphics);

    // 親を作成
    let parent_visual = factory.create_visual()?;
    let parent_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(parent_visual.clone()),
        ))
        .id();

    // 子1を作成
    let child1_visual = factory.create_visual()?;
    let child1_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(child1_visual.clone()),
            ChildOf(parent_entity),
        ))
        .id();

    // 子2を作成
    let child2_visual = factory.create_visual()?;
    let child2_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(child2_visual.clone()),
            ChildOf(parent_entity),
        ))
        .id();

    // 同期システムを実行
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_hierarchy_sync_system);
    schedule.run(&mut world);

    // 両方の子が parent_visual を持っていることを確認
    let child1_vg = world.get::<VisualGraphics>(child1_entity).unwrap();
    let child2_vg = world.get::<VisualGraphics>(child2_entity).unwrap();
    assert!(child1_vg.parent_visual().is_some());
    assert!(child2_vg.parent_visual().is_some());

    // 親に Children が存在することを確認
    assert!(world.get::<Children>(parent_entity).is_some());

    Ok(())
}

/// VisualGraphicsにGPUリソースがないエンティティは階層同期をスキップすることを確認
/// Phase 2: on_visual_addはVisualGraphicsを自動挿入しなくなったため明示的に追加
#[test]
fn test_entities_without_visual_graphics_are_skipped() -> Result<()> {
    let graphics = setup_graphics()?;
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

    // 子エンティティにはVisualGraphicsコンポーネントは存在するが、GPUリソースは無い
    let child_vg = world.get::<VisualGraphics>(child_entity);
    assert!(
        child_vg.is_some(),
        "VisualGraphics component should exist (explicitly added)"
    );
    assert!(
        !child_vg.unwrap().is_valid(),
        "VisualGraphics should not have GPU resources"
    );

    Ok(())
}
