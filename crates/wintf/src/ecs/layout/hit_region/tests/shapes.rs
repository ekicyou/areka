use super::super::*;

// ========================================================================
// Shapes 退化サイズ テスト
// ========================================================================

/// 退化 entity_size（幅0）でも Shapes 判定がパニックせず None を返す（W4b-T）
///
/// `local_x = rel_x * entity_size.width` が 0 に潰れるため、原点を含まない矩形では
/// 領域ヒットしない。ゼロ除算・パニックを起こさないことを特性化する。
#[test]
fn test_shapes_hit_test_degenerate_entity_size() {
    let map = HitRegionMap::builder()
        .rect("away", 10.0, 10.0, 30.0, 30.0)
        .build()
        .unwrap();

    let degenerate = Size {
        width: 0.0,
        height: 0.0,
    };
    // local 座標は (0,0) に潰れ、原点から外れた矩形にはヒットしない
    assert_eq!(map.hit_test_region(0.5, 0.5, &degenerate), None);
}

// ========================================================================
// ShapeRegion::Rect テスト
// ========================================================================

#[test]
fn test_rect_hit_test_inside() {
    let map = HitRegionMap::builder()
        .rect("test", 10.0, 20.0, 50.0, 30.0)
        .build()
        .unwrap();

    // エンティティサイズ 100x100、正規化座標に変換
    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // local (35, 35) → rel (0.35, 0.35)
    assert_eq!(map.hit_test_region(0.35, 0.35, &entity_size), Some("test"));
}

#[test]
fn test_rect_hit_test_outside() {
    let map = HitRegionMap::builder()
        .rect("test", 10.0, 20.0, 50.0, 30.0)
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // local (5, 5) → rel (0.05, 0.05) — 矩形外
    assert_eq!(map.hit_test_region(0.05, 0.05, &entity_size), None);
}

#[test]
fn test_rect_hit_test_boundary_inclusive() {
    let map = HitRegionMap::builder()
        .rect("test", 10.0, 20.0, 50.0, 30.0)
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 左上角 (10, 20) → rel (0.10, 0.20)
    assert_eq!(map.hit_test_region(0.10, 0.20, &entity_size), Some("test"));
    // 内部点 (59, 49) → rel (0.59, 0.49) — 右下角の近傍
    assert_eq!(map.hit_test_region(0.59, 0.49, &entity_size), Some("test"));
}

// ========================================================================
// HitRegionMapBuilder テスト
// ========================================================================

#[test]
fn test_builder_build_success() {
    let result = HitRegionMap::builder()
        .rect("head", 20.0, 0.0, 60.0, 40.0)
        .polygon("hand", &[(0.0, 50.0), (30.0, 80.0), (0.0, 80.0)])
        .build();
    assert!(result.is_ok());
}

#[test]
fn test_builder_invalid_rect_size_zero_width() {
    let result = HitRegionMap::builder()
        .rect("bad", 0.0, 0.0, 0.0, 10.0)
        .build();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, HitRegionError::InvalidRectSize { .. }));
}

#[test]
fn test_builder_invalid_rect_size_negative() {
    let result = HitRegionMap::builder()
        .rect("bad", 0.0, 0.0, -5.0, 10.0)
        .build();
    assert!(result.is_err());
}

#[test]
fn test_builder_insufficient_vertices() {
    let result = HitRegionMap::builder()
        .polygon("bad", &[(0.0, 0.0), (10.0, 0.0)])
        .build();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, HitRegionError::InsufficientVertices { .. }));
}

#[test]
fn test_builder_empty_builds_ok() {
    let result = HitRegionMap::builder().build();
    assert!(result.is_ok());
}

// ========================================================================
// HitRegionMap 先勝ちルール テスト
// ========================================================================

