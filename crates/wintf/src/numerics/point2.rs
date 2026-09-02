use super::calc_hash::*;
use core::hash::*;
use core::ops::*;
use windows_numerics::*;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Point2(pub Vector2);

impl CalcHash for Point2 {
    #[inline]
    fn calc_hash<H: Hasher>(&self, state: &mut H) {
        self.0.calc_hash(state);
    }
}

impl Hash for Point2 {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.calc_hash(state);
    }
}

impl Point2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self(Vector2 { X: x, Y: y })
    }
    #[inline]
    pub fn x(&self) -> f32 {
        self.0.X
    }
    #[inline]
    pub fn y(&self) -> f32 {
        self.0.Y
    }

    #[inline]
    pub fn transform(self, m: Matrix3x2) -> Point2 {
        let x = self.0.X;
        let y = self.0.Y;
        Point2::new(x * m.M11 + y * m.M21 + m.M31, x * m.M12 + y * m.M22 + m.M32)
    }
}

impl Deref for Point2 {
    type Target = Vector2;
    #[inline]
    fn deref(&self) -> &Vector2 {
        &self.0
    }
}
impl DerefMut for Point2 {
    #[inline]
    fn deref_mut(&mut self) -> &mut Vector2 {
        &mut self.0
    }
}

impl From<Vector2> for Point2 {
    #[inline]
    fn from(v: Vector2) -> Self {
        Self(v)
    }
}
impl From<Point2> for Vector2 {
    #[inline]
    fn from(p: Point2) -> Self {
        p.0
    }
}

impl Mul<Matrix3x2> for Point2 {
    type Output = Point2;
    #[inline]
    fn mul(self, m: Matrix3x2) -> Point2 {
        self.transform(m)
    }
}

impl MulAssign<Matrix3x2> for Point2 {
    #[inline]
    fn mul_assign(&mut self, m: Matrix3x2) {
        *self = *self * m;
    }
}
