//! 保存の「読んで重ねて書く」を錠で 1 本ずつにする檻（#3・要件 4.1・4.2）。
//!
//! - 同じプロセスの 2 つの書き手: A が読みで止まっている間 B は終わらず、門を開けると両方の値が残る。
//! - 錠が取れない: `Degraded(Io(..))`・書き込みは呼ばれない・ERROR `persist_lock_failed` が 1 件。
//!
//! 門（偽の IO の中の止め）はどのテストでも必ずテストの中で開く。テストが落ちても送り手が
//! 捨てられて開く（止めている間、同じバイナリのほかのテストの保存も `SAVE_GATE` で待つため）。

use super::*;
use crate::test_log_capture::capture;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// 止めの待ちの上限（壊れた作りでもテストが止まらずに赤で終わる）。
const BOUND: Duration = Duration::from_secs(5);

fn ghost_roots() -> ScopeRoots {
    ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    }
}

/// 1 回目の読みで中身を読んだ後に「入った」を知らせ、門が開く（送り手が送るか捨てられる）まで止まる偽の IO。
struct GatedFirstRead {
    inner: FakePersistIo,
    entered: Sender<()>,
    open: Mutex<Option<Receiver<()>>>,
}

impl PersistIo for GatedFirstRead {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        // 読んだ中身を持ったまま止まる（錠が無ければ、その間の B の書き込みを A が上書きで消す）。
        let content = self.inner.read(path);
        // 受け手は文を分けて取り出す（if let の行に書くと、止まっている間も自分の Mutex を握り続ける）。
        let open = self.open.lock().unwrap().take();
        if let Some(open) = open {
            let _ = self.entered.send(());
            let _ = open.recv_timeout(BOUND);
        }
        content
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.inner.commit(path, content)
    }
}

#[test]
fn two_writers_in_one_process_do_not_lose_an_update() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (open_tx, open_rx) = mpsc::channel::<()>();
    let io = GatedFirstRead {
        inner: FakePersistIo::new(),
        entered: entered_tx,
        open: Mutex::new(Some(open_rx)),
    };
    let roots = ghost_roots();

    let (b_finished_early, outcomes) = std::thread::scope(|s| {
        let a = s.spawn(|| {
            save_scope(
                PersistScope::Ghost,
                &roots,
                &io,
                vec![(PersistKey::BootCount, "7".into())],
            )
        });
        entered_rx
            .recv_timeout(BOUND)
            .expect("書き手 A が読みに入る");
        let (done_tx, done_rx) = mpsc::channel();
        let (roots, io) = (&roots, &io);
        let b = s.spawn(move || {
            let outcome = save_scope(
                PersistScope::Ghost,
                roots,
                io,
                vec![(PersistKey::LastGhost, "emo2".into())],
            );
            let _ = done_tx.send(());
            outcome
        });
        let early = done_rx.recv_timeout(Duration::from_millis(200));
        drop(open_tx); // 門を開ける
        let outcomes = (a.join().unwrap(), b.join().unwrap());
        (early != Err(RecvTimeoutError::Timeout), outcomes)
    });

    assert_eq!(outcomes, (PersistOutcome::Saved, PersistOutcome::Saved));
    let loaded = load_scope(PersistScope::Ghost, &roots, &io.inner);
    assert!(
        loaded.contains(&(PersistKey::BootCount, "7".into()))
            && loaded.contains(&(PersistKey::LastGhost, "emo2".into())),
        "両方の値がファイルに残る: {loaded:?}"
    );
    assert!(!b_finished_early, "A が読みの間、B は錠で並んで終わらない");
}

/// 錠がすぐ取れず、書き込みの回数を数える偽の IO。
#[derive(Default)]
struct LockRefused {
    commits: AtomicUsize,
}

impl PersistIo for LockRefused {
    fn read(&self, _path: &Path) -> std::io::Result<Option<String>> {
        Ok(None)
    }
    fn commit(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
        self.commits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn lock_for_update(&self, _path: &Path) -> std::io::Result<Option<std::fs::File>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "LockRefused: injected lock failure",
        ))
    }
}

#[test]
fn a_lock_that_cannot_be_taken_degrades_the_save_and_writes_nothing() {
    let io = LockRefused::default();
    let mut outcome = None;
    let events = capture(|| {
        outcome = Some(save_scope(
            PersistScope::Ghost,
            &ghost_roots(),
            &io,
            vec![(PersistKey::BootCount, "7".into())],
        ))
    });
    let outcome = outcome.unwrap();
    let PersistOutcome::Degraded(failure) = &outcome else {
        panic!("錠が取れない保存は Degraded になる: {outcome:?}");
    };
    assert!(
        matches!(&failure.cause, PersistFailureCause::Io(r) if r.contains("injected lock failure")),
        "理由は Io で運ぶ: {failure:?}"
    );
    assert_eq!(io.commits.load(Ordering::SeqCst), 0, "書き込みは呼ばれない");
    let lock_failed = events
        .iter()
        .filter(|e| {
            e.target == LOG_TARGET
                && e.level == tracing::Level::ERROR
                && e.message().contains("persist update lock not acquired")
        })
        .count();
    assert_eq!(
        lock_failed, 1,
        "ERROR persist_lock_failed が 1 件: {events:?}"
    );
}
