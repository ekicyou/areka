use super::*;

// ========================================================================
// point_in_polygon テスト
// ========================================================================

#[test]
fn test_point_in_polygon_convex_inside() {
    // 正方形 (0,0)-(100,100)
    let vertices = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    assert!(point_in_polygon(50.0, 50.0, &vertices));
}

#[test]
fn test_point_in_polygon_convex_outside() {
    let vertices = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    assert!(!point_in_polygon(150.0, 50.0, &vertices));
}

#[test]
fn test_point_in_polygon_concave_inside() {
    // L字形（凹多角形）
    let vertices = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (50.0, 50.0),
        (50.0, 100.0),
        (0.0, 100.0),
    ];
    // L字の内部
    assert!(point_in_polygon(25.0, 75.0, &vertices));
}

#[test]
fn test_point_in_polygon_concave_outside_notch() {
    // L字形の凹み部分
    let vertices = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 50.0),
        (50.0, 50.0),
        (50.0, 100.0),
        (0.0, 100.0),
    ];
    // 凹み部分（外部）
    assert!(!point_in_polygon(75.0, 75.0, &vertices));
}

#[test]
fn test_point_in_polygon_triangle() {
    let vertices = vec![(50.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    assert!(point_in_polygon(50.0, 50.0, &vertices));
    assert!(!point_in_polygon(5.0, 5.0, &vertices));
}

#[test]
fn test_point_in_polygon_insufficient_vertices() {
    let vertices = vec![(0.0, 0.0), (100.0, 0.0)];
    assert!(!point_in_polygon(50.0, 0.0, &vertices));
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

// ========================================================================
// Polygon hit_test_region 統一インターフェーステスト
// ========================================================================

#[test]
fn test_polygon_hit_test_region_inside() {
    // 三角形 (0,0)-(100,0)-(50,100) をエンティティ 100x100 で定義
    let map = HitRegionMap::builder()
        .polygon("tri", &[(0.0, 0.0), (100.0, 0.0), (50.0, 100.0)])
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 三角形の重心付近 (50, 33) → rel (0.50, 0.33)
    assert_eq!(map.hit_test_region(0.50, 0.33, &entity_size), Some("tri"));
}

#[test]
fn test_polygon_hit_test_region_outside() {
    let map = HitRegionMap::builder()
        .polygon("tri", &[(0.0, 0.0), (100.0, 0.0), (50.0, 100.0)])
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 三角形の外 (95, 90) → rel (0.95, 0.90)
    assert_eq!(map.hit_test_region(0.95, 0.90, &entity_size), None);
}

#[test]
fn test_polygon_hit_test_region_non_square_entity() {
    // エンティティが非正方形 (200x100) の場合の座標変換を検証
    // 多角形: (100,0)-(200,0)-(200,100)-(100,100) — 右半分の矩形
    let map = HitRegionMap::builder()
        .polygon(
            "right_half",
            &[(100.0, 0.0), (200.0, 0.0), (200.0, 100.0), (100.0, 100.0)],
        )
        .build()
        .unwrap();

    let entity_size = Size {
        width: 200.0,
        height: 100.0,
    };
    // 左半分 (50, 50) → rel (0.25, 0.50) → local (50, 50) … 外
    assert_eq!(map.hit_test_region(0.25, 0.50, &entity_size), None);
    // 右半分 (150, 50) → rel (0.75, 0.50) → local (150, 50) … 内
    assert_eq!(
        map.hit_test_region(0.75, 0.50, &entity_size),
        Some("right_half")
    );
}

#[test]
fn test_polygon_hit_test_region_multiple_regions() {
    // 4分割のポリゴンリージョン
    let map = HitRegionMap::builder()
        .polygon(
            "top-left",
            &[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)],
        )
        .polygon(
            "top-right",
            &[(50.0, 0.0), (100.0, 0.0), (100.0, 50.0), (50.0, 50.0)],
        )
        .polygon(
            "bottom-left",
            &[(0.0, 50.0), (50.0, 50.0), (50.0, 100.0), (0.0, 100.0)],
        )
        .polygon(
            "bottom-right",
            &[(50.0, 50.0), (100.0, 50.0), (100.0, 100.0), (50.0, 100.0)],
        )
        .build()
        .unwrap();

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    assert_eq!(
        map.hit_test_region(0.25, 0.25, &entity_size),
        Some("top-left")
    );
    assert_eq!(
        map.hit_test_region(0.75, 0.25, &entity_size),
        Some("top-right")
    );
    assert_eq!(
        map.hit_test_region(0.25, 0.75, &entity_size),
        Some("bottom-left")
    );
    assert_eq!(
        map.hit_test_region(0.75, 0.75, &entity_size),
        Some("bottom-right")
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
// ColorMapData テスト (ユニットテスト用のインライン構築)
// ========================================================================

#[test]
fn test_color_map_data_hit_test_mapped_color() {
    // 2x2 のテストデータを直接構築
    let data = ColorMapData {
        index_map: vec![1, 0, 0, 2], // (0,0)=頭, (1,0)=無名, (0,1)=無名, (1,1)=手
        region_names: vec!["head".to_string(), "hand".to_string()],
        width: 2,
        height: 2,
    };

    assert_eq!(data.hit_test(0, 0), Some("head"));
    assert_eq!(data.hit_test(1, 1), Some("hand"));
}

#[test]
fn test_color_map_data_hit_test_unmapped_color() {
    let data = ColorMapData {
        index_map: vec![1, 0, 0, 2],
        region_names: vec!["head".to_string(), "hand".to_string()],
        width: 2,
        height: 2,
    };

    // マッピング外 → None
    assert_eq!(data.hit_test(1, 0), None);
}

#[test]
fn test_color_map_data_hit_test_out_of_bounds() {
    let data = ColorMapData {
        index_map: vec![1, 0, 0, 2],
        region_names: vec!["head".to_string(), "hand".to_string()],
        width: 2,
        height: 2,
    };

    // 範囲外座標 → None
    assert_eq!(data.hit_test(5, 5), None);
}

// ========================================================================
// HitRegionMap ColorMap方式テスト
// ========================================================================

#[test]
fn test_color_map_hit_test_region() {
    // 4x4 カラーマップを手動構築
    // 左半分 = "left" (id=1)、右半分 = "right" (id=2)
    let mut index_map = vec![0u8; 16];
    for y in 0..4u32 {
        for x in 0..4u32 {
            let i = (y * 4 + x) as usize;
            if x < 2 {
                index_map[i] = 1;
            } else {
                index_map[i] = 2;
            }
        }
    }

    let map = HitRegionMap {
        kind: RegionKind::ColorMap(ColorMapData {
            index_map,
            region_names: vec!["left".to_string(), "right".to_string()],
            width: 4,
            height: 4,
        }),
    };

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };

    // 左側 (rel_x=0.25 → pixel_x=1)
    assert_eq!(map.hit_test_region(0.25, 0.5, &entity_size), Some("left"));
    // 右側 (rel_x=0.75 → pixel_x=3)
    assert_eq!(map.hit_test_region(0.75, 0.5, &entity_size), Some("right"));
}

#[test]
fn test_color_map_hit_test_region_unmapped_via_interface() {
    // 4x4 カラーマップ: 中央2x2のみマッピング、周囲は未マッピング(id=0)
    let mut index_map = vec![0u8; 16];
    // (1,1),(2,1),(1,2),(2,2) を "center" (id=1) に設定
    index_map[5] = 1; // (1,1)
    index_map[6] = 1; // (2,1)
    index_map[9] = 1; // (1,2)
    index_map[10] = 1; // (2,2)

    let map = HitRegionMap {
        kind: RegionKind::ColorMap(ColorMapData {
            index_map,
            region_names: vec!["center".to_string()],
            width: 4,
            height: 4,
        }),
    };

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // 中央 → "center"
    assert_eq!(
        map.hit_test_region(0.375, 0.375, &entity_size),
        Some("center")
    );
    // 左上隅 (0,0) → 未マッピング → None
    assert_eq!(map.hit_test_region(0.0, 0.0, &entity_size), None);
    // 右下隅 (3,3) → 未マッピング → None
    assert_eq!(map.hit_test_region(0.875, 0.875, &entity_size), None);
}

#[test]
fn test_color_map_hit_test_region_non_square_entity() {
    // 4x2 カラーマップ（横長）: 左半分 "a", 右半分 "b"
    //  a a b b
    //  a a b b
    let index_map = vec![1, 1, 2, 2, 1, 1, 2, 2];

    let map = HitRegionMap {
        kind: RegionKind::ColorMap(ColorMapData {
            index_map,
            region_names: vec!["a".to_string(), "b".to_string()],
            width: 4,
            height: 2,
        }),
    };

    // エンティティが非正方形: 200x50
    let entity_size = Size {
        width: 200.0,
        height: 50.0,
    };
    // 左1/4 → pixel_x=1 → "a"
    assert_eq!(map.hit_test_region(0.25, 0.5, &entity_size), Some("a"));
    // 右3/4 → pixel_x=3 → "b"
    assert_eq!(map.hit_test_region(0.75, 0.5, &entity_size), Some("b"));
}

#[test]
fn test_color_map_hit_test_region_four_quadrants() {
    // 4x4 カラーマップ: 4象限
    // TL=1, TR=2, BL=3, BR=4
    let mut index_map = vec![0u8; 16];
    for y in 0..4u32 {
        for x in 0..4u32 {
            let i = (y * 4 + x) as usize;
            index_map[i] = match (x < 2, y < 2) {
                (true, true) => 1,   // top-left
                (false, true) => 2,  // top-right
                (true, false) => 3,  // bottom-left
                (false, false) => 4, // bottom-right
            };
        }
    }

    let map = HitRegionMap {
        kind: RegionKind::ColorMap(ColorMapData {
            index_map,
            region_names: vec![
                "top-left".to_string(),
                "top-right".to_string(),
                "bottom-left".to_string(),
                "bottom-right".to_string(),
            ],
            width: 4,
            height: 4,
        }),
    };

    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    assert_eq!(
        map.hit_test_region(0.25, 0.25, &entity_size),
        Some("top-left")
    );
    assert_eq!(
        map.hit_test_region(0.75, 0.25, &entity_size),
        Some("top-right")
    );
    assert_eq!(
        map.hit_test_region(0.25, 0.75, &entity_size),
        Some("bottom-left")
    );
    assert_eq!(
        map.hit_test_region(0.75, 0.75, &entity_size),
        Some("bottom-right")
    );
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
