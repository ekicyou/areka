use super::*;

// Task 6.1: Size構造体とArrangement.local_bounds()のテスト

#[test]
fn test_size_default() {
    let size = Size::default();
    assert_eq!(size.width, 0.0);
    assert_eq!(size.height, 0.0);
}

#[test]
fn test_size_copy_clone() {
    let size1 = Size {
        width: 100.0,
        height: 50.0,
    };
    let size2 = size1; // Copy
    let size3 = size1.clone(); // Clone

    assert_eq!(size1, size2);
    assert_eq!(size1, size3);
}

#[test]
fn test_arrangement_local_bounds_positive_size() {
    let arrangement = Arrangement {
        offset: Offset { x: 10.0, y: 20.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 100.0,
            height: 50.0,
        },
    };

    let bounds = arrangement.local_bounds();
    // local_bounds()は原点(0,0)基準（offsetを含まない）
    assert_eq!(bounds.left, 0.0);
    assert_eq!(bounds.top, 0.0);
    assert_eq!(bounds.right, 100.0);
    assert_eq!(bounds.bottom, 50.0);
}

#[test]
fn test_arrangement_local_bounds_zero_size() {
    let arrangement = Arrangement {
        offset: Offset { x: 10.0, y: 20.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 0.0,
            height: 0.0,
        },
    };

    let bounds = arrangement.local_bounds();
    // local_bounds()は原点(0,0)基準（offsetを含まない）
    assert_eq!(bounds.left, 0.0);
    assert_eq!(bounds.top, 0.0);
    assert_eq!(bounds.right, 0.0);
    assert_eq!(bounds.bottom, 0.0);
}

#[test]
fn test_arrangement_local_bounds_negative_size() {
    // 負のサイズでも計算は動作する（警告ログのみ）
    let arrangement = Arrangement {
        offset: Offset { x: 10.0, y: 20.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: -10.0,
            height: -5.0,
        },
    };

    let bounds = arrangement.local_bounds();
    // local_bounds()は原点(0,0)基準（offsetを含まない）
    assert_eq!(bounds.left, 0.0);
    assert_eq!(bounds.top, 0.0);
    assert_eq!(bounds.right, -10.0);
    assert_eq!(bounds.bottom, -5.0);
}

// Task 6.4: GlobalArrangementのtrait実装テスト

#[test]
fn test_global_arrangement_from_arrangement() {
    let arrangement = Arrangement {
        offset: Offset { x: 10.0, y: 20.0 },
        scale: LayoutScale { x: 2.0, y: 2.0 },
        size: Size {
            width: 100.0,
            height: 50.0,
        },
    };

    let global: GlobalArrangement = arrangement.into();

    // デバッグ出力
    eprintln!(
        "transform.M11={}, M12={}, M21={}, M22={}, M31={}, M32={}",
        global.transform.M11,
        global.transform.M12,
        global.transform.M21,
        global.transform.M22,
        global.transform.M31,
        global.transform.M32
    );
    eprintln!(
        "bounds.left={}, top={}, right={}, bottom={}",
        global.bounds.left, global.bounds.top, global.bounds.right, global.bounds.bottom
    );

    // transform検証
    assert_eq!(global.transform.M11, 2.0); // scale x
    assert_eq!(global.transform.M22, 2.0); // scale y
    // translation * scaleの場合、offsetもscaleされる
    // Matrix3x2の演算では: M31 = offset.x * scale.x = 10 * 2 = 20
    assert_eq!(global.transform.M31, 20.0);
    assert_eq!(global.transform.M32, 40.0);

    // bounds検証: local_bounds()がtransformで変換される
    // local_bounds = (0,0,100,50)
    // transform = scale(2,2) + translate(20,40) (scaleされたoffset)
    // 左上 (0,0): x' = 2*0 + 20 = 20, y' = 2*0 + 40 = 40 → (20,40)
    // 右下 (100,50): x' = 2*100 + 20 = 220, y' = 2*50 + 40 = 140 → (220,140)
    assert_eq!(global.bounds.left, 20.0);
    assert_eq!(global.bounds.top, 40.0);
    assert_eq!(global.bounds.right, 220.0);
    assert_eq!(global.bounds.bottom, 140.0);
}

