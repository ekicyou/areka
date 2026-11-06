// Task 7.1: 階層的バウンディングボックス計算の統合テスト

use wintf::ecs::{Arrangement, GlobalArrangement, LayoutScale, Offset, Size};

/// 3階層のWidgetツリーでの階層的bounds計算をテスト
/// GlobalArrangementのtrait実装を通じて正しく計算されることを検証
#[test]
fn test_hierarchical_bounds_calculation() {
    // 親ウィジェット（レベル0）
    // Position: (10, 20), Scale: (2.0, 2.0), Size: (100, 50)
    let parent_arr = Arrangement {
        offset: Offset { x: 10.0, y: 20.0 },
        scale: LayoutScale { x: 2.0, y: 2.0 },
        size: Size {
            width: 100.0,
            height: 50.0,
        },
    };
    let parent_global: GlobalArrangement = parent_arr.into();

    // 子ウィジェット（レベル1）
    // Position: (5, 10), Scale: (1.0, 1.0), Size: (30, 20)
    let child_arr = Arrangement {
        offset: Offset { x: 5.0, y: 10.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 30.0,
            height: 20.0,
        },
    };
    let child_global = parent_global * child_arr;

    // 子のbounds検証（親の変換適用後）
    // 子のoffsetに親のスケールを適用。親の無い根のずらしは自分の倍率で伸ばさない
    // parent.bounds.left = 10・top = 20（offset のまま）
    // child.offset = (5, 10), parent.scale = (2, 2)
    // scaled_child_offset = (5*2, 10*2) = (10, 20)
    // child.bounds.left = 10 + 10 = 20・top = 20 + 20 = 40
    // child.bounds.right = 20 + 30*2 = 80 (size 30 * parent_scale 2)・bottom = 40 + 20*2 = 80
    assert_eq!(parent_global.bounds.left, 10.0);
    assert_eq!(parent_global.bounds.top, 20.0);
    assert_eq!(child_global.bounds.left, 20.0);
    assert_eq!(child_global.bounds.top, 40.0);
    assert_eq!(child_global.bounds.right, 80.0);
    assert_eq!(child_global.bounds.bottom, 80.0);

    // 孫ウィジェット（レベル2）
    // Position: (2, 3), Scale: (1.5, 1.5), Size: (10, 8)
    let grandchild_arr = Arrangement {
        offset: Offset { x: 2.0, y: 3.0 },
        scale: LayoutScale { x: 1.5, y: 1.5 },
        size: Size {
            width: 10.0,
            height: 8.0,
        },
    };
    let grandchild_global = child_global * grandchild_arr;

    // 孫のbounds検証（親と子の累積変換適用後）
    // 新ロジック: 孫のoffsetに親（child）のスケールを適用
    // child.bounds.left = 20・top = 40
    // child.transform.M11 = 2 (累積スケール)
    // grandchild.offset = (2, 3)
    // scaled_offset = (2*2, 3*2) = (4, 6)
    // grandchild.bounds.left = 20 + 4 = 24・top = 40 + 6 = 46
    // result.M11 = 2 * 1.5 = 3
    // grandchild.bounds.right = 24 + 10*3 = 54・bottom = 46 + 8*3 = 70
    assert_eq!(grandchild_global.bounds.left, 24.0);
    assert_eq!(grandchild_global.bounds.top, 46.0);
    assert_eq!(grandchild_global.bounds.right, 54.0);
    assert_eq!(grandchild_global.bounds.bottom, 70.0);
}

/// スケール変換のみのテスト
#[test]
fn test_scale_only_transformation() {
    let parent = Arrangement {
        offset: Offset { x: 0.0, y: 0.0 },
        scale: LayoutScale { x: 3.0, y: 2.0 },
        size: Size {
            width: 10.0,
            height: 10.0,
        },
    };
    let parent_global: GlobalArrangement = parent.into();

    let child = Arrangement {
        offset: Offset { x: 5.0, y: 10.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 10.0,
            height: 10.0,
        },
    };

    let child_global = parent_global * child;

    // 子のローカル: offset(5, 10), size(10, 10)
    // 親のscale(3.0, 2.0), offset(0, 0)
    // 子のoffsetに親のスケールを適用: (5*3, 10*2) = (15, 20)
    // child.local_bounds(): (0, 0, 10, 10) - 原点基準
    // result_transform: scale(3,2), translation(15, 20)
    // transformed: (15, 20, 45, 40)
    assert_eq!(child_global.bounds.left, 15.0);
    assert_eq!(child_global.bounds.top, 20.0);
    assert_eq!(child_global.bounds.right, 45.0);
    assert_eq!(child_global.bounds.bottom, 40.0);
}

