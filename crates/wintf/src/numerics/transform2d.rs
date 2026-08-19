use windows_numerics::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub offset: Vector2,
    pub scale: Vector2,
    pub rotation: f32,
    pub anchor: Vector2,
}

impl Transform2D {
    pub const IDENTITY: Transform2D = Transform2D {
        offset: Vector2 { X: 0.0, Y: 0.0 },
        scale: Vector2 { X: 1.0, Y: 1.0 },
        rotation: 0.0,
        anchor: Vector2 { X: 0.0, Y: 0.0 },
    };

    #[inline]
    pub fn to_affine(&self) -> Matrix3x2 {
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
            M31: ax + self.offset.X - (ax * m11 + ay * m21),
            M32: ay + self.offset.Y - (ax * m12 + ay * m22),
        }
    }
}

impl Default for Transform2D {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
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
