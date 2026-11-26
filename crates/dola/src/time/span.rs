use std::ops::{Add, Sub};
use std::time::Duration;

use super::{TimeDelta, TimeRangeError};

const TICKS_PER_SEC: i64 = 10_000_000;

/// 長さ（0 以上）。1 は 100ns。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct TimeSpan(i64);

impl TimeSpan {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i64::MAX);

    /// 100ns の数から作る。負なら `Err`（`TimeRangeError::Negative`）。
    pub const fn try_from_ticks(ticks: i64) -> Result<Self, TimeRangeError> {
        if ticks < 0 {
            Err(TimeRangeError::Negative)
        } else {
            Ok(Self(ticks))
        }
    }

    /// 呼び手が 0 以上を保証した値から作る。
    pub(super) const fn from_ticks_unchecked(ticks: i64) -> Self {
        debug_assert!(ticks >= 0);
        Self(ticks)
    }

    /// 100ns の数。
    pub const fn as_ticks(self) -> i64 {
        self.0
    }

    /// 長さを足す。型の範囲を越えれば `None`。
    pub fn checked_add(self, other: TimeSpan) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// 長さを引く。負になれば `None`。
    pub fn checked_sub(self, other: TimeSpan) -> Option<Self> {
        (self >= other).then(|| Self(self.0 - other.0))
    }

    /// `std::time::Duration` へ、ちょうどの値で変える。
    pub fn to_std(self) -> Duration {
        Duration::new(
            (self.0 / TICKS_PER_SEC) as u64,
            ((self.0 % TICKS_PER_SEC) * 100) as u32,
        )
    }
}

impl Add for TimeSpan {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for TimeSpan {
    type Output = TimeDelta;
    fn sub(self, rhs: Self) -> TimeDelta {
        // どちらも 0 以上なので、差は i64 に収まる。
        TimeDelta::from_ticks(self.0 - rhs.0)
    }
}

#[cfg(test)]
#[path = "span_tests.rs"]
mod tests;
