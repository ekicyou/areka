//! storyboard の文書の欄の直列化（秒の実数）と、文書の値の上限
//!
//! `#[serde(with = "crate::time::serde_secs")]` を `TimeSpan`・`TimeDelta` とその `Option` の欄に付ける。
//! `with` を付けた欄は、欄が無い時に既定へ倒れないので、省ける欄（`Option` の欄を含む）には `default` も付ける。
//! 文書の値の上限は、100ns の数の絶対値が 2^51 未満。この範囲なら、秒の実数で書いて
//! 最も近い 100ns へ丸め直すと、元の整数に戻る。

use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{TimeDelta, TimeRangeError, TimeSpan};

const TICKS_PER_SEC: f64 = 10_000_000.0;
const DOCUMENT_LIMIT: u64 = 1 << 51;

/// 100ns の数が、文書の値の上限（絶対値 2^51 未満）の中に在るか。
pub(crate) fn within_document_range(ticks: i64) -> bool {
    ticks.unsigned_abs() < DOCUMENT_LIMIT
}

pub(crate) trait Secs: Sized + Copy {
    fn ticks(self) -> i64;
    fn try_from_secs(secs: f64) -> Result<Self, TimeRangeError>;
}

impl Secs for TimeSpan {
    fn ticks(self) -> i64 {
        self.as_ticks()
    }
    fn try_from_secs(secs: f64) -> Result<Self, TimeRangeError> {
        TimeSpan::try_from_secs_f64(secs)
    }
}

impl Secs for TimeDelta {
    fn ticks(self) -> i64 {
        self.as_ticks()
    }
    fn try_from_secs(secs: f64) -> Result<Self, TimeRangeError> {
        TimeDelta::try_from_secs_f64(secs)
    }
}

fn out_of_document_range(ticks: i64) -> String {
    format!("time value {ticks} (100ns) is outside the document range |value| < 2^51")
}

fn to_secs<T: Secs>(value: T) -> Result<f64, String> {
    let ticks = value.ticks();
    if within_document_range(ticks) {
        Ok(ticks as f64 / TICKS_PER_SEC)
    } else {
        Err(out_of_document_range(ticks))
    }
}

fn from_secs<T: Secs>(secs: f64) -> Result<T, String> {
    let value = T::try_from_secs(secs).map_err(|e| e.to_string())?;
    if within_document_range(value.ticks()) {
        Ok(value)
    } else {
        Err(out_of_document_range(value.ticks()))
    }
}

pub(crate) trait SecsField: Sized {
    fn ser<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
    fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>;
}

impl<T: Secs> SecsField for T {
    fn ser<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(to_secs(*self).map_err(S::Error::custom)?)
    }
    fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        from_secs(f64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl<T: Secs> SecsField for Option<T> {
    fn ser<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.map(to_secs)
            .transpose()
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }
    fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<f64>::deserialize(deserializer)?
            .map(from_secs)
            .transpose()
            .map_err(D::Error::custom)
    }
}

pub(crate) fn serialize<T: SecsField, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value.ser(serializer)
}

pub(crate) fn deserialize<'de, T: SecsField, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    T::de(deserializer)
}

#[cfg(test)]
#[path = "serde_secs_tests.rs"]
mod tests;
