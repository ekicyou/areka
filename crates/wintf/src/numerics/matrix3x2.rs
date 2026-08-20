use super::point2::*;
use windows_numerics::*;

pub trait Matrix3x2Ext {
    fn axis_aligned(&self) -> bool;
    fn moved(self, dst: Matrix3x2, eps_px: f32) -> bool;
}

impl Matrix3x2Ext for Matrix3x2 {
    #[inline]
    fn axis_aligned(&self) -> bool {
        self.M12 == 0.0 && self.M21 == 0.0
    }

    #[inline]
    fn moved(self, dst: Matrix3x2, eps_px: f32) -> bool {
        const UNIT: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        for (x, y) in UNIT {
            let a = Point2::new(x, y) * self;
            let b = Point2::new(x, y) * dst;
            if (a.x() - b.x()).abs() > eps_px || (a.y() - b.y()).abs() > eps_px {
                return true;
            }
        }
        false
    }
}
