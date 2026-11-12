//! 送る点の頭書きの守りの檻（要件 9.2）。
//!
//! `round_trip_request` を直接呼び、SHIORI へ出る唯一の点で CR・LF・NUL がどう扱われるかを固定する:
//! ID にあれば送らずに断る・Reference にあれば取り除いて送る。外から頼まれた名前の Reference は
//! 取り除く前の生の値で断る（`error_response_tests::sstp_origin_invalid_name_or_reference_is_not_sent_and_logs_error`）・
//! 翻訳の要求の添字 1 の欠番の印は壊さない（`actor_translate::tests::on_translate_receives_expanded_script_and_request_status`）。

use super::error_response_tests::round_trip_recording;
use super::tests::get;
use super::*;
use crate::schedule::log_capture::CapturedEvent;
use tracing::Level;

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
