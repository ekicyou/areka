//! 倍率の写しと、比の掛け算・進み具合
//!
//! 丸めの向きは 2 つある。倍率の写しは負の無限大の側へ切り捨てる（届く前に「届いた」にしない）。
//! 長さを作る比の掛け算は、最も近い 100ns へ丸め、ちょうど中間は 0 から遠い側へ寄せる。

use super::{TimeDelta, TimeSpan};

/// 再生の速さの倍率（有限で正）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeScale(f64);

impl TimeScale {
    pub const ONE: Self = Self(1.0);

    /// 有限で正でなければ `None`。
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    /// 倍率の実数。
    pub fn get(self) -> f64 {
        self.0
    }

    /// 経った長さから台本の時刻を求める。まだ届いていない側（負の無限大の側）へ切り捨て、端へ張り付ける。
    pub fn forward(self, elapsed: TimeDelta) -> TimeDelta {
        if self.0 == 1.0 {
            // 2^53 を越える値は f64 を通すと変わるので、倍率 1 は通さない。
            return elapsed;
        }
        // `as` は端へ張り付ける。
        TimeDelta::from_ticks((elapsed.as_ticks() as f64 * self.0).floor() as i64)
    }

    /// 台本の時刻 `script` に届く、最初の経った長さ。どの長さでも届かなければ `None`。
    /// `script` が 0 以上なら、返る値も 0 以上。
    pub fn first_reaching(self, script: TimeDelta) -> Option<TimeDelta> {
        let reaches = |e: i128| self.forward(TimeDelta::from_ticks(e as i64)) >= script;
        let (mut lo, mut hi) = (i64::MIN as i128, i64::MAX as i128);
        if !reaches(hi) {
            return None;
        }
        // `forward` は減らないので、届く最小の値を二分で挟む（多くて 64 回）。
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if reaches(mid) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        Some(TimeDelta::from_ticks(hi as i64))
    }
}

impl TimeSpan {
    /// self × num ÷ den（最も近い 100ns）。途中は 128 ビットで、端数を積もらせない。den は 0 でない。
    pub fn mul_div(self, num: u64, den: u64) -> TimeSpan {
        let product = self.as_ticks() as u128 * num as u128;
        let den = den as u128;
        let rounded = product / den + u128::from(product % den * 2 >= den);
        TimeSpan::from_ticks_unchecked(i64::try_from(rounded).unwrap_or(i64::MAX))
    }

    /// self × ratio（ratio は 0 以上 1 以下へ収めて掛ける。NaN は 0）。
    pub fn mul_unit(self, ratio: f64) -> TimeSpan {
        if ratio.is_nan() || ratio <= 0.0 {
            return TimeSpan::ZERO;
        }
        if ratio >= 1.0 {
            return self;
        }
        let ticks = ((self.as_ticks() as f64 * ratio).round() as i64).min(self.as_ticks());
        TimeSpan::from_ticks_unchecked(ticks)
    }
}

impl TimeDelta {
    /// self ÷ whole を実数の比で返す（進み具合）。whole が 0 なら `None`。
    pub fn ratio_of(self, whole: TimeDelta) -> Option<f64> {
        (whole != TimeDelta::ZERO).then(|| self.as_ticks() as f64 / whole.as_ticks() as f64)
    }
}

#[cfg(test)]
#[path = "scale_tests.rs"]
mod tests;
