use super::super::*;
use super::make_global_arrangement;
use bevy_ecs::world::World;

// ========================================================================
// HitTestMode::AlphaMask テスト
// ========================================================================

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
    let point = PointF::new(50.0, 50.0);
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
    let point = PointF::new(200.0, 200.0);
    assert!(!hit_test_entity(&world, entity, point));
}
