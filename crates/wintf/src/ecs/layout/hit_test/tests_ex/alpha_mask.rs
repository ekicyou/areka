use super::super::*;
use super::make_global_arrangement;
use bevy_ecs::world::World;

// ========================================================================
// HitTestMode::AlphaMask テスト
// ========================================================================

#[test]
fn test_hit_test_alpha_mask_constructor() {
    let hit_test = HitTest::alpha_mask();
    assert_eq!(hit_test.mode, HitTestMode::AlphaMask);
}

/// αマスク未設定時は矩形判定にフォールバック
#[test]
fn test_hit_test_alpha_mask_fallback_no_bitmap_source() {
    let mut world = World::new();

    // BitmapSourceResourceなし、αマスクモード
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::alpha_mask(),
        ))
        .id();

    // BitmapSourceResourceがないので矩形判定にフォールバック
    let point = PhysicalPoint::new(50.0, 50.0);
    assert!(hit_test_entity(&world, entity, point));
}

/// αマスク未生成時は矩形判定にフォールバック
#[test]
fn test_hit_test_alpha_mask_fallback_no_mask() {
    let mut world = World::new();

    // BitmapSourceResourceはあるがαマスク未生成
    // Note: 実際のIWICBitmapSourceを作成するのは困難なため、
    // ここではBitmapSourceResourceなしの場合と同じくフォールバックを確認

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::alpha_mask(),
        ))
        .id();

    let point = PhysicalPoint::new(50.0, 50.0);
    // フォールバックでヒット
    assert!(hit_test_entity(&world, entity, point));
}

/// αマスクモードでも矩形外はヒットしない
#[test]
fn test_hit_test_alpha_mask_outside_bounds() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::alpha_mask(),
        ))
        .id();

    // bounds外の座標
    let point = PhysicalPoint::new(200.0, 200.0);
    assert!(!hit_test_entity(&world, entity, point));
}

// ========================================================================
// HitTestMode::NamedRegions テスト
// ========================================================================

#[test]
fn test_hit_test_named_regions_constructor() {
    let hit_test = HitTest::named_regions();
    assert_eq!(hit_test.mode, HitTestMode::NamedRegions);
}
