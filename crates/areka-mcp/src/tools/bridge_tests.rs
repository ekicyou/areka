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
