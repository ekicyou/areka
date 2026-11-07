//! `Matrix3x2Ext`: 軸に沿うかの判定（5.5）と動きの判定（5.11）

use super::*;

/// f32 の `sin_cos` で組む回転（d2d1.dll を呼ばない・90 度の倍数で浮動小数の残りが出る）。
fn rot(deg: f32) -> Matrix3x2 {
    let (s, c) = deg.to_radians().sin_cos();
    Matrix3x2 {
        M11: c,
        M12: s,
        M21: -s,
        M22: c,
        M31: 0.0,
        M32: 0.0,
    }
}

fn translation(x: f32, y: f32) -> Matrix3x2 {
    Matrix3x2::translation(x, y)
}

#[test]
fn quarter_rotations_are_axis_aligned() {
    for deg in [0.0, 90.0, 180.0, 270.0] {
        assert!(rot(deg).axis_aligned(), "{deg} 度は軸に沿う");
    }
}

#[test]
fn float_residue_within_tolerance_is_axis_aligned() {
    let mut m = Matrix3x2::identity();
    m.M12 = 1e-5;
    m.M21 = -1e-5;
    assert!(m.axis_aligned(), "1e-5 の残りは沿う");
    let mut q = rot(90.0);
    q.M11 = 1e-5;
    q.M22 = -1e-5;
    assert!(q.axis_aligned(), "90 度に 1e-5 の残りも沿う");
}

#[test]
fn diagonal_rotation_and_shear_are_not_axis_aligned() {
    assert!(!rot(45.0).axis_aligned(), "45 度は沿わない");
    let mut shear = Matrix3x2::identity();
    shear.M21 = 0.5;
    assert!(!shear.axis_aligned(), "せん断は沿わない");
}

#[test]
fn moved_compares_in_rect_units() {
    let rect = Aabb::new(0.0, 0.0, 100.0, 50.0);
    let base = Matrix3x2::identity();
    assert!(
        !base.moved(translation(0.5, 0.0), rect, 1.0),
        "0.5 は動かない"
    );
    assert!(base.moved(translation(2.0, 0.0), rect, 1.0), "2.0 は動いた");
}

#[test]
fn moved_uses_the_given_rect_corners() {
    // 拡大 1.01 は単位矩形の角では 0.01 しか動かないが、1000 の矩形の角では 10 動く。
    let rect = Aabb::new(0.0, 0.0, 1000.0, 1000.0);
    let base = Matrix3x2::identity();
    let scaled = Matrix3x2::scale(1.01, 1.01);
    assert!(base.moved(scaled, rect, 1.0), "与えた矩形の角で比べる");
    assert!(!base.moved(base, rect, 0.0), "同じ行列は動かない");
}
