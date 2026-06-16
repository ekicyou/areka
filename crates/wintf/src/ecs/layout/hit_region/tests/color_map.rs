use super::super::*;

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
// 整数境界・極値座標の特性化テスト（W4b-V）— ColorMap 分岐
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