#[test]
fn test_global_arrangement_mul_arrangement() {
    let parent = GlobalArrangement {
        transform: Matrix3x2::translation(10.0, 20.0),
        bounds: Rect {
            left: 10.0,
            top: 20.0,
            right: 50.0,
            bottom: 60.0,
        },
    };

    let child = Arrangement {
        offset: Offset { x: 5.0, y: 7.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 30.0,
            height: 20.0,
        },
    };

    let result = parent * child;

    // transform検証: 親と子の累積
    assert_eq!(result.transform.M31, 15.0); // 10 + 5
    assert_eq!(result.transform.M32, 27.0); // 20 + 7

    // bounds検証: 子のlocal_boundsが結果transformで変換される
    // 子のlocal_bounds: (0, 0, 30, 20) ← 原点基準
    // 結果transform: translate(10,20) * translate(5,7) = translate(15,27)
    // bounds変換: (0,0,30,20) + (15,27) = (15,27,45,47)
    assert_eq!(result.bounds.left, 15.0);
    assert_eq!(result.bounds.top, 27.0);
    assert_eq!(result.bounds.right, 45.0);
    assert_eq!(result.bounds.bottom, 47.0);
}

// ===== W4a ギャップテスト: GlobalArrangement アクセサーと Matrix3x2 変換 =====

/// Arrangement::default() は offset/scale/size すべて既定値
#[test]
fn test_arrangement_default() {
    let arr = Arrangement::default();
    assert_eq!(arr.offset, Offset::default());
    assert_eq!(arr.scale, LayoutScale::default());
    assert_eq!(arr.size, Size::default());
}

/// GlobalArrangement::default() は恒等変換 + ゼロ bounds
#[test]
fn test_global_arrangement_default_identity() {
    let global = GlobalArrangement::default();
    assert_eq!(global.transform, Matrix3x2::identity());
    assert_eq!(global.bounds.left, 0.0);
    assert_eq!(global.bounds.top, 0.0);
    assert_eq!(global.bounds.right, 0.0);
    assert_eq!(global.bounds.bottom, 0.0);
}

/// 変換行列アクセサー: scale_x/scale_y/scale は M11/M22、
/// offset_x/offset_y/offset は M31/M32 を返す
#[test]
fn test_global_arrangement_transform_accessors() {
    let global = GlobalArrangement {
        transform: Matrix3x2 {
            M11: 2.0,
            M12: 0.0,
            M21: 0.0,
            M22: 3.0,
            M31: 7.0,
            M32: 9.0,
        },
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        },
    };

    assert_eq!(global.scale_x(), 2.0);
    assert_eq!(global.scale_y(), 3.0);
    assert_eq!(global.scale(), (2.0, 3.0));
    assert_eq!(global.offset_x(), 7.0);
    assert_eq!(global.offset_y(), 9.0);
    assert_eq!(global.offset(), (7.0, 9.0));
}

/// bounds アクセサー: width/height/size は right-left / bottom-top を返す
#[test]
fn test_global_arrangement_bounds_accessors() {
    let global = GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: 10.0,
            top: 20.0,
            right: 110.0,
            bottom: 220.0,
        },
    };

    assert_eq!(global.width(), 100.0);
    assert_eq!(global.height(), 200.0);
    assert_eq!(global.size(), (100.0, 200.0));
}

/// From<Offset> for Matrix3x2: 平行移動行列（M31/M32 のみ）が生成される
#[test]
fn test_offset_to_matrix3x2_translation() {
    let m: Matrix3x2 = Offset { x: 15.0, y: -25.0 }.into();
    assert_eq!(m.M11, 1.0);
    assert_eq!(m.M12, 0.0);
    assert_eq!(m.M21, 0.0);
    assert_eq!(m.M22, 1.0);
    assert_eq!(m.M31, 15.0);
    assert_eq!(m.M32, -25.0);
}

/// From<LayoutScale> for Matrix3x2: スケール行列（M11/M22 のみ）が生成される
#[test]
fn test_layout_scale_to_matrix3x2_scale() {
    let m: Matrix3x2 = LayoutScale { x: 1.5, y: 0.5 }.into();
    assert_eq!(m.M11, 1.5);
    assert_eq!(m.M12, 0.0);
    assert_eq!(m.M21, 0.0);
    assert_eq!(m.M22, 0.5);
    assert_eq!(m.M31, 0.0);
    assert_eq!(m.M32, 0.0);
}

/// From<Arrangement> for Matrix3x2: translation * scale の合成順序により
/// 平行移動成分にもスケールが適用される（M31 = offset.x * scale.x）
#[test]
fn test_arrangement_to_matrix3x2_translation_then_scale() {
    let arr = Arrangement {
        offset: Offset { x: 10.0, y: 20.0 },
        scale: LayoutScale { x: 2.0, y: 3.0 },
        size: Size {
            width: 100.0,
            height: 50.0,
        },
    };
    let m: Matrix3x2 = arr.into();
    assert_eq!(m.M11, 2.0);
    assert_eq!(m.M22, 3.0);
    assert_eq!(m.M31, 20.0); // 10 * 2
    assert_eq!(m.M32, 60.0); // 20 * 3
}