#[test]
fn test_shapes_first_match_wins() {
    // 重なる2つの矩形: "first" が先に定義
    let map = HitRegionMap::builder()
        .rect("first", 0.0, 0.0, 100.0, 100.0)
        .rect("second", 0.0, 0.0, 100.0, 100.0)
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 両方の領域内 → 先に定義された "first" が返る
    assert_eq!(map.hit_test_region(0.5, 0.5, &entity_size), Some("first"));
}

#[test]
fn test_shapes_mixed_rect_polygon() {
    let map = HitRegionMap::builder()
        .rect("rect_region", 0.0, 0.0, 50.0, 50.0)
        .polygon(
            "poly_region",
            &[(50.0, 0.0), (100.0, 0.0), (100.0, 50.0), (50.0, 50.0)],
        )
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 矩形内 → "rect_region"
    assert_eq!(
        map.hit_test_region(0.25, 0.25, &entity_size),
        Some("rect_region")
    );
    // 多角形内 → "poly_region"
    assert_eq!(
        map.hit_test_region(0.75, 0.25, &entity_size),
        Some("poly_region")
    );
}

#[test]
fn test_shapes_no_region_hit() {
    let map = HitRegionMap::builder()
        .rect("test", 10.0, 10.0, 30.0, 30.0)
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // すべての領域外
    assert_eq!(map.hit_test_region(0.01, 0.01, &entity_size), None);
}

#[test]
fn test_empty_shapes_returns_none() {
    let map = HitRegionMap::builder().build().unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    assert_eq!(map.hit_test_region(0.5, 0.5, &entity_size), None);
}

// ========================================================================
// HitRegionError Display テスト
// ========================================================================

#[test]
fn test_error_display() {
    let err = HitRegionError::InsufficientVertices { vertices: 2 };
    assert!(err.to_string().contains("2 < 3"));

    let err = HitRegionError::InvalidRectSize {
        width: -1.0,
        height: 10.0,
    };
    assert!(err.to_string().contains("-1"));
}

// ========================================================================
// 整数境界・極値座標の特性化テスト（W4b-V）— Shapes 分岐
// ========================================================================

/// Shapes 分岐の極値正規化座標もパニックしないこと（local 座標が極大/非有限でも
/// f32 比較は false 化するのみで添字アクセス等の危険操作がない）を固定する。
#[test]
fn test_shapes_extreme_and_nonfinite_rel_do_not_panic() {
    let map = HitRegionMap::builder()
        .rect("box", 10.0, 10.0, 30.0, 30.0)
        .build()
        .unwrap();
    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 極大座標 → 矩形外 → None
    assert_eq!(map.hit_test_region(1e10, 1e10, &entity_size), None);
    // 負座標 → 矩形外 → None
    assert_eq!(map.hit_test_region(-1e10, -1e10, &entity_size), None);
    // NaN → すべての比較が false → None（パニックなし）
    assert_eq!(map.hit_test_region(f32::NAN, f32::NAN, &entity_size), None);
}

/// 退化矩形（幅0・高さ0 の Rect 領域）でも build を通過し、境界上の点で
/// パニックせず判定されることを固定する。
/// 注: build() は width<=0/height<=0 を弾くため、退化は x==x+width 等の縮退点で表現する。
/// ここでは極小幅（境界包含 <= による点ヒット）の安全性を特性化する。
#[test]
fn test_shapes_zero_extent_rect_via_inclusive_boundary() {
    // 幅・高さともに極小だが正の矩形（build を通過）
    let map = HitRegionMap::builder()
        .rect("pt", 50.0, 50.0, f32::MIN_POSITIVE, f32::MIN_POSITIVE)
        .build()
        .unwrap();
    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // local (50,50) ちょうど → 左上角は包含（>= かつ <=）→ "pt"
    assert_eq!(map.hit_test_region(0.5, 0.5, &entity_size), Some("pt"));
    // 明確に外側
    assert_eq!(map.hit_test_region(0.1, 0.1, &entity_size), None);
}
