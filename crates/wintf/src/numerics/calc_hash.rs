use core::hash::*;
use windows_numerics::*;

pub(crate) trait CalcHash {
    fn calc_hash<H: Hasher>(&self, state: &mut H);
}

#[inline]
fn f32_norm_bits(f: f32) -> u32 {
    if f == 0.0 { 0 } else { f.to_bits() }
}

impl CalcHash for f32 {
    #[inline]
    fn calc_hash<H: Hasher>(&self, state: &mut H) {
        f32_norm_bits(*self).hash(state);
    }
}

impl CalcHash for Vector2 {
    #[inline]
    fn calc_hash<H: Hasher>(&self, state: &mut H) {
        self.X.calc_hash(state);
        self.Y.calc_hash(state);
    }
}

impl CalcHash for Matrix3x2 {
    #[inline]
    fn calc_hash<H: Hasher>(&self, state: &mut H) {
        self.M11.calc_hash(state);
        self.M12.calc_hash(state);
        self.M21.calc_hash(state);
        self.M22.calc_hash(state);
        self.M31.calc_hash(state);
        self.M32.calc_hash(state);
    }
}
