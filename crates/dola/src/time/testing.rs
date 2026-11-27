//! テストの組み立て（ミリ秒の値から時刻・長さ・符号つきの差を作る）

use super::{TimeDelta, TimePoint, TimeSpan};

/// ミリ秒の値の時刻。端を越える値は端へ張り付ける。
pub const fn at_ms(ms: i64) -> TimePoint {
    TimePoint::from_ticks(TimeDelta::from_millis_saturating(ms).as_ticks())
}

/// ミリ秒の値の長さ。
pub const fn ms(ms: u32) -> TimeSpan {
    TimeSpan::from_millis(ms)
}

/// ミリ秒の値の符号つきの差。端を越える値は端へ張り付ける。
pub const fn delta_ms(ms: i64) -> TimeDelta {
    TimeDelta::from_millis_saturating(ms)
}
