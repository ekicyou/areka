use super::calc_hash::*;
use core::hash::*;
use windows_numerics::*;

/// ローカル変換（WinUI Composition の `Visual` 準拠）。
///
/// # 合成順（local → parent）
/// ```text
/// to_affine = pivot_srt · mat · T(offset)
///             └ 最内 ┘  汎用 └ 最外 ┘
/// ```
/// 1. `pivot_srt` : anchor 基準の Scale・Rotation  `(-anchor)·S·R·(+anchor)`
/// 2. `mat`    : 汎用行列
/// 3. `offset`    : 親空間での平行移動（最外）
///
/// 適用順序のイメージは「点はまず pivot 変形を受け、次に汎用 mat で歪み、
/// 最後に offset で親空間へ据えられる」。`rotation == 0` かつ `mat == identity`
/// のとき `M12 == M21 == 0`（`Aabb::transform` の軸平行 fast path に一致）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    /// Scale / Rotation のピボット（中心点）。
    pub anchor: Vector2,

    /// anchor 基準のスケール（pivot_srt の一部＝**最内**）。
    pub scale: Vector2,

    /// anchor 基準の回転 [rad]（CCW、pivot_srt の一部）。
    pub rotation: f32,

    /// 汎用行列。pivot_srt と offset の**間**に適用。
    /// 既定は `Matrix3x2::identity()`。恒等時は合成をスキップする fast path が効く。
    pub mat: Matrix3x2,

    /// 親空間での平行移動（合成順では**最外**＝ mat より後に適用）。
    pub offset: Vector2,
}

impl CalcHash for Transform2D {
    #[inline]
    fn calc_hash<H: Hasher>(&self, state: &mut H) {
        self.anchor.calc_hash(state);
        self.scale.calc_hash(state);
        self.rotation.calc_hash(state);
        self.mat.calc_hash(state);
        self.offset.calc_hash(state);
    }
}

impl Hash for Transform2D {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.calc_hash(state);
    }
}

impl Transform2D {
    pub const fn identity() -> Transform2D {
        Transform2D {
            offset: Vector2 { X: 0.0, Y: 0.0 },
            scale: Vector2 { X: 1.0, Y: 1.0 },
            rotation: 0.0,
            anchor: Vector2 { X: 0.0, Y: 0.0 },
            mat: Matrix3x2::identity(),
        }
    }

    #[inline]
    pub fn pivot_srt(&self) -> Matrix3x2 {
        let (sin, cos) = self.rotation.sin_cos();
        let (sx, sy) = (self.scale.X, self.scale.Y);
        let (ax, ay) = (self.anchor.X, self.anchor.Y);
        let m11 = sx * cos;
        let m12 = sx * sin;
        let m21 = -sy * sin;
        let m22 = sy * cos;
        Matrix3x2 {
            M11: m11,
            M12: m12,
            M21: m21,
            M22: m22,
            M31: ax - (ax * m11 + ay * m21),
            M32: ay - (ax * m12 + ay * m22),
        }
    }

    #[inline]
    pub fn to_affine(&self) -> Matrix3x2 {
        let srt = self.pivot_srt();
        let a = if self.mat == Matrix3x2::identity() {
            srt
        } else {
            srt * self.mat
        };
        Matrix3x2 {
            M31: a.M31 + self.offset.X,
            M32: a.M32 + self.offset.Y,
            ..a
        }
    }
}

impl Default for Transform2D {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl From<Transform2D> for Matrix3x2 {
    #[inline]
    fn from(t: Transform2D) -> Matrix3x2 {
        t.to_affine()
    }
}
impl From<&Transform2D> for Matrix3x2 {
    #[inline]
    fn from(t: &Transform2D) -> Matrix3x2 {
        t.to_affine()
    }
}
