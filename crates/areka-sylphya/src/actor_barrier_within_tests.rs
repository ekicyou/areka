//! 期限つきの反映待ちの檻（audit-sylphya-actor #4・要件 4.1・4.2）。
//!
//! - 書き手が門で止まっている間、期限つきの待ちは `Timeout` と ERROR 1 件を返し、門の後は `Ok`。
//! - 閉じた後は `Dropped`。
//!
//! 門（偽の IO の書き込みの中の止め）は必ずテストの中で開く。テストが落ちても送り手が捨てられて
//! 開く（止めている間、同じバイナリのほかのテストの保存も `SAVE_GATE` で待つため）。

use super::*;
use crate::test_log_capture::capture;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};

/// 止めの待ちの上限（壊れた作りでもテストが止まらずに赤で終わる）。
const BOUND: Duration = Duration::from_secs(5);

/// 1 回目の書き込みで「入った」を知らせ、門が開く（送り手が送るか捨てられる）まで止まる偽の IO。
struct GatedFirstCommit {
    entered: Sender<()>,
    open: Mutex<Option<Receiver<()>>>,
}

impl PersistIo for GatedFirstCommit {
    fn read(&self, _path: &Path) -> std::io::Result<Option<String>> {
        Ok(None)
    }
    fn commit(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
        // 受け手は文を分けて取り出す（if let の行に書くと、止まっている間も自分の Mutex を握り続ける）。
        let open = self.open.lock().unwrap().take();
        if let Some(open) = open {
            let _ = self.entered.send(());
            let _ = open.recv_timeout(BOUND);
        }
        Ok(())
    }
}

fn init(io: GatedFirstCommit) -> SylphyaInit {
    SylphyaInit {
        roots: ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        },
        io: Box::new(io),
        runtime_sink: None,
    }
}

#[test]
fn barrier_within_times_out_while_the_writer_is_busy() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (open_tx, open_rx) = mpsc::channel::<()>();
    let parts = spawn_sylphya(init(GatedFirstCommit {
        entered: entered_tx,
        open: Mutex::new(Some(open_rx)),
    }));
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "1".into())],
    );
    entered_rx
        .recv_timeout(BOUND)
        .expect("書き手が書き込みに入る");

    // 期限つきの待ちは別のスレッドで。期限が無ければ門の前で止まり、下の 5 秒の待ちが切れて赤。
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
    drop(open_tx); // 門を開ける（赤でもここで開けてから畳む）
    let after = parts.publisher.barrier_within(BOUND);
    parts.publisher.close();
    parts.handle.join().expect("join");

    let (result, errors) = answer.expect("期限つきの待ちが期限で戻る");
    assert!(
        matches!(result, Err(areka_actor::ReplyError::Timeout)),
        "書き手が止まっている間は Timeout: {result:?}"
    );
    assert_eq!(errors, 1, "期限切れの ERROR は 1 件");
    assert!(after.is_ok(), "門の後は Ok: {after:?}");
}

#[test]
fn barrier_within_reports_dropped_after_close() {
    let (entered_tx, _entered_rx) = mpsc::channel();
    let parts = spawn_sylphya(init(GatedFirstCommit {
        entered: entered_tx,
        open: Mutex::new(None),
    }));
    parts.publisher.close();
    parts.handle.join().expect("join");

    let result = parts.publisher.barrier_within(Duration::from_secs(1));
    assert!(
        matches!(result, Err(areka_actor::ReplyError::Dropped)),
        "閉じた後は Dropped: {result:?}"
    );
}
