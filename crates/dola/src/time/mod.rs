//! 時刻の土台（時刻・長さ・符号つきの差・倍率・時計）

mod convert;
mod delta;
mod point;
mod scale;
mod span;
mod text;

pub(crate) mod clock;
// shortcut: no field uses the adapters until the cue and document types move to the time types; drop the allow then.
#[allow(dead_code)]
pub(crate) mod serde_secs;
#[allow(dead_code)]
pub(crate) mod serde_ticks;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;

pub use convert::TimeRangeError;
pub use delta::TimeDelta;
pub use point::TimePoint;
pub use scale::TimeScale;
pub use span::TimeSpan;
pub use text::TimeParseError;

#[cfg(windows)]
pub use clock::now;
