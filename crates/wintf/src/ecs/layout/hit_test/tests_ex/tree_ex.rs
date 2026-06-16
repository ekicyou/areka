use super::super::*;
use super::make_global_arrangement;
use crate::ecs::types::{Point, SizeI};
use bevy_ecs::world::World;

// ========================================================================
// hit_test_ex テスト
// ========================================================================

/// hit_test_ex: ツリー走査で最前面エンティティとリージョン名を返す
#[test]
fn test_hit_test_ex_returns_region_name() {
    let mut world = World::new();

    let region_map = HitRegionMap::builder()
        .rect("head", 0.0, 0.0, 100.0, 50.0)
        .rect("body", 0.0, 50.0, 100.0, 50.0)
        .build()
        .unwrap();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::named_regions(),
            region_map,
        ))
        .id();

    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[entity]);

    // head 領域
    let result = hit_test_ex(&world, root, PhysicalPoint::new(50.0, 25.0));
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.entity, entity);
    assert_eq!(result.region.as_deref(), Some("head"));

    // body 領域
    let result = hit_test_ex(&world, root, PhysicalPoint::new(50.0, 75.0));
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.entity, entity);
    assert_eq!(result.region.as_deref(), Some("body"));
}

/// hit_test_ex: Bounds モードでは region: None
#[test]
fn test_hit_test_ex_bounds_mode() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::bounds(),
        ))
        .id();

    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[entity]);

    let result = hit_test_ex(&world, root, PhysicalPoint::new(50.0, 50.0));
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.entity, entity);
    assert_eq!(result.region, None);
}

/// hit_test_ex: 後方互換 — 既存 hit_test と同じエンティティを返す
#[test]
fn test_hit_test_ex_backward_compat_same_entity() {
    let mut world = World::new();

    let back = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();
    let front = world
        .spawn(make_global_arrangement(20.0, 20.0, 80.0, 80.0))
        .id();

    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[back, front]);

    let point = PhysicalPoint::new(50.0, 50.0);

    let old_result = hit_test(&world, root, point);
    let new_result = hit_test_ex(&world, root, point);

    assert_eq!(old_result.unwrap(), new_result.unwrap().entity);
}

// ========================================================================
// hit_test_in_window_ex テスト
// ========================================================================

/// hit_test_in_window_ex: クライアント座標変換 + リージョン名
#[test]
fn test_hit_test_in_window_ex_with_region() {
    let mut world = World::new();

    let region_map = HitRegionMap::builder()
        .rect("button", 0.0, 0.0, 100.0, 50.0)
        .build()
        .unwrap();

    let window = world
        .spawn((
            make_global_arrangement(100.0, 200.0, 500.0, 500.0),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 400,
                    height: 300,
                }),
                ..Default::default()
            },
        ))
        .id();

    let widget = world
        .spawn((
            make_global_arrangement(150.0, 250.0, 250.0, 300.0), // 100x50
            HitTest::named_regions(),
            region_map,
        ))
        .id();

    world.entity_mut(window).add_children(&[widget]);

    // クライアント (50, 50) → スクリーン (150, 250)
    let result = hit_test_in_window_ex(&world, window, PhysicalPoint::new(50.0, 50.0));
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.entity, widget);
    assert_eq!(result.region.as_deref(), Some("button"));
}

/// hit_test_in_window_ex: WindowPos なし → None
#[test]
fn test_hit_test_in_window_ex_no_window_pos() {
    let mut world = World::new();
    let window = world
        .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
        .id();

    let result = hit_test_in_window_ex(&world, window, PhysicalPoint::new(50.0, 50.0));
    assert!(result.is_none());
}

/// hit_test_in_window_ex: WindowPos.position が None → None
///
/// 既存 ex テストは WindowPos コンポーネント不在のみ固定していた。
/// position フィールドが None（`window_pos.position?` で early return）の経路を特性化する。
#[test]
fn test_hit_test_in_window_ex_position_none() {
    let mut world = World::new();
    let window = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            WindowPos {
                position: None,
                ..Default::default()
            },
        ))
        .id();

    let result = hit_test_in_window_ex(&world, window, PhysicalPoint::new(50.0, 50.0));
    assert!(result.is_none());
}

/// 既存 hit_test / hit_test_in_window の後方互換性（NamedRegions以外の動作不変）
#[test]
fn test_existing_api_backward_compat() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::bounds(),
        ))
        .id();

    let root = world.spawn_empty().id();
    world.entity_mut(root).add_children(&[entity]);

    // 既存 API は変更なく動作
    let result = hit_test(&world, root, PhysicalPoint::new(50.0, 50.0));
    assert_eq!(result, Some(entity));

    // bounds外
    let result = hit_test(&world, root, PhysicalPoint::new(200.0, 200.0));
    assert_eq!(result, None);
}
