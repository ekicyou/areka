use std::ops::{Add, Sub};

use super::{TimeDelta, TimeSpan};

/// 時刻（ある瞬間）。1 は 100ns。`QueryUnbiasedInterruptTimePrecise` の値と同じ物差し。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct TimePoint(i64);

impl TimePoint {
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

    /// 長さを足す。型の範囲を越えれば `None`。
    pub fn checked_add(self, span: TimeSpan) -> Option<Self> {
        self.0.checked_add(span.as_ticks()).map(Self)
    }

    /// 符号つきの差を足す。型の範囲を越えれば `None`。
    pub fn checked_add_delta(self, delta: TimeDelta) -> Option<Self> {
        self.0.checked_add(delta.as_ticks()).map(Self)
    }

    /// self が earlier 以後なら、その長さ。前なら `None`。
    pub fn span_since(self, earlier: TimePoint) -> Option<TimeSpan> {
        (self >= earlier).then(|| TimeSpan::from_ticks_unchecked(self.0.saturating_sub(earlier.0)))
    }
}

impl Add<TimeSpan> for TimePoint {
    type Output = Self;
    fn add(self, rhs: TimeSpan) -> Self {
        Self(self.0.saturating_add(rhs.as_ticks()))
    }
}

impl Sub<TimeSpan> for TimePoint {
    type Output = Self;
    fn sub(self, rhs: TimeSpan) -> Self {
        Self(self.0.saturating_sub(rhs.as_ticks()))
    }
}

impl Add<TimeDelta> for TimePoint {
    type Output = Self;
    fn add(self, rhs: TimeDelta) -> Self {
        Self(self.0.saturating_add(rhs.as_ticks()))
    }
}

impl Sub<TimeDelta> for TimePoint {
    type Output = Self;
    fn sub(self, rhs: TimeDelta) -> Self {
        Self(self.0.saturating_sub(rhs.as_ticks()))
    }
}

impl Sub for TimePoint {
    type Output = TimeDelta;
    fn sub(self, rhs: Self) -> TimeDelta {
        TimeDelta::from_ticks(self.0.saturating_sub(rhs.0))
    }
}

#[cfg(test)]
#[path = "point_tests.rs"]
mod tests;
