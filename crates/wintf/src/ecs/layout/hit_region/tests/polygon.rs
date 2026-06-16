use super::super::*;

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
