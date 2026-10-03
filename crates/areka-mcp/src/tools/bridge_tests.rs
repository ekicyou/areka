//! `bridge` の決定論テスト（上限・終了の途中・記録の件数）。

use areka_actor::ReplyError;

use super::*;
use crate::tools::{ToolCall, outcome};

fn new_pair() -> (ToolRequest, Pending) {
    ToolRequest::new(ToolCall::GetActiveGhostList)
}

/// `ReplyTo` は別スレッド（kanade など）から送れる。`ToolRequest` は UI スレッドへ渡る。
#[test]
fn reply_to_and_tool_request_are_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ReplyTo>();
    assert_send::<ToolRequest>();
}

#[test]
fn new_keeps_the_call() {
    let (request, _pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    assert_eq!(request.call, ToolCall::GetActiveGhostList);
}

#[test]
fn is_abandoned_is_false_while_pending_lives_and_true_after_drop() {
    let (request, pending) = new_pair();
    assert!(!request.reply.is_abandoned());
    drop(pending);
    assert!(request.reply.is_abandoned());
}

#[test]
fn try_answer_is_none_before_send() {
    let (_request, pending) = new_pair();
    assert!(matches!(pending.try_answer(), Ok(None)));
    assert!(!pending.answered().is_cancelled());
}

#[test]
fn send_delivers_answer_with_ghost_label() {
    let (request, pending) = new_pair();
    request.reply.for_ghost("emo").send(outcome::value("x"));
    assert_eq!(
        pending.try_answer().ok(),
        Some(Some(Answer {
            ghost: "emo".to_string(),
            outcome: outcome::value("x"),
        }))
    );
}

#[test]
fn send_without_ghost_label_has_empty_ghost() {
    let (request, pending) = new_pair();
    request.reply.send(outcome::ng("no"));
    assert_eq!(
        pending.try_answer().ok(),
        Some(Some(Answer {
            ghost: String::new(),
            outcome: outcome::ng("no"),
        }))
    );
}

#[test]
fn answered_signal_is_raised_after_send() {
    let (request, pending) = new_pair();
    let answered = pending.answered();
    request.reply.send(outcome::ok(""));
    assert!(answered.is_cancelled());
}

#[test]
fn drop_without_send_raises_signal_and_try_answer_is_dropped() {
    let (request, pending) = new_pair();
    let answered = pending.answered();
    drop(request);
    assert!(answered.is_cancelled());
    assert!(matches!(pending.try_answer(), Err(ReplyError::Dropped)));
}

#[test]
fn send_after_pending_dropped_does_nothing() {
    let (request, pending) = new_pair();
    let answered = pending.answered();
    drop(pending);
    // 受け手が居ないので黙って捨てる（panic しない）。
    request.reply.for_ghost("emo").send(outcome::value("late"));
    assert!(answered.is_cancelled());
}

// ---- 送って待つ（`call`）: テストのスレッドで tokio を回して記録を数える ----

use std::sync::mpsc;
use std::time::{Duration, Instant};

use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use crate::ToolOutcome;
use crate::tools::REPLY_WAIT;

/// 終了の途中の枝が「待たずに返った」と言える経過の上限（長い上限 60 秒に比べて十分短い）。
const LONG: Duration = Duration::from_secs(60);
const QUICK: Duration = Duration::from_secs(5);

fn count(events: &[CapturedEvent], level: Level) -> usize {
    events.iter().filter(|e| e.level == level).count()
}

/// `call` をテストのスレッドの current_thread で回し、結果・記録・経過を返す。
fn run(
    tx: mpsc::Sender<ToolRequest>,
    limit: Duration,
) -> (ToolOutcome, Vec<CapturedEvent>, Duration) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("tokio");
    let started = Instant::now();
    let (got, events) = capture(|| runtime.block_on(call(tx, ToolCall::GetActiveGhostList, limit)));
    (got, events, started.elapsed())
}

fn the_debug(events: &[CapturedEvent]) -> &CapturedEvent {
    let debugs: Vec<_> = events.iter().filter(|e| e.level == Level::DEBUG).collect();
    assert_eq!(debugs.len(), 1, "debug! は 1 件: {events:?}");
    debugs[0]
}

/// (要件 6.8) `call` のフューチャは `Send`（登録表の処理の約束）。
#[test]
fn call_future_is_send() {
    fn assert_send<T: Send>(_: &T) {}
    let (tx, _rx) = mpsc::channel();
    let future = call(tx, ToolCall::GetActiveGhostList, LONG);
    assert_send(&future);
}

/// ⑹ (要件 6.2) 本番の上限の文言。
#[test]
fn timeout_text_of_reply_wait() {
    assert_eq!(
        timeout_text(REPLY_WAIT),
        "areka did not respond within 10 seconds"
    );
}

