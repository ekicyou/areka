//! 倍率と DPI の読み手のテストが分け合う小道具（`#[test]` を持たない）。

use areka_emo_present::{AppliedScale, ScaleRatio};

pub(crate) fn ratio(num: u32, den: u32) -> ScaleRatio {
    ScaleRatio::new(num, den).unwrap()
}

pub fn applied(num: u32, den: u32, dpi_x: Option<u16>) -> AppliedScale {
    AppliedScale {
        ratio: ratio(num, den),
        dpi_x,
    }
}
