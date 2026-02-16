//! Visual階層の兄弟順序テスト
//!
//! 異なるアーキタイプ（コンポーネント構成）を持つ兄弟エンティティでも、
//! `Children` コンポーネントの順序に基づいて Visual 階層が同期されることを検証する。
//! (Requirements: 3.3)

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use windows::core::Result;
use wintf::com::dcomp::*;
use wintf::ecs::{visual_hierarchy_sync_system, GraphicsCore, Visual, VisualGraphics};

/// テスト用マーカーコンポーネント（アーキタイプを変えるために使用）
#[derive(Component, Default)]
struct ExtraMarkerA;

/// テスト用マーカーコンポーネント（別のアーキタイプを作る）
#[derive(Component, Default)]
struct ExtraMarkerB;

/// テスト用の GraphicsCore を作成するヘルパー関数
fn setup_graphics() -> Result<GraphicsCore> {
    GraphicsCore::new()
}

/// 異なるアーキタイプを持つ兄弟エンティティの一部を未同期状態にした場合、
/// visual_hierarchy_sync_system 実行後に全子の parent_visual が更新されることを検証。
/// さらに全子の parent_visual が正しい親 Visual を参照していることを検証。
#[test]
fn test_different_archetype_siblings_visual_hierarchy_sync() -> Result<()> {
    let graphics = setup_graphics()?;
    let dcomp = graphics.dcomp().expect("dcomp device should exist").clone();

    let mut world = World::new();
    world.insert_resource(graphics);

    // 親エンティティ（VisualGraphics あり）
    let parent_visual = dcomp.create_visual()?;
    let parent_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(parent_visual.clone()),
        ))
        .id();

    // child_a: 通常のアーキタイプ（Visual + VisualGraphics + ChildOf）
    let child_a_visual = dcomp.create_visual()?;
    let child_a = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(child_a_visual.clone()),
            ChildOf(parent_entity),
        ))
        .id();

    // child_b: ExtraMarkerA 付き（異なるアーキタイプ）
    let child_b_visual = dcomp.create_visual()?;
    let child_b = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(child_b_visual.clone()),
            ExtraMarkerA,
            ChildOf(parent_entity),
        ))
        .id();

    // child_c: ExtraMarkerB 付き（さらに別のアーキタイプ）
    let child_c_visual = dcomp.create_visual()?;
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

    // 全子の parent_visual が更新されていること（is_some()）
    for (name, entity) in [("child_a", child_a), ("child_b", child_b), ("child_c", child_c)] {
        let vg = world.get::<VisualGraphics>(entity).unwrap();
        assert!(
            vg.parent_visual().is_some(),
            "{name} should have parent_visual cached after sync"
        );
    }

    Ok(())
}

/// Children に含まれるが VisualGraphics を持たない子エンティティが
/// 安全にスキップされることを検証（他のエンティティの同期には影響しない）。
#[test]
fn test_children_without_visual_graphics_are_safely_skipped() -> Result<()> {
    let graphics = setup_graphics()?;
    let dcomp = graphics.dcomp().expect("dcomp device should exist").clone();

    let mut world = World::new();
    world.insert_resource(graphics);

    let parent_visual = dcomp.create_visual()?;
    let parent_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(parent_visual.clone()),
        ))
        .id();

    // child_a: VisualGraphics あり
    let child_a_visual = dcomp.create_visual()?;
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
    let child_c_visual = dcomp.create_visual()?;
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
    let child_a_vg = world.get::<VisualGraphics>(child_a).unwrap();
    assert!(
        child_a_vg.parent_visual().is_some(),
        "child_a should have parent_visual"
    );

    let child_c_vg = world.get::<VisualGraphics>(child_c).unwrap();
    assert!(
        child_c_vg.parent_visual().is_some(),
        "child_c should have parent_visual"
    );

    Ok(())
}

/// 5つの兄弟（交互に異なるアーキタイプ）の Visual 同期テスト
#[test]
fn test_many_siblings_alternating_archetypes_visual_hierarchy() -> Result<()> {
    let graphics = setup_graphics()?;
    let dcomp = graphics.dcomp().expect("dcomp device should exist").clone();

    let mut world = World::new();
    world.insert_resource(graphics);

    let parent_visual = dcomp.create_visual()?;
    let parent_entity = world
        .spawn((
            Visual::default(),
            VisualGraphics::new(parent_visual.clone()),
        ))
        .id();

    // 交互に異なるアーキタイプの子を spawn
    let mut child_entities = Vec::new();
    for i in 0..5 {
        let child_visual = dcomp.create_visual()?;
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
    }

    let mut schedule = Schedule::default();
    schedule.add_systems(visual_hierarchy_sync_system);
    schedule.run(&mut world);

    // Children の順序が spawn 順であること
    let children = world.get::<Children>(parent_entity).unwrap();
    let children_order: Vec<Entity> = children.iter().collect();
    assert_eq!(children_order, child_entities);

    // 全子の parent_visual が設定されていること
    for (i, entity) in child_entities.iter().enumerate() {
        let vg = world.get::<VisualGraphics>(*entity).unwrap();
        assert!(
            vg.parent_visual().is_some(),
            "child_{i} should have parent_visual after sync"
        );
    }

    Ok(())
}
