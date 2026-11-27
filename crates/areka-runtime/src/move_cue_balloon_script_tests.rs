// MoveCueSink の set,balloonoffset・set,balloonalign の腕と受け手の表の 2 行。

use super::*;
use crate::consumer_ledger::test_support::actor_carrier_cue;
use crate::consumer_ledger::{CommandConsumer, ConsumerLedger};
use crate::strict_judges::ArekaStrictJudges;
use areka_placement::script_balloon::{OffsetAxis, ScriptAlign, ScriptOffset};
use areka_sakura::strict::StrictJudges;
use dola::cue::{CueSink, TalkCue};
use log_capture_kit::CapturedEvent;
use std::sync::mpsc::channel;

fn emit_one(cue: TalkCue) -> Vec<PositionCue> {
    let (tx, rx) = channel::<PositionCue>();
    MoveCueSink::new(tx).emit(cue);
    rx.try_iter().collect()
}

fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect()
}

#[test]
fn both_commands_belong_to_move_sink_and_check_script_handles_them() {
    let ledger = ConsumerLedger::canonical();
    let judges = ArekaStrictJudges::canonical();
    for selector in ["balloonoffset", "balloonalign"] {
        assert_eq!(
            ledger.consumer_of("set", Some(selector)),
            Some(CommandConsumer::MoveSink),
            "set,{selector}"
        );
        assert!(
            judges.command_handled("set", Some(selector)),
            "check_script が set,{selector} を受け手の無い命令と報告する"
        );
    }
}

#[test]
fn offset_and_align_are_sent_once_in_the_actor_scope() {
    for (actor, scope) in [("0", 0), ("1", 1)] {
        assert_eq!(
            emit_one(actor_carrier_cue(
                actor,
                "set",
                &["balloonoffset", "@-10", "200"]
            )),
            vec![PositionCue::SetBalloonOffset {
                scope,
                offset: ScriptOffset {
                    x: OffsetAxis::Relative(-10),
                    y: OffsetAxis::Absolute(200),
                },
            }],
            "\\{actor} の balloonoffset"
        );
        assert_eq!(
            emit_one(actor_carrier_cue(actor, "set", &["balloonalign", "right"])),
            vec![PositionCue::SetBalloonAlign {
                scope,
                align: ScriptAlign::Right,
            }],
            "\\{actor} の balloonalign"
        );
    }
}

#[test]
fn broken_offset_sends_nothing_and_warns_once_with_the_raw_text() {
    for args in [
        &["balloonoffset", "abc", "0"][..],
        &["balloonoffset", "10"][..],
    ] {
        let (sent, events) =
            log_capture_kit::capture(|| emit_one(actor_carrier_cue("0", "set", args)));
        assert_eq!(sent, vec![], "{args:?} は送らない");
        let warns = warns(&events);
        assert_eq!(warns.len(), 1, "{events:?}");
        assert_eq!(warns[0].field_str("event"), Some("script_balloon_rejected"));
        assert_eq!(warns[0].field_str("command"), Some("set,balloonoffset"));
        let raw = warns[0].field("raw").unwrap_or_default();
        assert!(raw.contains(args[1]), "受け取った字が無い: {raw}");
    }
}

#[test]
fn missing_align_id_sends_nothing_and_warns_once() {
    for args in [&["balloonalign"][..], &["balloonalign", "  "][..]] {
        let (sent, events) =
            log_capture_kit::capture(|| emit_one(actor_carrier_cue("1", "set", args)));
        assert_eq!(sent, vec![], "{args:?} は送らない");
        let warns = warns(&events);
        assert_eq!(warns.len(), 1, "{events:?}");
        assert_eq!(warns[0].field_str("event"), Some("script_balloon_rejected"));
        assert_eq!(warns[0].field_str("command"), Some("set,balloonalign"));
        assert_eq!(warns[0].field_str("reason"), Some("missing_id"));
    }
}

#[test]
fn extra_args_are_ignored() {
    assert_eq!(
        emit_one(actor_carrier_cue(
            "0",
            "set",
            &["balloonoffset", "5", "6", "7"]
        )),
        vec![PositionCue::SetBalloonOffset {
            scope: 0,
            offset: ScriptOffset {
                x: OffsetAxis::Absolute(5),
                y: OffsetAxis::Absolute(6),
            },
        }]
    );
    assert_eq!(
        emit_one(actor_carrier_cue(
            "0",
            "set",
            &["balloonalign", "LEFT", "foo"]
        )),
        vec![PositionCue::SetBalloonAlign {
            scope: 0,
            align: ScriptAlign::Left,
        }]
    );
}

#[test]
fn non_numeric_actor_sends_nothing_with_the_current_warning() {
    for (args, tag) in [
        (&["balloonoffset", "1", "2"][..], "set,balloonoffset"),
        (&["balloonalign", "left"][..], "set,balloonalign"),
    ] {
        let (sent, events) =
            log_capture_kit::capture(|| emit_one(actor_carrier_cue("x", "set", args)));
        assert_eq!(sent, vec![]);
        let warns = warns(&events);
        assert_eq!(warns.len(), 1, "{events:?}");
        assert_eq!(warns[0].field_str("event"), Some("move_cue_bad_actor"));
        assert_eq!(warns[0].field_str("tag"), Some(tag));
    }
}