/// ⑴ (要件 6.2, 6.7, 6.8) 返事をしない受け手・上限 50 ms → 上限の `NG:`・warn! 1 件・debug! 1 件。
#[test]
fn no_reply_times_out_with_one_warn_and_one_debug() {
    let limit = Duration::from_millis(50);
    let (tx, _rx) = mpsc::channel();
    let (got, events, _) = run(tx, limit);
    assert_eq!(got, outcome::ng(timeout_text(limit)));
    assert!(got.is_error);
    assert_eq!(count(&events, Level::WARN), 1, "{events:?}");
    let warn = events.iter().find(|e| e.level == Level::WARN).unwrap();
    assert_eq!(warn.field_str("tool"), Some("get_active_ghost_list"));
    assert_eq!(warn.field("limit_ms"), Some("50"));
    let debug = the_debug(&events);
    assert_eq!(debug.field_str("tool"), Some("get_active_ghost_list"));
    assert_eq!(debug.field_str("ghost"), Some(""));
    assert_eq!(debug.field("is_error"), Some("true"));
    assert_eq!(
        debug.field_str("text"),
        Some(format!("NG:{}", timeout_text(limit)).as_str())
    );
    assert_eq!(events.len(), 2, "{events:?}");
}

/// ⑵ (要件 6.4, 6.8) 受け口を落としてから呼ぶ → 上限を待たずに shutting down・warn! 1 件。
#[test]
fn closed_inbox_answers_shutting_down_without_waiting() {
    let (tx, rx) = mpsc::channel();
    drop(rx);
    let (got, events, elapsed) = run(tx, LONG);
    assert_eq!(got, outcome::ng("areka is shutting down"));
    assert!(elapsed < QUICK, "待たずに返る: {elapsed:?}");
    assert_eq!(count(&events, Level::WARN), 1, "{events:?}");
    let debug = the_debug(&events);
    assert_eq!(debug.field_str("text"), Some("NG:areka is shutting down"));
    assert_eq!(events.len(), 2, "{events:?}");
}

/// ⑶ (要件 6.4, 6.8) 受けた要求を答えずに落とす → 上限を待たずに shutting down・warn! 1 件。
#[test]
fn dropped_request_answers_shutting_down_without_waiting() {
    let (tx, rx) = mpsc::channel::<ToolRequest>();
    let receiver = std::thread::spawn(move || {
        let request = rx.recv().expect("要求が届く");
        drop(request);
    });
    let (got, events, elapsed) = run(tx, LONG);
    receiver.join().unwrap();
    assert_eq!(got, outcome::ng("areka is shutting down"));
    assert!(elapsed < QUICK, "待たずに返る: {elapsed:?}");
    assert_eq!(count(&events, Level::WARN), 1, "{events:?}");
    the_debug(&events);
    assert_eq!(events.len(), 2, "{events:?}");
}

/// ⑷ (要件 6.1, 6.7) 返事あり → その結果・warn! 0 件・debug! にゴーストの名前。
#[test]
fn answer_is_returned_with_ghost_in_debug() {
    let (tx, rx) = mpsc::channel::<ToolRequest>();
    let receiver = std::thread::spawn(move || {
        let request = rx.recv().expect("要求が届く");
        assert_eq!(request.call, ToolCall::GetActiveGhostList);
        request.reply.for_ghost("emo").send(outcome::value("emo"));
    });
    let (got, events, elapsed) = run(tx, LONG);
    receiver.join().unwrap();
    assert_eq!(got, outcome::value("emo"));
    assert!(elapsed < QUICK, "返事で起きる: {elapsed:?}");
    assert_eq!(count(&events, Level::WARN), 0, "{events:?}");
    let debug = the_debug(&events);
    assert_eq!(debug.field_str("tool"), Some("get_active_ghost_list"));
    assert_eq!(debug.field_str("ghost"), Some("emo"));
    assert_eq!(debug.field("is_error"), Some("false"));
    assert_eq!(debug.field_str("text"), Some(""));
    assert_eq!(events.len(), 1, "{events:?}");
}

/// ⑷' (要件 6.7) 返事が失敗なら debug! の text に失敗の本文が載る。
#[test]
fn failed_answer_puts_text_in_debug() {
    let (tx, rx) = mpsc::channel::<ToolRequest>();
    let receiver = std::thread::spawn(move || {
        let request = rx.recv().expect("要求が届く");
        request
            .reply
            .send(outcome::ng("Specified ghost is not active"));
    });
    let (got, events, _) = run(tx, LONG);
    receiver.join().unwrap();
    assert_eq!(got, outcome::ng("Specified ghost is not active"));
    let debug = the_debug(&events);
    assert_eq!(debug.field_str("ghost"), Some(""));
    assert_eq!(debug.field("is_error"), Some("true"));
    assert_eq!(
        debug.field_str("text"),
        Some("NG:Specified ghost is not active")
    );
    assert_eq!(count(&events, Level::WARN), 0, "{events:?}");
}

/// ⑸ (要件 6.2) 上限の後の送りは何も起こさない（待つ側はもう居ない・panic しない）。
#[test]
fn send_after_timeout_does_nothing() {
    let limit = Duration::from_millis(50);
    let (tx, rx) = mpsc::channel::<ToolRequest>();
    let (got, _, _) = run(tx, limit);
    assert_eq!(got, outcome::ng(timeout_text(limit)));
    // 要求は受け口に溜まったまま。上限の後に取り出して答える。
    let request = rx.try_recv().expect("要求は届いている");
    assert!(request.reply.is_abandoned());
    let ((), events) = capture(|| request.reply.for_ghost("emo").send(outcome::value("late")));
    assert!(events.is_empty(), "{events:?}");
}
