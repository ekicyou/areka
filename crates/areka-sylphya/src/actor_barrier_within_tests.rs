//! 期限つきの反映待ちの檻。
//!
//! - 覚える値の書きが別の書き手の錠を待っている間、期限つきの待ちは `Timeout` と ERROR 1 件を返し、
//!   錠を放した後は `Ok`。
//! - 閉じた後は `Dropped`。

use super::*;
use crate::persist::ScopeRoots;
use crate::store::MemBackend;
use crate::test_log_capture::capture;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

/// 待ちの上限（壊れた作りでもテストが止まらずに赤で終わる）。
const BOUND: Duration = Duration::from_secs(5);

fn ghost() -> PathBuf {
    PathBuf::from("/g")
}

fn init(mem: &Arc<MemBackend>) -> SylphyaInit {
    SylphyaInit::new(
        ScopeRoots {
            ghost: Some(ghost()),
            ..ScopeRoots::default()
        },
        mem.clone(),
    )
}

#[test]
fn barrier_within_times_out_while_the_writer_is_busy() {
    let mem = MemBackend::new();
    let hold = mem.hold_writer(&ghost());
    let parts = spawn_sylphya(init(&mem));
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "1".into())],
    );

    // 期限つきの待ちは別のスレッドで。期限が無ければ錠を放すまで止まり、下の 5 秒の待ちが切れて赤。
    let (answer_tx, answer_rx) = mpsc::channel();
    let publisher = parts.publisher.clone();
    std::thread::spawn(move || {
        let mut result = None;
        let events = capture(|| result = Some(publisher.barrier_within(Duration::from_millis(10))));
        let errors = events
            .iter()
            .filter(|e| {
                e.level == tracing::Level::ERROR
                    && e.target == "areka_sylphya::actor"
                    && e.message().contains("sylphya barrier timed out")
            })
            .count();
        let _ = answer_tx.send((result.unwrap(), errors));
    });
    let answer = answer_rx.recv_timeout(BOUND);
    drop(hold);
    let after = parts.publisher.barrier_within(BOUND);
    parts.publisher.close();
    parts.handle.join().expect("join");

    let (result, errors) = answer.expect("期限つきの待ちが期限で戻る");
    assert!(
        matches!(result, Err(areka_actor::ReplyError::Timeout)),
        "書きが錠を待っている間は Timeout: {result:?}"
    );
    assert_eq!(errors, 1, "期限切れの ERROR は 1 件");
    assert!(after.is_ok(), "錠を放した後は Ok: {after:?}");
}

#[test]
fn barrier_within_reports_dropped_after_close() {
    let parts = spawn_sylphya(init(&MemBackend::new()));
    parts.publisher.close();
    parts.handle.join().expect("join");

    let result = parts.publisher.barrier_within(Duration::from_secs(1));
    assert!(
        matches!(result, Err(areka_actor::ReplyError::Dropped)),
        "閉じた後は Dropped: {result:?}"
    );
}
