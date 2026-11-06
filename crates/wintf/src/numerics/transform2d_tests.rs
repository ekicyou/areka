//! `Transform2D`: 合成の順と符号（純粋・d2d1.dll を呼ばない）

use super::*;

/// 行ベクトル規約（`p' = p * M`）で点を写す。
fn apply(m: &Matrix3x2, x: f32, y: f32) -> (f32, f32) {
    (x * m.M11 + y * m.M21 + m.M31, x * m.M12 + y * m.M22 + m.M32)
}

fn assert_pt(m: &Matrix3x2, from: (f32, f32), want: (f32, f32)) {
    let got = apply(m, from.0, from.1);
    assert!(
        (got.0 - want.0).abs() < 1e-4 && (got.1 - want.1).abs() < 1e-4,
        "{from:?} -> {got:?} (期待 {want:?})"
    );
}

fn v(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
}

#[test]
fn rotation_sign_is_positive_x_to_positive_y() {
    let t = Transform2D {
        rotation: core::f32::consts::FRAC_PI_2,
        ..Transform2D::identity()
    };
    // 正の回転は +X 軸を +Y 軸へ回す（符号を裏返すと (0,-1) になり赤）
    assert_pt(&t.to_affine(), (1.0, 0.0), (0.0, 1.0));
    assert_pt(&t.to_affine(), (0.0, 1.0), (-1.0, 0.0));
}

#[test]
fn rotation_and_scale_turn_around_the_anchor() {
    let t = Transform2D {
        anchor: v(10.0, 10.0),
        rotation: core::f32::consts::FRAC_PI_2,
        ..Transform2D::identity()
    };
    let m = t.to_affine();
    assert_pt(&m, (10.0, 10.0), (10.0, 10.0)); // 支点は動かない
    assert_pt(&m, (11.0, 10.0), (10.0, 11.0));

    let s = Transform2D {
        anchor: v(10.0, 10.0),
        scale: v(2.0, 3.0),
        ..Transform2D::identity()
    };
    assert_pt(&s.to_affine(), (10.0, 10.0), (10.0, 10.0));
    assert_pt(&s.to_affine(), (11.0, 11.0), (12.0, 13.0));
}

#[test]
fn composition_order_is_pivot_then_mat_then_offset() {
    let t = Transform2D {
        rotation: core::f32::consts::FRAC_PI_2,
        mat: Matrix3x2::translation(10.0, 0.0),
        offset: v(0.0, 5.0),
        ..Transform2D::identity()
    };
    // (1,0) -> 回転 (0,1) -> mat (10,1) -> offset (10,6)
    assert_pt(&t.to_affine(), (1.0, 0.0), (10.0, 6.0));
}

#[test]
fn offset_is_outermost_and_not_rotated_or_scaled() {
    let t = Transform2D {
        scale: v(2.0, 2.0),
        rotation: core::f32::consts::FRAC_PI_2,
        offset: v(100.0, 200.0),
        ..Transform2D::identity()
    };
    assert_pt(&t.to_affine(), (0.0, 0.0), (100.0, 200.0));
    assert_pt(&t.to_affine(), (1.0, 0.0), (100.0, 202.0));
}

#[test]
fn negative_scale_flips_around_the_anchor() {
    let t = Transform2D {
        anchor: v(50.0, 20.0),
        scale: v(-1.0, 1.0),
        ..Transform2D::identity()
    };
    let m = t.to_affine();
    assert_pt(&m, (60.0, 30.0), (40.0, 30.0)); // 左右反転・支点の線は固定
    assert_pt(&m, (50.0, 30.0), (50.0, 30.0));
    let f = Transform2D {
        scale: v(1.0, -1.0),
        ..Transform2D::identity()
    };
    assert_pt(&f.to_affine(), (3.0, 4.0), (3.0, -4.0));
}

#[test]
fn identity_is_axis_parallel_fast_path() {
    let m = Transform2D::identity().to_affine();
    assert_eq!((m.M12, m.M21), (0.0, 0.0));
}
