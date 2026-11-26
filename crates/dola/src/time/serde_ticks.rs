//! cue の層の欄の直列化（100ns の整数）
//!
//! `#[serde(with = "crate::time::serde_ticks")]` を `TimePoint`・`TimeSpan`・`TimeDelta` と
//! その `Option` の欄に付ける。`Option` の欄は `default` も付ける（欄が無い時を「無い」にする）。

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{TimeDelta, TimePoint, TimeRangeError, TimeSpan};

pub(crate) trait Ticks: Sized + Copy {
    fn ticks(self) -> i64;
    fn from_ticks(ticks: i64) -> Result<Self, TimeRangeError>;
}

impl Ticks for TimePoint {
    fn ticks(self) -> i64 {
        self.as_ticks()
    }
    fn from_ticks(ticks: i64) -> Result<Self, TimeRangeError> {
        Ok(TimePoint::from_ticks(ticks))
    }
}

impl Ticks for TimeSpan {
    fn ticks(self) -> i64 {
        self.as_ticks()
    }
    fn from_ticks(ticks: i64) -> Result<Self, TimeRangeError> {
        TimeSpan::try_from_ticks(ticks)
    }
}

impl Ticks for TimeDelta {
    fn ticks(self) -> i64 {
        self.as_ticks()
    }
    fn from_ticks(ticks: i64) -> Result<Self, TimeRangeError> {
        Ok(TimeDelta::from_ticks(ticks))
    }
}

pub(crate) trait TicksField: Sized {
    fn ser<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
    fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>;
}

impl<T: Ticks> TicksField for T {
    fn ser<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.ticks())
    }
    fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::from_ticks(i64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl<T: Ticks> TicksField for Option<T> {
    fn ser<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.map(Ticks::ticks).serialize(serializer)
    }
    fn de<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<i64>::deserialize(deserializer)?
            .map(T::from_ticks)
            .transpose()
            .map_err(D::Error::custom)
    }
}

pub(crate) fn serialize<T: TicksField, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value.ser(serializer)
}

pub(crate) fn deserialize<'de, T: TicksField, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    T::de(deserializer)
}

#[cfg(test)]
#[path = "serde_ticks_tests.rs"]
mod tests;
