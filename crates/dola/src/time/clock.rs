//! ただ 1 つの時計（Windows の時だけ）
#![cfg(windows)]

use windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTimePrecise;

use super::TimePoint;

/// 今の時刻。眠り（スリープ・休止）の間は進まない。プロセスごとの起点も倍率も持たない。
pub fn now() -> TimePoint {
    // SAFETY: 引数もポインタも渡さない読み取りで、呼ぶ側に前提が無い。
    let ticks = unsafe { QueryUnbiasedInterruptTimePrecise() };
    TimePoint::from_ticks(i64::try_from(ticks).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;
