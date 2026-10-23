//! `log_history` の写しの窓の決定論テスト（本文を切らない・warn 以上だけ・スレッド・閉じた窓）。
//!
//! 窓はプロセスに 1 つなので、窓を触るテストは [`WINDOW`] を握って順に回す。
//! 層を通った写しは exe の試験で確かめる（単体テストはプロセスの受け口を据えない）。

use std::sync::Mutex;

use super::*;

/// 窓を触るテストの排他（窓を開け閉めする書き出しの走りの試験 `offline_dump_tests` も握る）。
static WINDOW: Mutex<()> = Mutex::new(());

pub(crate) fn hold() -> MutexGuard<'static, ()> {
    WINDOW.lock().unwrap_or_else(PoisonError::into_inner)
}

const AT: Hms = Hms {
    hour: 9,
    minute: 8,
    second: 7,
};

fn field<'a>(name: &'a str, debug: &'a str) -> FieldText<'a> {
    FieldText {
        name,
        debug,
        raw: None,
    }
}

#[test]
fn body_text_does_not_truncate_but_draft_does() {
    let long = "あ".repeat(BODY_CAP_CHARS + 10);
    let msg = FieldText {
        name: "message",
        debug: "",
        raw: Some(&long),
    };
    let (body, _, _) = body_text(false, [msg, field("code", "5")]);
    assert_eq!(body, format!("{long} code=5"));
    let msg = FieldText {
        name: "message",
        debug: "",
        raw: Some(&long),
    };
    let d = draft(Level::WARN, "kanade", [msg]).unwrap();
    assert!(d.body.ends_with(TRUNCATED_SUFFIX));
    assert_eq!(
        d.body.chars().count(),
        BODY_CAP_CHARS + TRUNCATED_SUFFIX.chars().count()
    );
}

#[test]
fn only_warn_and_error_events_are_wanted() {
    assert!(capture_wants(true, Level::ERROR));
    assert!(capture_wants(true, Level::WARN));
    for level in [Level::INFO, Level::DEBUG, Level::TRACE] {
        assert!(!capture_wants(true, level));
    }
    // スパンは関心なし。ただし出来事でない問い合わせはフィルタが通す（`passes` と同じ理由）。
    assert!(!capture_wants(false, Level::ERROR));
    assert!(capture_passes(false, Level::TRACE));
    assert!(capture_passes(true, Level::WARN));
    assert!(!capture_passes(true, Level::INFO));
}

#[test]
fn only_the_thread_that_opened_the_window_is_captured() {
    let _g = hold();
    capture_begin();
    std::thread::spawn(|| capture_push(AT, Level::ERROR, "よそ".into()))
        .join()
        .unwrap();
    capture_push(AT, Level::WARN, "ここ1".into());
    capture_push(AT, Level::ERROR, "ここ2".into());
    let got = capture_finish();
    let want = [(Level::WARN, "ここ1"), (Level::ERROR, "ここ2")];
    assert_eq!(got.len(), want.len());
    for (r, (level, body)) in got.iter().zip(want) {
        assert_eq!((r.at, r.level, r.body.as_str()), (AT, level, body));
    }
}

#[test]
fn closed_window_keeps_nothing_and_reopening_clears() {
    let _g = hold();
    let _ = capture_finish();
    capture_push(AT, Level::ERROR, "閉じている".into());
    assert_eq!(capture_finish(), vec![]);
    capture_begin();
    capture_push(AT, Level::ERROR, "古い".into());
    capture_begin();
    capture_push(AT, Level::WARN, "新しい".into());
    let got = capture_finish();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].body, "新しい");
    // 閉じた後は積まない。
    capture_push(AT, Level::ERROR, "後".into());
    assert_eq!(capture_finish(), vec![]);
}
