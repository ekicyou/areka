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

/// 閉じ辺（最終頂点→始点）をまたぐ判定が正しいこと（W4b-T）
///
/// ray casting の `j = n-1` 初期化（最終頂点から始点への閉じ辺）が
/// 正しく評価されることを、始点・終点をまたぐ水平位置の点で特性化する。
#[test]
fn test_point_in_polygon_closing_edge() {
    // 上辺を共有しない台形: 閉じ辺 (0,100)->(0,0) が左端の縦辺
    let vertices = vec![(0.0, 0.0), (80.0, 0.0), (60.0, 100.0), (0.0, 100.0)];
    // 左端付近・閉じ辺の内側
    assert!(point_in_polygon(10.0, 50.0, &vertices));
    // 閉じ辺より左（外側）
    assert!(!point_in_polygon(-10.0, 50.0, &vertices));
}

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

/// ColorMapData::width()/height() アクセサ（W4b-T: 未検証だった）
#[test]
fn test_color_map_data_size_accessors() {
    let data = ColorMapData {
        index_map: vec![0u8; 12],
        region_names: vec![],
        width: 4,
        height: 3,
    };
    assert_eq!(data.width(), 4);
    assert_eq!(data.height(), 3);
}

/// ColorMapData::hit_test: region_id が region_names 範囲外の場合 None
///
/// index_map に region_names.len() を超える ID が混入した場合、
/// `region_names.get(id-1)` が None を返し領域なし扱いとなる防御経路を特性化する。
#[test]
fn test_color_map_data_hit_test_id_out_of_range_names() {
    let data = ColorMapData {
        index_map: vec![9], // id=9 だが region_names は 1 件のみ
        region_names: vec!["only".to_string()],
        width: 1,
        height: 1,
    };
    // id-1=8 は region_names 範囲外 → None
    assert_eq!(data.hit_test(0, 0), None);
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

// ========================================================================
// 整数境界・極値座標の特性化テスト（W4b-V）
//
// hit_test_region の ColorMap 分岐は正規化座標を `(rel * width as f32) as u32` で
// ピクセル座標へ変換する。Rust の浮動小数 → 整数キャストは飽和的（負値→0、
// 範囲超過→u32::MAX、NaN→0）であり、変換後は ColorMapData::hit_test の
// 範囲チェック（pixel >= width/height → None）で吸収される。
// 極値・負値・非有限の正規化座標がパニックせず None へ縮退することを固定する
// （現状の安全な飽和挙動の特性化。挙動変更ではない）。
// ========================================================================

/// ヘルパ: 2x2 単色カラーマップ（全画素 id=1 = "fill"）
fn make_fill_color_map_2x2() -> HitRegionMap {
    HitRegionMap {
        kind: RegionKind::ColorMap(ColorMapData {
            index_map: vec![1, 1, 1, 1],
            region_names: vec!["fill".to_string()],
            width: 2,
            height: 2,
        }),
    }
}

/// 範囲を大きく超える正の正規化座標は飽和キャストで u32::MAX 付近となり、
/// ColorMapData::hit_test の範囲チェックで None へ縮退する（パニックしない）。
#[test]
fn test_color_map_extreme_positive_rel_saturates_to_none() {
    let map = make_fill_color_map_2x2();
    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // rel = 1e10 → pixel = (1e10 * 2) as u32 = u32::MAX（飽和）→ 範囲外 → None
    assert_eq!(map.hit_test_region(1e10, 1e10, &entity_size), None);
    // f32::MAX でも同様に飽和して範囲外
    assert_eq!(
        map.hit_test_region(f32::MAX, f32::MAX, &entity_size),
        None
    );
}

/// 負の正規化座標は飽和キャストで 0 になり、境界(0,0)へ丸められて領域内に入り得る。
/// パニックせず安全に判定されることを固定する（負 → 0 飽和の特性化）。
#[test]
fn test_color_map_negative_rel_saturates_to_zero_pixel() {
    let map = make_fill_color_map_2x2();
    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // rel = -5.0 → pixel = (-10.0) as u32 = 0（飽和）→ (0,0) は範囲内・id=1 → "fill"
    assert_eq!(map.hit_test_region(-5.0, -5.0, &entity_size), Some("fill"));
}

/// 非有限（NaN）正規化座標は飽和キャストで 0 になり、パニックしない。
/// NaN → 0 飽和により (0,0) 画素として判定される現挙動を固定する。
#[test]
fn test_color_map_nan_rel_does_not_panic() {
    let map = make_fill_color_map_2x2();
    let entity_size = Size {
        width: 100.0,
        height: 100.0,
    };
    // NaN as u32 == 0 → (0,0) → id=1 → "fill"（パニックなし）
    assert_eq!(
        map.hit_test_region(f32::NAN, f32::NAN, &entity_size),
        Some("fill")
    );
    // +inf as u32 == u32::MAX（飽和）→ 範囲外 → None
    assert_eq!(
        map.hit_test_region(f32::INFINITY, f32::INFINITY, &entity_size),
        None
    );
}

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
    assert_eq!(
        map.hit_test_region(f32::NAN, f32::NAN, &entity_size),
        None
    );
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
