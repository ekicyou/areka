//! 置き場の変更の記録を渡す順と、記録の途中で倒れたときの決定論テスト。
//!
//! 記録の中身は差し替え、`roots-log` へ渡した順に残ること・倒れた回の理由が残ること・
//! 倒れた後と受け手の居ない送り先でも次の記録が残ることを見る。

use std::path::Path;
use std::sync::mpsc;

use areka_runtime::spine::SPIN_WAIT;
use log_capture_kit::capture;

use super::*;
use crate::roots::roots_test_support::roots_of;

/// 根の数を `listed` に残して済んだことを知らせる。並びが空なら倒れる。
fn work(done: mpsc::Sender<()>) -> impl Fn(BasewareRoots) + Send + 'static {
    move |roots| {
        assert!(!roots.roots().is_empty(), "empty list");
        tracing::info!(event = "listed", n = roots.roots().len());
        let _ = done.send(());
    }
}

#[test]
fn records_keep_the_queued_order_and_survive_a_panic() {
    let slot = ChangeLogSlot::new(None);
    let (done_tx, done_rx) = mpsc::channel();
    let (a, b) = (Path::new("a"), Path::new("b"));

    let ((), events) = capture(|| {
        queue_change_log(&slot, roots_of(&[a]), work(done_tx.clone()));
        queue_change_log(&slot, roots_of(&[]), work(done_tx.clone()));
        queue_change_log(&slot, roots_of(&[a, b]), work(done_tx.clone()));
        for _ in 0..2 {
            done_rx.recv_timeout(SPIN_WAIT).expect("記録が済む");
        }
    });

    let got: Vec<_> = events
        .iter()
        .filter_map(|e| {
            Some((
                e.field_str("event")?,
                e.field("n").or(e.field_str("reason"))?,
            ))
        })
        .collect();
    assert_eq!(
        got,
        [
            ("listed", "1"),
            ("roots_log_spawn_failed", "panicked"),
            ("listed", "2"),
        ]
    );
    let detail = events
        .iter()
        .find_map(|e| e.field("detail"))
        .expect("倒れた回の中身が残る");
    assert_eq!(detail, "empty list");
}

#[test]
fn dead_receiver_gets_a_fresh_thread() {
    let (dead, _) = mpsc::channel::<ChangeLogJob>();
    let slot = ChangeLogSlot::new(Some(dead));
    let (done_tx, done_rx) = mpsc::channel();

    queue_change_log(&slot, roots_of(&[Path::new("a")]), work(done_tx));

    done_rx
        .recv_timeout(SPIN_WAIT)
        .expect("起こし直したスレッドが記録する");
}
