//! 作者が書く単位（ミリ秒・秒・`std::time::Duration`）からの変え方
//!
//! 丸めの決まり: 最も近い 100ns へ丸め、ちょうど中間は 0 から遠い側へ寄せる。
//! 有限でない値と、型で表せない値（長さでの負・範囲の外）は `Err` を返す。
//! 秒の実数は、10^7 倍した f64 の積をこの決まりで整数へ丸める。

use std::time::Duration;

use super::{TimeDelta, TimeSpan};

const TICKS_PER_MILLI: i64 = 10_000;
const TICKS_PER_SEC: f64 = 10_000_000.0;
/// 2^63。i64 の範囲は [-2^63, 2^63) なので、上の端はこの値を含まない。
const I64_END: f64 = 9_223_372_036_854_775_808.0;

/// 作者が書く単位から時刻の型へ変えられなかった理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TimeRangeError {
    /// NaN か無限大。
    #[error("time value is not finite")]
    NotFinite,
    /// 長さに負の値。
    #[error("time span is negative")]
    Negative,
    /// 型の範囲の外。
    #[error("time value is out of range")]
    OutOfRange,
}

fn round_secs(secs: f64) -> Result<i64, TimeRangeError> {
    if !secs.is_finite() {
        return Err(TimeRangeError::NotFinite);
    }
    let ticks = (secs * TICKS_PER_SEC).round();
    if (-I64_END..I64_END).contains(&ticks) {
        Ok(ticks as i64)
    } else {
        Err(TimeRangeError::OutOfRange)
    }
}

impl TimeSpan {
    /// ミリ秒から作る。1 ミリ秒＝10,000。u32 の全部の値がちょうど収まる。
    pub const fn from_millis(ms: u32) -> TimeSpan {
        TimeSpan::from_ticks_unchecked(ms as i64 * TICKS_PER_MILLI)
    }

    /// ミリ秒から作る。範囲の外なら `Err`。
    pub fn try_from_millis(ms: u64) -> Result<TimeSpan, TimeRangeError> {
        i64::try_from(ms)
            .ok()
            .and_then(|ms| ms.checked_mul(TICKS_PER_MILLI))
            .map(TimeSpan::from_ticks_unchecked)
            .ok_or(TimeRangeError::OutOfRange)
    }

    /// 秒の実数から作る。
    pub fn try_from_secs_f64(secs: f64) -> Result<TimeSpan, TimeRangeError> {
        if secs.is_finite() && secs < 0.0 {
            return Err(TimeRangeError::Negative);
        }
        round_secs(secs).map(TimeSpan::from_ticks_unchecked)
    }
}

impl TryFrom<Duration> for TimeSpan {
    type Error = TimeRangeError;

    /// 100ns より細かい端数は丸める。失敗は範囲の外だけ。
    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        i64::try_from((value.as_nanos() + 50) / 100)
            .map(TimeSpan::from_ticks_unchecked)
            .map_err(|_| TimeRangeError::OutOfRange)
    }
}

impl TimeDelta {
    /// 符号つきのミリ秒から作る。端を越える値は端へ張り付ける。
    pub const fn from_millis_saturating(ms: i64) -> TimeDelta {
        TimeDelta::from_ticks(ms.saturating_mul(TICKS_PER_MILLI))
    }

    /// 秒の実数から作る。
    pub fn try_from_secs_f64(secs: f64) -> Result<TimeDelta, TimeRangeError> {
        round_secs(secs).map(TimeDelta::from_ticks)
    }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
