use super::*;

// Task 6.3: D2DRectExt拡張トレイトのテスト

#[test]
fn test_d2drect_from_offset_size() {
    let offset = Offset { x: 10.0, y: 20.0 };
    let size = Size {
        width: 100.0,
        height: 50.0,
    };

    let rect = Rect::from_offset_size(offset, size);
    assert_eq!(rect.left, 10.0);
    assert_eq!(rect.top, 20.0);
    assert_eq!(rect.right, 110.0);
    assert_eq!(rect.bottom, 70.0);
}

#[test]
fn test_d2drect_width_height() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    assert_eq!(rect.width(), 100.0);
    assert_eq!(rect.height(), 50.0);
}

#[test]
fn test_d2drect_offset() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    let offset = rect.offset();
    assert_eq!(offset.x, 10.0);
    assert_eq!(offset.y, 20.0);
}

#[test]
fn test_d2drect_size() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    let size = rect.size();
    assert_eq!(size.width, 100.0);
    assert_eq!(size.height, 50.0);
}

#[test]
fn test_d2drect_set_offset() {
    let mut rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    rect.set_offset(PointF { x: 30.0, y: 40.0 });
    assert_eq!(rect.left, 30.0);
    assert_eq!(rect.top, 40.0);
    assert_eq!(rect.right, 130.0); // 幅100を維持
    assert_eq!(rect.bottom, 90.0); // 高さ50を維持
}

#[test]
fn test_d2drect_set_size() {
    let mut rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    rect.set_size(Size { width: 200.0, height: 100.0 });
    assert_eq!(rect.left, 10.0); // 左上を維持
    assert_eq!(rect.top, 20.0);
    assert_eq!(rect.right, 210.0); // 10 + 200
    assert_eq!(rect.bottom, 120.0); // 20 + 100
}

#[test]
fn test_d2drect_contains() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    assert!(rect.contains(10.0, 20.0)); // 左上
    assert!(rect.contains(110.0, 70.0)); // 右下
    assert!(rect.contains(50.0, 40.0)); // 中央
    assert!(!rect.contains(5.0, 20.0)); // 左外
    assert!(!rect.contains(10.0, 15.0)); // 上外
    assert!(!rect.contains(115.0, 40.0)); // 右外
    assert!(!rect.contains(50.0, 75.0)); // 下外
}

#[test]
fn test_d2drect_union() {
    let rect1 = Rect {
        left: 10.0,
        top: 20.0,
        right: 50.0,
        bottom: 60.0,
    };

    let rect2 = Rect {
        left: 30.0,
        top: 40.0,
        right: 70.0,
        bottom: 80.0,
    };

    let union = rect1.union(&rect2);
    assert_eq!(union.left, 10.0); // min(10, 30)
    assert_eq!(union.top, 20.0); // min(20, 40)
    assert_eq!(union.right, 70.0); // max(50, 70)
    assert_eq!(union.bottom, 80.0); // max(60, 80)
}

// W4b-T: 個別エッジセッターのギャップテスト
// set_offset / set_size は既存テストで固定済みだが、4 個の単独エッジセッター
// (set_left/set_top/set_right/set_bottom) は未検証だった。各々が対応する
// フィールドのみを書き換え、他のエッジに副作用がないことを特性化する。

#[test]
fn test_d2drect_set_left() {
    let mut rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };
    rect.set_left(5.0);
    assert_eq!(rect.left, 5.0);
    // 他のエッジは不変
    assert_eq!(rect.top, 20.0);
    assert_eq!(rect.right, 110.0);
    assert_eq!(rect.bottom, 70.0);
}

#[test]
fn test_d2drect_set_top() {
    let mut rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };
    rect.set_top(15.0);
    assert_eq!(rect.top, 15.0);
    assert_eq!(rect.left, 10.0);
    assert_eq!(rect.right, 110.0);
    assert_eq!(rect.bottom, 70.0);
}

