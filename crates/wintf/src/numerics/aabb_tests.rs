//! `Aabb`: 空の判定・交わり・和・変形（5.6, 5.7, 6.6）

use super::*;

const NAN: f32 = f32::NAN;

fn r(l: f32, t: f32, rr: f32, b: f32) -> Aabb {
    Aabb::new(l, t, rr, b)
}

fn assert_rect(a: Aabb, e: Aabb) {
    for (x, y) in [
        (a.left(), e.left()),
        (a.top(), e.top()),
        (a.right(), e.right()),
        (a.bottom(), e.bottom()),
    ] {
        assert!((x - y).abs() < 1e-4, "{a:?} != {e:?}");
    }
}

#[test]
fn nan_degenerate_and_empty_are_empty() {
    assert!(Aabb::EMPTY.is_empty());
    assert!(r(NAN, 0.0, 10.0, 10.0).is_empty());
    assert!(r(0.0, 0.0, 10.0, NAN).is_empty());
    assert!(r(0.0, 0.0, 0.0, 10.0).is_empty(), "幅 0");
    assert!(r(0.0, 5.0, 10.0, 5.0).is_empty(), "高さ 0");
    assert!(r(10.0, 0.0, 0.0, 10.0).is_empty(), "反転");
    assert!(!r(0.0, 0.0, 10.0, 10.0).is_empty());
}

#[test]
fn intersect_disjoint_is_empty() {
    let a = r(0.0, 0.0, 10.0, 10.0);
    assert_eq!(a.intersect(r(20.0, 20.0, 30.0, 30.0)), Aabb::EMPTY);
    assert_eq!(
        a.intersect(r(10.0, 0.0, 20.0, 10.0)),
        Aabb::EMPTY,
        "辺が触れるだけ"
    );
    assert_rect(
        a.intersect(r(5.0, 5.0, 15.0, 15.0)),
        r(5.0, 5.0, 10.0, 10.0),
    );
}

#[test]
fn intersect_with_nan_or_empty_is_empty() {
    let a = r(0.0, 0.0, 10.0, 10.0);
    // f32::max は NaN を捨てるので、素の max/min だと a がそのまま残る
    assert_eq!(a.intersect(r(NAN, NAN, NAN, NAN)), Aabb::EMPTY);
    assert_eq!(r(NAN, 0.0, 5.0, 5.0).intersect(a), Aabb::EMPTY);
    assert_eq!(a.intersect(Aabb::EMPTY), Aabb::EMPTY);
}

#[test]
fn union_has_empty_as_identity() {
    let a = r(0.0, 0.0, 10.0, 10.0);
    assert_eq!(a.union(Aabb::EMPTY), a);
    assert_eq!(Aabb::EMPTY.union(a), a);
    assert_eq!(a.union(r(NAN, 0.0, 1.0, 1.0)), a, "NaN は空");
    assert_eq!(r(NAN, 0.0, 1.0, 1.0).union(a), a, "NaN は空");
    assert!(Aabb::EMPTY.union(Aabb::EMPTY).is_empty());
    assert_rect(a.union(r(20.0, 5.0, 30.0, 15.0)), r(0.0, 0.0, 30.0, 15.0));
}

#[test]
fn transform_axis_aligned_and_flip_orders_corners() {
    let a = r(1.0, 2.0, 3.0, 5.0);
    let m = Matrix3x2::translation(10.0, 20.0);
    assert_rect(a.transform(m), r(11.0, 22.0, 13.0, 25.0));
    // 反転（負の拡大）でも左上右下が揃う
    let flip = Matrix3x2 {
        M11: -1.0,
        M12: 0.0,
        M21: 0.0,
        M22: -2.0,
        M31: 0.0,
        M32: 0.0,
    };
    assert_rect(a.transform(flip), r(-3.0, -10.0, -1.0, -4.0));
}

#[test]
fn transform_rotation_gives_bounding_box() {
    let a = r(0.0, 0.0, 4.0, 2.0);
    // 90 度: (x, y) -> (-y, x)
    let rot90 = Matrix3x2 {
        M11: 0.0,
        M12: 1.0,
        M21: -1.0,
        M22: 0.0,
        M31: 0.0,
        M32: 0.0,
    };
    assert_rect(a.transform(rot90), r(-2.0, 0.0, 0.0, 4.0));
    // 45 度: 4 角の外接矩形
    let (s, c) = 45f32.to_radians().sin_cos();
    let rot45 = Matrix3x2 {
        M11: c,
        M12: s,
        M21: -s,
        M22: c,
        M31: 0.0,
        M32: 0.0,
    };
    let h = 2f32.sqrt();
    assert_rect(
        r(0.0, 0.0, 2.0, 2.0).transform(rot45),
        r(-h, 0.0, h, 2.0 * h),
    );
}

#[test]
fn transform_empty_or_nan_stays_empty() {
    let m = Matrix3x2::translation(10.0, 20.0);
    assert_eq!(Aabb::EMPTY.transform(m), Aabb::EMPTY);
    assert_eq!(r(NAN, 0.0, 1.0, 1.0).transform(m), Aabb::EMPTY);
    assert_eq!(
        r(5.0, 0.0, 1.0, 1.0).transform(m),
        Aabb::EMPTY,
        "反転した矩形"
    );
    let a = r(0.0, 0.0, 1.0, 1.0);
    let mut nan_m = m;
    nan_m.M31 = NAN;
    assert_eq!(a.transform(nan_m), Aabb::EMPTY, "NaN の行列（軸に沿う）");
    let mut nan_skew = nan_m;
    nan_skew.M12 = 0.5;
    assert_eq!(a.transform(nan_skew), Aabb::EMPTY, "NaN の行列（一般）");
}
