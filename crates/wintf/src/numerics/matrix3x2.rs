use windows_numerics::*;

pub trait Matrix3x2Ext {
    fn axis_aligned(&self) -> bool;
}

impl Matrix3x2Ext for Matrix3x2 {
    #[inline]
    fn axis_aligned(&self) -> bool {
        self.M12 == 0.0 && self.M21 == 0.0
    }
}