#[test]
fn test_d2drect_set_right() {
    let mut rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };
    rect.set_right(200.0);
    assert_eq!(rect.right, 200.0);
    assert_eq!(rect.left, 10.0);
    assert_eq!(rect.top, 20.0);
    assert_eq!(rect.bottom, 70.0);
}

#[test]
fn test_d2drect_set_bottom() {
    let mut rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };
    rect.set_bottom(120.0);
    assert_eq!(rect.bottom, 120.0);
    assert_eq!(rect.left, 10.0);
    assert_eq!(rect.top, 20.0);
    assert_eq!(rect.right, 110.0);
}

/// validate() は正常な矩形（left<=right, top<=bottom）でパニックしない
///
/// 既存テストは left>right / top>bottom の panic ケースのみ固定していた。
/// 等号境界を含む正常系で debug_assert が発火しないことを特性化する。
#[cfg(debug_assertions)]
#[test]
fn test_d2drect_validate_valid_does_not_panic() {
    // 通常の正常矩形
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };
    rect.validate();

    // 退化矩形（left==right, top==bottom）も「不正」ではない（<= 判定）
    let degenerate = Rect {
        left: 50.0,
        top: 50.0,
        right: 50.0,
        bottom: 50.0,
    };
    degenerate.validate();
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Invalid rect: left > right")]
fn test_d2drect_validate_invalid_horizontal() {
    let rect = Rect {
        left: 100.0,
        top: 20.0,
        right: 50.0, // left > right
        bottom: 70.0,
    };

    rect.validate();
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Invalid rect: top > bottom")]
fn test_d2drect_validate_invalid_vertical() {
    let rect = Rect {
        left: 10.0,
        top: 100.0,
        right: 50.0,
        bottom: 70.0, // top > bottom
    };

    rect.validate();
}

// Task 6.2: transform_rect_axis_aligned関数のテスト

#[test]
fn test_transform_rect_identity() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    let matrix = Matrix3x2::identity();
    let result = transform_rect_axis_aligned(&rect, &matrix);

    assert_eq!(result.left, rect.left);
    assert_eq!(result.top, rect.top);
    assert_eq!(result.right, rect.right);
    assert_eq!(result.bottom, rect.bottom);
}

#[test]
fn test_transform_rect_translation_only() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    let matrix = Matrix3x2::translation(5.0, 10.0);
    let result = transform_rect_axis_aligned(&rect, &matrix);

    assert_eq!(result.left, 15.0); // 10 + 5
    assert_eq!(result.top, 30.0); // 20 + 10
    assert_eq!(result.right, 115.0); // 110 + 5
    assert_eq!(result.bottom, 80.0); // 70 + 10
}

#[test]
fn test_transform_rect_scale_only() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    let matrix = Matrix3x2::scale(2.0, 2.0);
    let result = transform_rect_axis_aligned(&rect, &matrix);

    assert_eq!(result.left, 20.0); // 10 * 2
    assert_eq!(result.top, 40.0); // 20 * 2
    assert_eq!(result.right, 220.0); // 110 * 2
    assert_eq!(result.bottom, 140.0); // 70 * 2
}

#[test]
fn test_transform_rect_translation_and_scale() {
    let rect = Rect {
        left: 10.0,
        top: 20.0,
        right: 110.0,
        bottom: 70.0,
    };

    // スケール -> 平行移動の順
    let scale = Matrix3x2::scale(2.0, 2.0);
    let translation = Matrix3x2::translation(5.0, 10.0);
    let matrix = scale * translation;

    let result = transform_rect_axis_aligned(&rect, &matrix);

    // (10, 20) -> scale -> (20, 40) -> translate -> (25, 50)
    // (110, 70) -> scale -> (220, 140) -> translate -> (225, 150)
    assert_eq!(result.left, 25.0);
    assert_eq!(result.top, 50.0);
    assert_eq!(result.right, 225.0);
    assert_eq!(result.bottom, 150.0);
}
