//! 刻みと seriko が同じ時計を読むことの檻。

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use areka_ghost::ticker::LoopTickerConfig;
use areka_seriko::SerikoClock;
use dola::TimePoint;

use super::loop_ticker_config;

#[test]
fn ticker_and_seriko_read_the_same_clock() {
    let now = Arc::new(AtomicI64::new(1_000));
    let source = Arc::clone(&now);
    let seriko_clock: SerikoClock =
        Arc::new(move || TimePoint::from_ticks(source.load(Ordering::SeqCst)));
    let config = loop_ticker_config(&seriko_clock);

    for t in [1_000, 1_016, 987_654_321] {
        now.store(t, Ordering::SeqCst);
        assert_eq!((config.clock)(), TimePoint::from_ticks(t), "刻みの時計");
        assert_eq!(seriko_clock(), TimePoint::from_ticks(t), "seriko の時計");
    }
    // 周期は既定（16 ms）のまま。
    assert_eq!(config.interval, LoopTickerConfig::default().interval);
}
