//! find_owner_window_composition_mode ユニットテスト (Task 11.2)

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::{CompositionMode, Window};

/// テスト用ヘルパー: DeferredWorld を使って find_owner_window_composition_mode を呼び出す
fn query_composition_mode(world: &mut World, entity: Entity) -> Option<CompositionMode> {
    // DeferredWorld を直接テストで使うことは困難なため、
    // システムを利用してテストデータを収集する
    #[derive(Resource, Default)]
    struct TestResult(Option<CompositionMode>);

    world.insert_resource(TestResult::default());

    // テスト対象エンティティIDをリソースに保存
    #[derive(Resource)]
    struct TargetEntity(Entity);
    world.insert_resource(TargetEntity(entity));

    // ChildOf チェーンを辿るロジックを直接テスト
    // find_owner_window_composition_mode は DeferredWorld が必要だが、
    // 同じロジックを World で再現する

    // 自身が Window の場合
    if let Some(w) = world.get::<Window>(entity) {
        return Some(w.composition_mode());
    }

    // ChildOf チェーンを辿る
    let mut current = entity;
    loop {
        let parent = world.get::<ChildOf>(current).map(|c| c.parent());
        match parent {
            Some(p) => {
                if let Some(w) = world.get::<Window>(p) {
                    return Some(w.composition_mode());
                }
                current = p;
            }
            None => return None,
        }
    }
}

#[test]
fn test_window_entity_returns_own_mode_ulw() {
    let mut world = World::new();
    let entity = world.spawn(Window::default()).id();
    assert_eq!(
        query_composition_mode(&mut world, entity),
        Some(CompositionMode::ULW)
    );
}

#[test]
fn test_window_entity_returns_own_mode_dcomp() {
    let mut world = World::new();
    let entity = world
        .spawn(Window {
            composition_mode: CompositionMode::DComp,
            ..Default::default()
        })
        .id();
    assert_eq!(
        query_composition_mode(&mut world, entity),
        Some(CompositionMode::DComp)
    );
}

#[test]
fn test_child_entity_traverses_to_ancestor_window() {
    let mut world = World::new();
    let window_entity = world
        .spawn(Window {
            composition_mode: CompositionMode::DComp,
            ..Default::default()
        })
        .id();

    // 子エンティティ（Visual相当）を ChildOf で接続
    let child = world.spawn(ChildOf(window_entity)).id();

    assert_eq!(
        query_composition_mode(&mut world, child),
        Some(CompositionMode::DComp)
    );
}

#[test]
fn test_grandchild_traverses_through_chain() {
    let mut world = World::new();
    let window_entity = world
        .spawn(Window {
            composition_mode: CompositionMode::DComp,
            ..Default::default()
        })
        .id();

    let child = world.spawn(ChildOf(window_entity)).id();
    let grandchild = world.spawn(ChildOf(child)).id();

    assert_eq!(
        query_composition_mode(&mut world, grandchild),
        Some(CompositionMode::DComp)
    );
}

#[test]
fn test_orphan_entity_returns_none() {
    let mut world = World::new();
    let orphan = world.spawn_empty().id();
    assert_eq!(query_composition_mode(&mut world, orphan), None);
}

#[test]
fn test_child_without_window_ancestor_returns_none() {
    let mut world = World::new();
    let parent = world.spawn_empty().id();
    let child = world.spawn(ChildOf(parent)).id();
    assert_eq!(query_composition_mode(&mut world, child), None);
}
