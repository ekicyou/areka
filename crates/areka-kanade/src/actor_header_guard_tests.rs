//! 送る点の頭書きの守りの檻（audit-kanade #1 の二段目・要件 9.2）。
//!
//! `round_trip_request` を直接呼び、SHIORI へ出る唯一の点で CR・LF・NUL がどう扱われるかを固定する:
//! ID にあれば送らずに断る・Reference にあれば取り除いて送る・外から頼まれた名前の Reference は
//! 今どおり断る（判定は取り除く前の生の値で行う）・翻訳の要求の添字 1 の欠番の印は壊さない。

use super::error_response_tests::round_trip_recording;
use super::*;
use crate::request_origin::RequestOrigin;
use crate::schedule::log_capture::CapturedEvent;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use tracing::Level;

fn get(id: EventId, references: &[&str]) -> ShioriCall {
    ShioriCall::Get {
        id,
        references: references.iter().map(|r| r.to_string()).collect(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

fn records(events: &[CapturedEvent], level: Level, event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.target == "kanade" && e.level == level && e.event.as_deref() == Some(event))
        .count()
}

#[test]
fn a_reference_with_line_breaks_is_sent_without_them() {
    let (outcome, sent, events) =
        round_trip_recording(get(EventId::Static("OnBoot"), &["a\r\nb\0c", "plain"]));
    assert!(matches!(outcome, ShioriOutcome::NoContent), "送られるはず");
    assert_eq!(
        sent,
        Some((
            "OnBoot".to_string(),
            vec!["abc".to_string(), "plain".to_string()]
        )),
        "Ref0 は 3 字を取り除いて送り、ほかは逐語"
    );
    assert_eq!(
        records(&events, Level::WARN, "reference_line_break_removed"),
        1,
        "取り除いた位置ごとに 1 件"
    );
}

#[test]
fn an_event_id_with_a_line_break_is_refused_before_the_wire() {
    for name in ["OnX\nY", "OnX\rY", "OnX\0Y"] {
        let (outcome, sent, events) =
            round_trip_recording(get(EventId::Choice(name.to_string()), &[]));
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Internal(_))),
            "{name:?} は Failed(Internal) のはず"
        );
        assert_eq!(sent, None, "{name:?} は送られてはならない");
        assert_eq!(records(&events, Level::ERROR, "event_id_not_allowed"), 1);
    }
}

#[test]
fn an_sstp_reference_with_a_line_break_is_still_refused_at_the_wire() {
    let (outcome, sent, events) =
        round_trip_recording(get(EventId::Sstp("OnX".to_string()), &["a\nb"]));
    assert!(
        matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Internal(_))),
        "外から頼まれた名前の Reference の改行は今どおり断る"
    );
    assert_eq!(sent, None, "取り除いて送ってはならない");
    assert_eq!(records(&events, Level::ERROR, "event_id_not_allowed"), 1);
    assert_eq!(
        records(&events, Level::WARN, "reference_line_break_removed"),
        0
    );
}

#[test]
fn the_translate_absent_marker_survives_the_guard() {
    let (outcome, sent, events) = round_trip_recording(get(
        EventId::Translate(RequestOrigin::MCP_SEND_TRANSLATE),
        &["script", ABSENT_REFERENCE],
    ));
    assert!(matches!(outcome, ShioriOutcome::NoContent), "送られるはず");
    let (_, references) = sent.expect("送られるはず");
    assert_eq!(references[1], ABSENT_REFERENCE, "添字 1 は欠番の印のまま");
    assert!(
        !events.iter().any(|e| e.level == Level::WARN),
        "二段目は印を壊さず WARN も出さない"
    );
}
