//! 「シェル位置初期化」の葉の檻（areka-P0-move-canon 要件 8.3〜8.5・design ResetPosFrame）。
//!
//! 並べ直したかは [`reset_window_positions`] の記録で見る: 材料の無い World では
//! `reset_window_pos_skipped` が必ず 1 行出るので、その行の有無が「呼んだか」になる。

use std::path::PathBuf;
use std::sync::mpsc;

use areka_actor::{ReplyError, reply_channel};
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod};
use bevy_ecs::world::World;

use super::{
    PendingResetRaise, ResetDecision, decide_reset, poll_reset_raise, raise_reset_window_pos,
};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::placement::test_support::{LogEvent, capture_logs};

fn count(events: &[LogEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

fn outcome_of(events: &[LogEvent]) -> Vec<(tracing::Level, String)> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("reset_pos_menu_outcome"))
        .map(|e| {
            let o = e
                .field_str("outcome")
                .map(str::to_owned)
                .or_else(|| e.field("outcome").map(str::to_owned))
                .unwrap_or_default();
            (e.level, o)
        })
        .collect()
}

#[test]
fn decide_reset_classifies_all_seven_answers() {
    use ResetDecision::*;
    assert_eq!(decide_reset(Ok(None)), Wait, "まだ");
    assert_eq!(decide_reset(Ok(Some(RaiseOutcome::Script))), Leave, "台本");
    assert_eq!(
        decide_reset(Ok(Some(RaiseOutcome::NoReply))),
        Reset,
        "返事なし"
    );
    assert_eq!(
        decide_reset(Ok(Some(RaiseOutcome::NotSteady))),
        Reset,
        "NotSteady"
    );
    assert_eq!(
        decide_reset(Ok(Some(RaiseOutcome::NotAllowed))),
        Reset,
        "NotAllowed"
    );
    assert_eq!(
        decide_reset(Ok(Some(RaiseOutcome::Failed))),
        Reset,
        "Failed"
    );
    assert_eq!(
        decide_reset(Err(ReplyError::Dropped)),
        Leave,
        "受け手が切れた"
    );
}

#[test]
fn raise_without_sender_resets_at_once() {
    let mut world = World::new();
    let (_, events) = capture_logs(|| raise_reset_window_pos(&mut world));
    assert!(world.get_non_send::<PendingResetRaise>().is_none());
    assert_eq!(
        outcome_of(&events),
        vec![(tracing::Level::WARN, "not_sent".to_owned())]
    );
    assert!(
        count(&events, "reset_window_pos_skipped") >= 1,
        "並べ直しを呼ぶ: {events:?}"
    );
}

#[test]
fn raise_with_closed_sender_resets_at_once() {
    let (tx, rx) = mpsc::channel();
    drop(rx);
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/A"),
    ))));
    let (_, events) = capture_logs(|| raise_reset_window_pos(&mut world));
    assert!(world.get_non_send::<PendingResetRaise>().is_none());
    assert!(
        count(&events, "reset_window_pos_skipped") >= 1,
        "並べ直しを呼ぶ: {events:?}"
    );
}

#[test]
fn raise_sends_event_and_holds_one_pending_receiver() {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/A"),
    ))));
    let (_, events) = capture_logs(|| raise_reset_window_pos(&mut world));
    assert_eq!(
        count(&events, "reset_window_pos_skipped"),
        0,
        "送れたら並べ直さない"
    );
    assert!(world.get_non_send::<PendingResetRaise>().is_some());
    match rx.try_recv().expect("kanade へ届く") {
        KanadeMsg::RaiseEvent {
            id,
            references,
            method,
            reply,
        } => {
            assert_eq!(id, "OnResetWindowPos");
            assert!(references.is_empty());
            assert_eq!(method, ShioriMethod::Get);
            assert!(reply.is_some(), "答えの受け手つき");
        }
        _ => panic!("RaiseEvent でない"),
    }
}

/// 答えを置いた保留で 1 tick 回し、（保留が残ったか, 記録）を返す。`None` は答えを送らない。
fn poll_with(answer: Option<RaiseOutcome>, drop_sender: bool) -> (bool, Vec<LogEvent>) {
    let (tx, rx) = reply_channel::<RaiseOutcome>();
    let mut world = World::new();
    world.insert_non_send(PendingResetRaise(rx));
    let _alive;
    if let Some(a) = answer {
        tx.send(a).expect("受け手は生きている");
    } else if drop_sender {
        drop(tx);
    } else {
        _alive = tx;
    }
    let (_, events) = capture_logs(|| poll_reset_raise(&mut world));
    (world.get_non_send::<PendingResetRaise>().is_some(), events)
}

#[test]
fn poll_waits_while_no_answer() {
    let (pending, events) = poll_with(None, false);
    assert!(pending, "まだなら保留を残す");
    assert!(outcome_of(&events).is_empty());
    assert_eq!(count(&events, "reset_window_pos_skipped"), 0);
}

#[test]
fn poll_script_leaves_positions_and_clears_pending() {
    let (pending, events) = poll_with(Some(RaiseOutcome::Script), false);
    assert!(!pending);
    assert_eq!(
        outcome_of(&events),
        vec![(tracing::Level::INFO, "script".to_owned())]
    );
    assert_eq!(
        count(&events, "reset_window_pos_skipped"),
        0,
        "並べ直さない"
    );
}

#[test]
fn poll_no_reply_resets_and_clears_pending() {
    let (pending, events) = poll_with(Some(RaiseOutcome::NoReply), false);
    assert!(!pending);
    assert_eq!(
        outcome_of(&events),
        vec![(tracing::Level::INFO, "no_reply".to_owned())]
    );
    assert!(count(&events, "reset_window_pos_skipped") >= 1, "並べ直す");
}

#[test]
fn poll_failures_reset_with_warn_and_clear_pending() {
    for answer in [
        RaiseOutcome::NotSteady,
        RaiseOutcome::NotAllowed,
        RaiseOutcome::Failed,
    ] {
        let (pending, events) = poll_with(Some(answer), false);
        assert!(!pending, "{answer:?}");
        assert_eq!(
            outcome_of(&events),
            vec![(tracing::Level::WARN, format!("{answer:?}"))],
            "{answer:?}"
        );
        assert!(
            count(&events, "reset_window_pos_skipped") >= 1,
            "{answer:?} は並べ直す"
        );
    }
}

#[test]
fn poll_dropped_receiver_warns_without_reset_and_clears_pending() {
    let (pending, events) = poll_with(None, true);
    assert!(!pending);
    assert_eq!(
        outcome_of(&events),
        vec![(tracing::Level::WARN, "dropped".to_owned())]
    );
    assert_eq!(
        count(&events, "reset_window_pos_skipped"),
        0,
        "並べ直さない"
    );
}
