use std::ops::{Add, Neg, Sub};

use super::TimeSpan;

/// 符号つきの差。負は「前」。1 は 100ns。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct TimeDelta(i64);

impl TimeDelta {
    pub const ZERO: Self = Self(0);
    pub const MIN: Self = Self(i64::MIN);
    pub const MAX: Self = Self(i64::MAX);

    /// 100ns の数から作る。
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks)
    }

    /// 100ns の数。
    pub const fn as_ticks(self) -> i64 {
        self.0
    }

    /// 負か。
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// 0 以上なら長さ。負なら `None`。
    pub fn to_span(self) -> Option<TimeSpan> {
        TimeSpan::try_from_ticks(self.0).ok()
    }

    /// 差を足す。型の範囲を越えれば `None`。
    pub fn checked_add(self, other: TimeDelta) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }
}

impl From<TimeSpan> for TimeDelta {
    fn from(span: TimeSpan) -> Self {
        Self(span.as_ticks())
    }
}

impl Add for TimeDelta {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for TimeDelta {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for TimeDelta {
    type Output = Self;
    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod tests;
