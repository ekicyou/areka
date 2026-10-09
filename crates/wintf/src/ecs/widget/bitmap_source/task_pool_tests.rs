//! `WintfTaskPool::with_threads` の檻

use super::WintfTaskPool;

/// `with_threads(1)` のプールはスレッドを 1 本だけ持つ
#[test]
fn with_threads_one_builds_a_single_thread_pool() {
    assert_eq!(WintfTaskPool::with_threads(1).pool.thread_num(), 1);
}
