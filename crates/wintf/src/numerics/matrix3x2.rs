use windows_numerics::*;

use super::aabb::Aabb;

pub trait Matrix3x2Ext {
    /// 軸に沿う変換か（`PushAxisAlignedClip` を使えるか）。せん断なし、または 90 度の倍数の回転。
    /// 許容 1e-4: f32 の `sin_cos` が 90 度の倍数で残す 1e-7 程度を吸う（最大の DPI でも 1 画素の 1/1000 以下）。
    fn axis_aligned(&self) -> bool;
    /// `rect` の 4 角を `self` と `dst` で写し、どれかの差が `eps` を越えれば真。`eps` の単位は `rect` と同じ。
    fn moved(self, dst: Matrix3x2, rect: Aabb, eps: f32) -> bool;
}

impl Matrix3x2Ext for Matrix3x2 {
    #[inline]
    fn axis_aligned(&self) -> bool {
        const EPS: f32 = 1e-4;
        let no_skew = self.M12.abs() < EPS && self.M21.abs() < EPS;
        let quarter = self.M11.abs() < EPS && self.M22.abs() < EPS;
        no_skew || quarter
    }

    #[inline]
    fn moved(self, dst: Matrix3x2, rect: Aabb, eps: f32) -> bool {
        [
            (rect.left(), rect.top()),
            (rect.right(), rect.top()),
            (rect.right(), rect.bottom()),
            (rect.left(), rect.bottom()),
        ]
        .into_iter()
        .any(|(x, y)| {
            let (ax, ay) = transform_point(x, y, &self);
            let (bx, by) = transform_point(x, y, &dst);
            (ax - bx).abs() > eps || (ay - by).abs() > eps
        })
    }
}

pub(super) fn transform_point(x: f32, y: f32, m: &Matrix3x2) -> (f32, f32) {
    (x * m.M11 + y * m.M21 + m.M31, x * m.M12 + y * m.M22 + m.M32)
}

#[cfg(test)]
#[path = "matrix3x2_tests.rs"]
mod matrix3x2_tests;
