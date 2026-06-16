use super::super::*;
use super::make_global_arrangement;
use bevy_ecs::world::World;

// ========================================================================
// hit_test_entity_ex テスト
// ========================================================================

/// NamedRegions + HitRegionMap 矩形領域ヒット
#[test]
fn test_hit_test_entity_ex_named_regions_rect_hit() {
    let mut world = World::new();

    let region_map = HitRegionMap::builder()
        .rect("head", 20.0, 0.0, 60.0, 40.0) // DIP: (20,0)-(80,40) on 100x100
        .build()
        .unwrap();

    // Arrangement の on_add hookが GlobalArrangement を上書きするため、
    // GlobalArrangement のみをセットし、Arrangement.size は bounds からフォールバック
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::named_regions(),
            region_map,
        ))
        .id();

    // local (50, 20) → screen (50, 20) — "head" 領域内
    let point = PhysicalPoint::new(50.0, 20.0);
    match hit_test_entity_ex(&world, entity, point) {
        RegionHit::Hit(Some(name)) => assert_eq!(name, "head"),
        other => panic!(
            "Expected Hit(Some('head')), got {:?}",
            match other {
                RegionHit::Miss => "Miss",
                RegionHit::Hit(None) => "Hit(None)",
                RegionHit::Hit(Some(_)) => "Hit(Some(...))",
            }
        ),
    }
}

/// NamedRegions + HitRegionMap 領域外（bounds内、領域外 → 無名ヒット）
#[test]
fn test_hit_test_entity_ex_named_regions_no_region() {
    let mut world = World::new();

    let region_map = HitRegionMap::builder()
        .rect("head", 20.0, 0.0, 60.0, 40.0)
        .build()
        .unwrap();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::named_regions(),
            region_map,
        ))
        .id();

    // local (5, 80) — bounds内だが領域外
    let point = PhysicalPoint::new(5.0, 80.0);
    match hit_test_entity_ex(&world, entity, point) {
        RegionHit::Hit(None) => {} // 期待通り: 無名ヒット
        other => panic!(
            "Expected Hit(None), got {:?}",
            match other {
                RegionHit::Miss => "Miss",
                RegionHit::Hit(None) => "Hit(None)",
                RegionHit::Hit(Some(ref s)) => s.as_str(),
            }
        ),
    }
}

/// NamedRegions + HitRegionMap 不在 → Bounds フォールバック (1.3)
#[test]
fn test_hit_test_entity_ex_named_regions_no_region_map_fallback() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::named_regions(),
            // HitRegionMap なし
        ))
        .id();

    let point = PhysicalPoint::new(50.0, 50.0);
    match hit_test_entity_ex(&world, entity, point) {
        RegionHit::Hit(None) => {} // Bounds フォールバック
        other => panic!(
            "Expected Hit(None) (fallback), got {:?}",
            match other {
                RegionHit::Miss => "Miss",
                RegionHit::Hit(None) => "Hit(None)",
                RegionHit::Hit(Some(ref s)) => s.as_str(),
            }
        ),
    }
}

/// hit_test_entity_ex: Bounds モードでは region: None
#[test]
fn test_hit_test_entity_ex_bounds_mode_region_none() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::bounds(),
        ))
        .id();

    let point = PhysicalPoint::new(50.0, 50.0);
    match hit_test_entity_ex(&world, entity, point) {
        RegionHit::Hit(None) => {}
        _ => panic!("Expected Hit(None) for Bounds mode"),
    }
}

/// NamedRegions + 退化 bounds（幅0）→ Hit(None) フォールバック
///
/// bounds_width <= 0.0 の分岐（ゼロ除算回避のための早期 Hit(None)）を特性化する。
/// 退化 bounds (50,0)-(50,100) は幅0だが、contains は x==50 の点を内側と判定するため
/// 正規化座標計算の手前で Hit(None) を返す経路に到達する。
#[test]
fn test_hit_test_entity_ex_named_regions_degenerate_bounds_width() {
    let mut world = World::new();

    let region_map = HitRegionMap::builder()
        .rect("head", 0.0, 0.0, 100.0, 100.0)
        .build()
        .unwrap();

    let entity = world
        .spawn((
            make_global_arrangement(50.0, 0.0, 50.0, 100.0), // 幅0の退化 bounds
            HitTest::named_regions(),
            region_map,
        ))
        .id();

    // x==50 は contains を通過し、bounds_width<=0 分岐で Hit(None) フォールバック
    match hit_test_entity_ex(&world, entity, PhysicalPoint::new(50.0, 50.0)) {
        RegionHit::Hit(None) => {}
        other => panic!(
            "Expected Hit(None) for degenerate bounds, got {:?}",
            match other {
                RegionHit::Miss => "Miss",
                RegionHit::Hit(None) => "Hit(None)",
                RegionHit::Hit(Some(ref s)) => s.as_str(),
            }
        ),
    }
}

/// NamedRegions + 退化 bounds（高さ0）→ Hit(None) フォールバック
#[test]
fn test_hit_test_entity_ex_named_regions_degenerate_bounds_height() {
    let mut world = World::new();

    let region_map = HitRegionMap::builder()
        .rect("head", 0.0, 0.0, 100.0, 100.0)
        .build()
        .unwrap();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 50.0, 100.0, 50.0), // 高さ0の退化 bounds
            HitTest::named_regions(),
            region_map,
        ))
        .id();

    // y==50 は contains を通過し、bounds_height<=0 分岐で Hit(None) フォールバック
    match hit_test_entity_ex(&world, entity, PhysicalPoint::new(50.0, 50.0)) {
        RegionHit::Hit(None) => {}
        other => panic!(
            "Expected Hit(None) for degenerate bounds, got {:?}",
            match other {
                RegionHit::Miss => "Miss",
                RegionHit::Hit(None) => "Hit(None)",
                RegionHit::Hit(Some(ref s)) => s.as_str(),
            }
        ),
    }
}

/// hit_test_entity_ex: bounds外は Miss
#[test]
fn test_hit_test_entity_ex_outside_bounds() {
    let mut world = World::new();

    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::named_regions(),
        ))
        .id();

    let point = PhysicalPoint::new(200.0, 200.0);
    match hit_test_entity_ex(&world, entity, point) {
        RegionHit::Miss => {}
        _ => panic!("Expected Miss for outside bounds"),
    }
}
