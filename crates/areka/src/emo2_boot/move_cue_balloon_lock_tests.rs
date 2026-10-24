// =============================================================================
// MoveCueSink の lock・unlock の名前の選別と実行者のスコープ（descript-window-keys・要件 5.1・5.2）
// =============================================================================

use super::*;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::channel;

/// `\![name,tokens...]` の汎用キャリアの cue を組む（正準形＝`Custom` の String Array）。
fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

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
                emit_one(carrier_cue(actor, name, &["balloonmove"])),
                vec![PositionCue::BalloonMoveLock { scope, locked }],
                "\\{actor} の \\![{name},balloonmove]"
            );
        }
    }
}

/// 第 1 引数が balloonmove でない lock・unlock は今どおり担当外で、何も送らない。
#[test]
fn lock_with_other_first_arg_is_not_mine() {
    for (name, tokens) in [
        ("lock", &["repaint"][..]),
        ("unlock", &["repaint"][..]),
        ("lock", &[][..]),
    ] {
        assert!(
            emit_one(carrier_cue("0", name, tokens)).is_empty(),
            "\\![{name},{tokens:?}] は送らない"
        );
    }
}

/// 余分な引数（`\![lock,balloonmove,foo]`）は読まずに受ける。
#[test]
fn extra_args_are_accepted_unread() {
    assert_eq!(
        emit_one(carrier_cue("1", "lock", &["balloonmove", "foo", "bar"])),
        vec![PositionCue::BalloonMoveLock {
            scope: 1,
            locked: true
        }]
    );
    assert_eq!(
        emit_one(carrier_cue("0", "unlock", &["balloonmove", "foo"])),
        vec![PositionCue::BalloonMoveLock {
            scope: 0,
            locked: false
        }]
    );
}

/// 実行者が数でない lock は捨てる（今の `actor_scope` の扱い）。
#[test]
fn non_numeric_actor_sends_nothing() {
    assert_eq!(
        emit_one(carrier_cue("x", "lock", &["balloonmove"])),
        vec![],
        "数でない実行者の lock は送らない"
    );
}
