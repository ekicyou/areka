use super::point2::*;
use core::ops::*;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows_numerics::*;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Aabb(pub D2D_RECT_F);

impl Aabb {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self(D2D_RECT_F {
            left,
            top,
            right,
            bottom,
        })
    }

    /// union の単位元（反転無限大）。`is_empty()` とも整合。
    pub const EMPTY: Aabb = Aabb::new(
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );

    #[inline]
    pub fn width(&self) -> f32 {
        self.0.right - self.0.left
    }
    #[inline]
    pub fn height(&self) -> f32 {
        self.0.bottom - self.0.top
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        !(self.0.right > self.0.left && self.0.bottom > self.0.top)
    }

    #[inline]
    pub fn contains(&self, p: Vector2) -> bool {
        p.X >= self.0.left && p.X < self.0.right && p.Y >= self.0.top && p.Y < self.0.bottom
    }

    #[inline]
    pub fn intersect(self, o: Aabb) -> Aabb {
        Aabb::new(
            self.0.left.max(o.0.left),
            self.0.top.max(o.0.top),
            self.0.right.min(o.0.right),
            self.0.bottom.min(o.0.bottom),
        )
    }

    #[inline]
    pub fn union(self, o: Aabb) -> Aabb {
        if self.is_empty() {
            return o;
        }
        if o.is_empty() {
            return self;
        }
        Aabb::new(
            self.0.left.min(o.0.left),
            self.0.top.min(o.0.top),
            self.0.right.max(o.0.right),
            self.0.bottom.max(o.0.bottom),
        )
    }

    #[inline]
    pub fn translate(self, v: Vector2) -> Aabb {
        Aabb::new(
            self.0.left + v.X,
            self.0.top + v.Y,
            self.0.right + v.X,
            self.0.bottom + v.Y,
        )
    }

    #[inline]
    pub fn transform(self, m: Matrix3x2) -> Aabb {
        if self.is_empty() {
            return Aabb::EMPTY;
        }
        let r = self.0;

        if m.M12 == 0.0 && m.M21 == 0.0 {
            let a = Point2::new(r.left, r.top) * m;
            let b = Point2::new(r.right, r.bottom) * m;
            return Aabb::new(
                a.x().min(b.x()),
                a.y().min(b.y()),
                a.x().max(b.x()),
                a.y().max(b.y()),
            );
        }

        // --- 一般（回転/シアー）：4隅を Point2 で変換して bbox ---
        let ps = [
            Point2::new(r.left, r.top) * m,
            Point2::new(r.right, r.top) * m,
            Point2::new(r.right, r.bottom) * m,
            Point2::new(r.left, r.bottom) * m,
        ];
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in ps {
            min_x = min_x.min(p.x());
            max_x = max_x.max(p.x());
            min_y = min_y.min(p.y());
            max_y = max_y.max(p.y());
        }
        Aabb::new(min_x, min_y, max_x, max_y)
    }
}

impl From<D2D_RECT_F> for Aabb {
    fn from(r: D2D_RECT_F) -> Self {
        Aabb(r)
    }
}

use core::ops::{Add, BitAnd, BitAndAssign, BitOr, BitOrAssign};

impl BitOr for Aabb {
    type Output = Aabb;
    #[inline]
    fn bitor(self, o: Aabb) -> Aabb {
        self.union(o)
    }
}
impl BitOrAssign for Aabb {
    #[inline]
    fn bitor_assign(&mut self, o: Aabb) {
        *self = *self | o;
    }
}

impl BitAnd for Aabb {
    type Output = Aabb;
    #[inline]
    fn bitand(self, o: Aabb) -> Aabb {
        self.intersect(o)
    }
}
impl BitAndAssign for Aabb {
    #[inline]
    fn bitand_assign(&mut self, o: Aabb) {
        *self = *self & o;
    }
}

impl Add<Vector2> for Aabb {
    type Output = Aabb;
    #[inline]
    fn add(self, v: Vector2) -> Aabb {
        self.translate(v)
    }
}
impl AddAssign<Vector2> for Aabb {
    #[inline]
    fn add_assign(&mut self, v: Vector2) {
        *self = *self + v;
    }
}

impl Sub<Vector2> for Aabb {
    type Output = Aabb;
    #[inline]
    fn sub(self, v: Vector2) -> Aabb {
        self.translate(Vector2 { X: -v.X, Y: -v.Y })
    }
}
impl SubAssign<Vector2> for Aabb {
    #[inline]
    fn sub_assign(&mut self, v: Vector2) {
        *self = *self - v;
    }
}

impl Mul<Matrix3x2> for Aabb {
    type Output = Aabb;
    #[inline]
    fn mul(self, m: Matrix3x2) -> Aabb {
        self.transform(m)
    }
}
impl MulAssign<Matrix3x2> for Aabb {
    #[inline]
    fn mul_assign(&mut self, m: Matrix3x2) {
        *self = *self * m;
    }
}