/// 複雑なスケール階層のテスト
#[test]
fn test_complex_scale_hierarchy() {
    // ルート: scale(2.0, 2.0)
    let root = Arrangement {
        offset: Offset { x: 0.0, y: 0.0 },
        scale: LayoutScale { x: 2.0, y: 2.0 },
        size: Size {
            width: 100.0,
            height: 100.0,
        },
    };
    let root_global: GlobalArrangement = root.into();

    // レベル1: offset(10, 10), scale(1.5, 1.5)
    let level1 = Arrangement {
        offset: Offset { x: 10.0, y: 10.0 },
        scale: LayoutScale { x: 1.5, y: 1.5 },
        size: Size {
            width: 50.0,
            height: 50.0,
        },
    };
    let level1_global = root_global * level1;

    // 新ロジック: offsetに親スケールを適用
    // root: offset=0, scale=2, bounds = (0, 0, 200, 200)
    // level1.offset = (10, 10), parent_scale = 2
    // scaled_offset = (20, 20)
    // level1.bounds.left = 0 + 20 = 20
    // result.M11 = 2 * 1.5 = 3
    // level1.bounds.right = 20 + 50*3 = 170
    assert_eq!(level1_global.bounds.left, 20.0);
    assert_eq!(level1_global.bounds.top, 20.0);
    assert_eq!(level1_global.bounds.right, 170.0);
    assert_eq!(level1_global.bounds.bottom, 170.0);

    // レベル2: offset(5, 5), scale(1.0, 1.0), size(10, 10)
    let level2 = Arrangement {
        offset: Offset { x: 5.0, y: 5.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 10.0,
            height: 10.0,
        },
    };
    let level2_global = level1_global * level2;

    // level2.offset = (5, 5), parent_scale = 3 (累積)
    // scaled_offset = (15, 15)
    // level2.bounds.left = 20 + 15 = 35
    // result.M11 = 3 * 1 = 3
    // level2.bounds.right = 35 + 10*3 = 65
    assert_eq!(level2_global.bounds.left, 35.0);
    assert_eq!(level2_global.bounds.top, 35.0);
    assert_eq!(level2_global.bounds.right, 65.0);
    assert_eq!(level2_global.bounds.bottom, 65.0);
}

/// LayoutRoot → Window のDPIスケール適用テスト (Task 6 検証)
///
/// LayoutRoot は物理ピクセル座標系（スケール 1.0）
/// Window は DIP 座標で offset を指定し、自身の scale で DPI スケールを表現
/// Window の内部に入って初めて DPI スケールが適用される
#[test]
fn test_layout_root_to_window_dpi_scale() {
    // LayoutRoot: 仮想デスクトップ原点、スケール 1.0（物理ピクセル座標系）
    let layout_root = Arrangement {
        offset: Offset { x: 0.0, y: 0.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 9999.0,
            height: 9999.0,
        },
    };
    let layout_root_global: GlobalArrangement = layout_root.into();

    // LayoutRoot の bounds は原点から
    assert_eq!(layout_root_global.bounds.left, 0.0);
    assert_eq!(layout_root_global.bounds.top, 0.0);

    // Window: offset=100 DIP, scale=1.25 (DPI 125%)
    // Window の位置は DIP で指定されるが、LayoutRoot の scale=1.0 なので
    // bounds.left = 100 * 1.0 = 100 となる
    let window = Arrangement {
        offset: Offset { x: 100.0, y: 100.0 },
        scale: LayoutScale { x: 1.25, y: 1.25 },
        size: Size {
            width: 800.0,
            height: 600.0,
        },
    };
    let window_global = layout_root_global * window;

    // Window の bounds:
    // - parent(LayoutRoot).scale = 1.0
    // - scaled_offset = (100 * 1.0, 100 * 1.0) = (100, 100)
    // - bounds.left = 0 + 100 = 100
    // - result_scale = 1.0 * 1.25 = 1.25
    // - bounds.right = 100 + 800 * 1.25 = 100 + 1000 = 1100
    assert_eq!(window_global.bounds.left, 100.0);
    assert_eq!(window_global.bounds.top, 100.0);
    assert_eq!(window_global.bounds.right, 1100.0);
    assert_eq!(window_global.bounds.bottom, 850.0); // 100 + 600 * 1.25

    // Window 内部の Container: offset=10 DIP
    // Window の scale=1.25 が適用される
    let container = Arrangement {
        offset: Offset { x: 10.0, y: 10.0 },
        scale: LayoutScale { x: 1.0, y: 1.0 },
        size: Size {
            width: 400.0,
            height: 300.0,
        },
    };
    let container_global = window_global * container;

    // Container の bounds:
    // - parent(Window).scale = 1.25 (累積)
    // - scaled_offset = (10 * 1.25, 10 * 1.25) = (12.5, 12.5)
    // - bounds.left = 100 + 12.5 = 112.5
    // - result_scale = 1.25 * 1.0 = 1.25
    // - bounds.right = 112.5 + 400 * 1.25 = 112.5 + 500 = 612.5
    assert_eq!(container_global.bounds.left, 112.5);
    assert_eq!(container_global.bounds.top, 112.5);
    assert_eq!(container_global.bounds.right, 612.5);
    assert_eq!(container_global.bounds.bottom, 487.5); // 112.5 + 300 * 1.25
}
