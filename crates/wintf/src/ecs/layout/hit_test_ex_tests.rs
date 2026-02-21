use super::*;
use bevy_ecs::world::World;
use windows::Win32::Foundation::{POINT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows_numerics::Matrix3x2;

/// テスト用ヘルパー: 指定した bounds を持つ GlobalArrangement を作成
fn make_global_arrangement(left: f32, top: f32, right: f32, bottom: f32) -> GlobalArrangement {
    GlobalArrangement {
        transform: Matrix3x2::translation(left, top),
        bounds: D2D_RECT_F {
            left,
            top,
            right,
            bottom,
        },
    }
}

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
                position: Some(POINT { x: 100, y: 200 }),
                size: Some(SIZE { cx: 400, cy: 300 }),
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

// ========================================================================
// Opacity/Brushes α値判定テスト（HitTestMode::Bounds）
// ========================================================================

/// Visual.opacity=0.502 * foreground.a=1.0 → 合成α ≈ 0.502 ≥ 128/255 → HTCLIENT（ヒット）
#[test]
fn test_hit_test_entity_bounds_alpha_boundary_above() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.502,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
        ))
        .id();
    // Visual.on_add → Arrangement.on_add が GlobalArrangement::default() を挿入するため、
    // flush 後にテスト用の値で上書きする
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 0.502 * 1.0 = 0.502 ≥ 128/255 ≈ 0.50196 → ヒット
    assert!(hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=0.501 * foreground.a=1.0 → 合成α ≈ 0.501 < 128/255 → 透明領域（ミス）
#[test]
fn test_hit_test_entity_bounds_alpha_boundary_below() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.501,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 0.501 * 1.0 = 0.501 < 128/255 ≈ 0.50196 → 透明
    assert!(!hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=0.4 * foreground.a=1.0 → 合成α = 0.4 < 128/255 → 透明領域
#[test]
fn test_hit_test_entity_bounds_low_opacity() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.4,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            }),
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 0.4 * 1.0 = 0.4 < 128/255 → 透明
    assert!(!hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=1.0 * foreground.a=0.4 → 合成α = 0.4 < 128/255 → 透明領域（foreground側が低い）
#[test]
fn test_hit_test_entity_bounds_low_foreground_alpha() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 1.0,
                ..Default::default()
            },
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 0.4,
            }),
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // 合成α = 1.0 * 0.4 = 0.4 < 128/255 → 透明
    assert!(!hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Opacity なし + Brushes なし → デフォルト（1.0 * 1.0 = 1.0）→ ヒット
#[test]
fn test_hit_test_entity_bounds_no_opacity_no_brushes() {
    let mut world = World::new();
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 100.0, 100.0),
            HitTest::bounds(),
        ))
        .id();

    // Opacity=1.0(default), foreground.a=1.0(DEFAULT_FOREGROUND=BLACK) → ヒット
    assert!(hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}

/// Visual.opacity=0.502 + Brushes::Inherit → DEFAULT_FOREGROUND (a=1.0) → 合成α ≥ 閾値 → ヒット
#[test]
fn test_hit_test_entity_bounds_inherit_foreground() {
    use crate::ecs::graphics::Visual;
    use crate::ecs::widget::brushes::Brushes;

    let mut world = World::new();
    let entity = world
        .spawn((
            HitTest::bounds(),
            Visual {
                opacity: 0.502,
                ..Default::default()
            },
            Brushes::default(), // foreground = Inherit
        ))
        .id();
    world.flush();
    world
        .entity_mut(entity)
        .insert(make_global_arrangement(0.0, 0.0, 100.0, 100.0));

    // Inherit → DEFAULT_FOREGROUND (BLACK, a=1.0)
    // 合成α = 0.502 * 1.0 ≥ 128/255 → ヒット
    assert!(hit_test_entity(
        &world,
        entity,
        PhysicalPoint::new(50.0, 50.0)
    ));
}
