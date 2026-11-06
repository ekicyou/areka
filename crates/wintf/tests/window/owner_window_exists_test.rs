//! owner_window_exists のテスト
//!
//! 写しでなく本物の判定を通す: `Visual` を足して `on_visual_add` のフックを走らせ、
//! 窓の下の部品にだけ見た目の部品（`VisualGraphics`）が入るかを見る。

use std::sync::mpsc;
use std::time::Duration;

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use wintf::ecs::{Visual, VisualGraphics, Window};

/// `entity` に `Visual` を足し、本物のフックが見た目の部品を入れたかを返す。
fn gets_visual_graphics(world: &mut World, entity: Entity) -> bool {
    world.entity_mut(entity).insert(Visual::default());
    world.flush();
    world.get::<VisualGraphics>(entity).is_some()
}

#[test]
fn test_window_entity_is_owner() {
    let mut world = World::new();
    let entity = world.spawn(Window::default()).id();
    assert!(gets_visual_graphics(&mut world, entity));
}

#[test]
fn test_child_entity_traverses_to_ancestor_window() {
    let mut world = World::new();
    let window_entity = world.spawn(Window::default()).id();
    let child = world.spawn(ChildOf(window_entity)).id();
    assert!(gets_visual_graphics(&mut world, child));
}

#[test]
fn test_grandchild_traverses_through_chain() {
    let mut world = World::new();
    let window_entity = world.spawn(Window::default()).id();
    // 2 ホップの ChildOf チェーン（W3b-V 多段経路カバレッジ）
    let child = world.spawn(ChildOf(window_entity)).id();
    let grandchild = world.spawn(ChildOf(child)).id();
    assert!(gets_visual_graphics(&mut world, grandchild));
}

#[test]
fn test_orphan_entity_returns_false() {
    let mut world = World::new();
    let orphan = world.spawn_empty().id();
    assert!(!gets_visual_graphics(&mut world, orphan));
}

#[test]
fn test_child_without_window_ancestor_returns_false() {
    let mut world = World::new();
    let parent = world.spawn_empty().id();
    let child = world.spawn(ChildOf(parent)).id();
    assert!(!gets_visual_graphics(&mut world, child));
}

/// 親の輪（A→B→A）の部品に `Visual` を足しても World が返り、見た目の部品は入らない。
/// 止まらない作りを見分けるため別スレッドで回し、30 秒で返らなければ落とす。
#[test]
fn test_parent_cycle_returns_without_window() {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn(ChildOf(a)).id();
        world.entity_mut(a).insert(ChildOf(b));
        assert_eq!(world.get::<ChildOf>(a).map(|c| c.parent()), Some(b));
        let _ = tx.send(gets_visual_graphics(&mut world, a));
    });
    let got = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("親の輪で Visual を足した World が 30 秒で返らない");
    assert!(!got);
}
