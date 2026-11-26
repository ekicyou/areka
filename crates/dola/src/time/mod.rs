//! 時刻の土台（時刻・長さ・符号つきの差・倍率・時計）

mod convert;
mod delta;
mod point;
mod scale;
mod span;
mod text;

pub(crate) mod clock;
pub(crate) mod serde_secs;
pub(crate) mod serde_ticks;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;

pub use convert::TimeRangeError;
pub use delta::TimeDelta;
pub use point::TimePoint;
pub use scale::TimeScale;
pub use span::TimeSpan;
pub use text::TimeParseError;

// shortcut: clock.rs has no `now` yet; name `now` here and drop the allow once it exists.
#[cfg(windows)]
#[allow(unused_imports)]
pub use clock::*;
