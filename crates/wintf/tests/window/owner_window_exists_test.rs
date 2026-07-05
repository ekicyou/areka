//! owner_window_exists ユニットテスト

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::Window;

/// テスト用ヘルパー: owner_window_exists の判定ロジックを World で再現する
///
/// production の `owner_window_exists` は `DeferredWorld` を要求するため、
/// 同じロジック（自身が Window → true／ChildOf 祖先に Window → true／
/// 見つからなければ false）を `World` で再現する。
fn query_owner_window_exists(world: &mut World, entity: Entity) -> bool {
    // 自身が Window の場合
    if world.get::<Window>(entity).is_some() {
        return true;
    }

    // ChildOf チェーンを辿る
    let mut current = entity;
    loop {
        let parent = world.get::<ChildOf>(current).map(|c| c.parent());
        match parent {
            Some(p) => {
                if world.get::<Window>(p).is_some() {
                    return true;
                }
                current = p;
            }
            None => return false,
        }
    }
}

#[test]
fn test_window_entity_is_owner() {
    let mut world = World::new();
    let entity = world.spawn(Window::default()).id();
    assert!(query_owner_window_exists(&mut world, entity));
}

#[test]
fn test_child_entity_traverses_to_ancestor_window() {
    let mut world = World::new();
    let window_entity = world.spawn(Window::default()).id();

    // 子エンティティ（Visual相当）を ChildOf で接続
    let child = world.spawn(ChildOf(window_entity)).id();

    assert!(query_owner_window_exists(&mut world, child));
}

#[test]
fn test_grandchild_traverses_through_chain() {
    let mut world = World::new();
    let window_entity = world.spawn(Window::default()).id();

    // 2 ホップの ChildOf チェーン（W3b-V 多段経路カバレッジ）
    let child = world.spawn(ChildOf(window_entity)).id();
    let grandchild = world.spawn(ChildOf(child)).id();

    assert!(query_owner_window_exists(&mut world, grandchild));
}

#[test]
fn test_orphan_entity_returns_false() {
    let mut world = World::new();
    let orphan = world.spawn_empty().id();
    assert!(!query_owner_window_exists(&mut world, orphan));
}

#[test]
fn test_child_without_window_ancestor_returns_false() {
    let mut world = World::new();
    let parent = world.spawn_empty().id();
    let child = world.spawn(ChildOf(parent)).id();
    assert!(!query_owner_window_exists(&mut world, child));
}
