//! 現実の出来事に時刻を付ける、差し込める時計。

use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

use dola::TimePoint;

/// 現実の出来事（トークの受け口が落ちた・利用者の押下を受けた）に付ける時刻を読む時計。
///
/// 起点も台本の時刻も持たない。トークのスレッドと UI のスレッドが同じ 1 つの時計を読み、テストは
/// 偽の時計へ替える（既定は `dola::now`＝`FrameTime` と同じ時計）。
#[derive(Clone)]
pub struct TalkClock {
    clock: Arc<dyn Fn() -> TimePoint + Send + Sync>,
}

impl TalkClock {
    /// 時計を差し込んで作る（本番は `dola::now` を渡す）。
    pub fn new(clock: Arc<dyn Fn() -> TimePoint + Send + Sync>) -> Self {
        Self { clock }
    }

    /// 今の時刻（差し込まれた時計の読み）。
    pub fn now(&self) -> TimePoint {
        (self.clock)()
    }
}

/// テスト用の可制御クロック: 返り値の `Arc<Mutex<TimePoint>>` に時刻を書けば、
/// `TalkClock` がその時刻を読む（決定論・注入クロック）。
#[cfg(test)]
pub(crate) fn controllable_clock() -> (Arc<Mutex<TimePoint>>, TalkClock) {
    let now = Arc::new(Mutex::new(TimePoint::ZERO));
    let now_for_clock = Arc::clone(&now);
    let clock = TalkClock::new(Arc::new(move || {
        *now_for_clock.lock().expect("test clock mutex poisoned")
    }));
    (now, clock)
}

/// 可制御クロックの時刻を書き換える。
#[cfg(test)]
pub(crate) fn set_now(now: &Arc<Mutex<TimePoint>>, wall: TimePoint) {
    *now.lock().expect("test clock mutex poisoned") = wall;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dola::time::testing::at_ms;

    #[test]
    fn now_reads_the_injected_clock_and_clones_share_it() {
        let (wall, clock) = controllable_clock();
        let copy = clock.clone();
        for t in [at_ms(0), at_ms(10_000), at_ms(-5_000)] {
            set_now(&wall, t);
            assert_eq!(clock.now(), t);
            assert_eq!(copy.now(), t, "複製も同じ時計を読む");
        }
    }
}
