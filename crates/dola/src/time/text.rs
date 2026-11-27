//! 時刻・長さ・符号つきの差の文字の形（秒の小数・小数点の下 7 桁）と、その読み戻し

use std::fmt;
use std::str::FromStr;

use super::{TimeDelta, TimePoint, TimeRangeError, TimeSpan};

const TICKS_PER_SEC: i128 = 10_000_000;

/// 秒の小数の文字から時刻の型へ読めなかった理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TimeParseError {
    /// `[-]整数部[.小数部（1〜7 桁）]` の形ではない。
    #[error("time text is malformed")]
    Malformed,
    /// 型の範囲の外（長さでの負を含む）。
    #[error("time text is out of range")]
    OutOfRange,
}

fn write_ticks(f: &mut fmt::Formatter<'_>, ticks: i64) -> fmt::Result {
    let t = i128::from(ticks);
    let sign = if t < 0 { "-" } else { "" };
    let a = t.abs();
    write!(f, "{sign}{}.{:07}", a / TICKS_PER_SEC, a % TICKS_PER_SEC)
}

fn parse_ticks(s: &str) -> Result<i64, TimeParseError> {
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int, frac) = body.split_once('.').unwrap_or((body, "0"));
    let digits = |x: &str| x.bytes().all(|b| b.is_ascii_digit());
    if int.is_empty() || !digits(int) || frac.is_empty() || !digits(frac) || frac.len() > 7 {
        return Err(TimeParseError::Malformed);
    }
    let int: i128 = int.parse().map_err(|_| TimeParseError::OutOfRange)?;
    let frac_ticks: i128 = format!("{frac:0<7}")
        .parse()
        .map_err(|_| TimeParseError::Malformed)?;
    let mag = int
        .checked_mul(TICKS_PER_SEC)
        .and_then(|v| v.checked_add(frac_ticks))
        .ok_or(TimeParseError::OutOfRange)?;
    let signed = if neg { -mag } else { mag };
    i64::try_from(signed).map_err(|_| TimeParseError::OutOfRange)
}

macro_rules! impl_text {
    ($ty:ty, $make:expr) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_ticks(f, self.as_ticks())
            }
        }
        impl FromStr for $ty {
            type Err = TimeParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $make(parse_ticks(s)?)
            }
        }
    };
}

impl_text!(TimePoint, |t| Ok(TimePoint::from_ticks(t)));
impl_text!(TimeDelta, |t| Ok(TimeDelta::from_ticks(t)));
impl_text!(TimeSpan, |t| TimeSpan::try_from_ticks(t).map_err(
    |e: TimeRangeError| {
        debug_assert_eq!(e, TimeRangeError::Negative);
        TimeParseError::OutOfRange
    }
));

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
