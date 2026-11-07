use super::calc_hash::*;
use super::matrix3x2::{Matrix3x2Ext, transform_point};
use core::hash::*;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows_numerics::*;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Aabb(pub D2D_RECT_F);

impl CalcHash for Aabb {
    #[inline]
    fn calc_hash<H: Hasher>(&self, state: &mut H) {
        let r = &self.0;
        r.left.calc_hash(state);
        r.top.calc_hash(state);
        r.right.calc_hash(state);
        r.bottom.calc_hash(state);
    }
}

impl Hash for Aabb {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.calc_hash(state);
    }
}

impl Aabb {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self(D2D_RECT_F {
            left,
            top,
            right,
            bottom,
        })
    }

    pub const fn new_rect(r: D2D_RECT_F) -> Self {
        Self(r)
    }

    /// union の単位元（反転無限大）。`is_empty()` とも整合。
    pub const EMPTY: Aabb = Aabb::new(
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );

    #[inline]
    pub fn left(&self) -> f32 {
        self.0.left
    }

    #[inline]
    pub fn top(&self) -> f32 {
        self.0.top
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.0.right
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.0.bottom
    }

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

    /// 空（NaN・退化を含む）は `EMPTY` へそろえる。
    #[inline]
    fn or_empty(self) -> Aabb {
        if self.is_empty() { Aabb::EMPTY } else { self }
    }

    /// 交わり。離れた矩形・空・NaN との交わりは `EMPTY`。
    #[inline]
    pub fn intersect(self, o: Aabb) -> Aabb {
        // f32::max/min は NaN を捨てるので、先に空を弾く
        if self.is_empty() || o.is_empty() {
            return Aabb::EMPTY;
        }
        Aabb::new(
            self.0.left.max(o.0.left),
            self.0.top.max(o.0.top),
            self.0.right.min(o.0.right),
            self.0.bottom.min(o.0.bottom),
        )
        .or_empty()
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
    pub fn transform(self, m: Matrix3x2) -> Aabb {
        if self.is_empty() {
            return Aabb::EMPTY;
        }
        let r = self.0;

        if m.axis_aligned() {
            let (ax, ay) = transform_point(r.left, r.top, &m);
            let (bx, by) = transform_point(r.right, r.bottom, &m);
            return Aabb::new(ax.min(bx), ay.min(by), ax.max(bx), ay.max(by)).or_empty();
        }

        // --- 一般（回転/シアー）：4隅を変換して bbox ---
        let ps = [
            transform_point(r.left, r.top, &m),
            transform_point(r.right, r.top, &m),
            transform_point(r.right, r.bottom, &m),
            transform_point(r.left, r.bottom, &m),
        ];
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for (x, y) in ps {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        Aabb::new(min_x, min_y, max_x, max_y).or_empty()
    }
}

impl From<D2D_RECT_F> for Aabb {
    fn from(r: D2D_RECT_F) -> Self {
        Aabb(r)
    }
}

#[cfg(test)]
#[path = "aabb_tests.rs"]
mod aabb_tests;
