// MoveCueSink の lock・unlock の名前の選別と実行者のスコープ。

use super::*;
use crate::consumer_ledger::test_support::actor_carrier_cue;
use dola::cue::{CueSink, TalkCue};
use std::sync::mpsc::channel;

/// 1 件の cue を受け口へ渡し、線に届いたものを全部返す。
fn emit_one(cue: TalkCue) -> Vec<PositionCue> {
    let (tx, rx) = channel::<PositionCue>();
    MoveCueSink::new(tx).emit(cue);
    rx.try_iter().collect()
}

/// `\0`・`\1` で書いた lock・unlock は、書いた所（実行者）のスコープで合図が 1 件だけ届く。
#[test]
fn lock_and_unlock_send_one_cue_in_actor_scope() {
    for (actor, scope) in [("0", 0), ("1", 1)] {
        for (name, locked) in [("lock", true), ("unlock", false)] {
            assert_eq!(
                emit_one(actor_carrier_cue(actor, name, &["balloonmove"])),
                vec![PositionCue::BalloonMoveLock { scope, locked }],
                "\\{actor} の \\![{name},balloonmove]"
            );
        }
    }
}

/// 第 1 引数が balloonmove でない lock・unlock は担当外で、何も送らない。
#[test]
fn lock_with_other_first_arg_is_not_mine() {
    for (name, tokens) in [
        ("lock", &["repaint"][..]),
        ("unlock", &["repaint"][..]),
        ("lock", &[][..]),
    ] {
        assert!(
            emit_one(actor_carrier_cue("0", name, tokens)).is_empty(),
            "\\![{name},{tokens:?}] は送らない"
        );
    }
}

/// 余分な引数（`\![lock,balloonmove,foo]`）は読まずに受ける。
#[test]
fn extra_args_are_accepted_unread() {
    assert_eq!(
        emit_one(actor_carrier_cue(
            "1",
            "lock",
            &["balloonmove", "foo", "bar"]
        )),
        vec![PositionCue::BalloonMoveLock {
            scope: 1,
            locked: true
        }]
    );
    assert_eq!(
        emit_one(actor_carrier_cue("0", "unlock", &["balloonmove", "foo"])),
        vec![PositionCue::BalloonMoveLock {
            scope: 0,
            locked: false
        }]
    );
}

/// 実行者が数でない lock は `warn!` 1 件で捨てる（`actor_scope` の扱い）。
#[test]
fn non_numeric_actor_sends_nothing() {
    let (sent, events) =
        log_capture_kit::capture(|| emit_one(actor_carrier_cue("x", "lock", &["balloonmove"])));
    assert_eq!(sent, vec![], "数でない実行者の lock は送らない");
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "{events:?}");
    assert_eq!(warns[0].field_str("event"), Some("move_cue_bad_actor"));
    assert_eq!(warns[0].field_str("tag"), Some("lock,balloonmove"));
}
