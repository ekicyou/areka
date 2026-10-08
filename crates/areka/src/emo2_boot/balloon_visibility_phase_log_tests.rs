// =============================================================================
// トークの終わりから計る時間切れの記録の水準と欄（areka-P0-balloon-lifecycle-events task 4.3）
//
// 判断中核が返した起点の採り方（3 つの値）と、番号の無い時間切れの知らせを、配線層が
// どの水準・どの欄で 1 行に写すかを固定する（要件 5.6・7.3・design の Error Strategy の
// `balloon_timeout_notice_failed`）。既定の起点（占有区間の終端）の行の水準と欄は
// `balloon_visibility_phase_tests.rs` の
// `decision_events_are_written_with_the_agreed_level_and_fields` が持つ。
// =============================================================================

use tracing::Level;

use super::*;
use crate::placement::test_support::{ExpectField, LogEvent, capture_logs};

/// 可視性の相が書いた行だけを拾う。
fn visibility_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains("[balloon-visibility]"))
        .collect()
}

/// 起点の採り方 3 つは同じ欄の語で分かれ、止まった時刻を採ったときだけ起点の時刻の欄が載る。
/// 止まった時刻が届かなかったことは警告で残す（要件 5.6・7.3）。
#[test]
fn measurement_origin_is_written_with_its_level_and_fields() {
    let logs = vec![
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::StoppedAt(4.5),
            display_end: 10.0,
            deadline: 34.5,
        },
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::DisplayEnd,
            display_end: 10.0,
            deadline: 40.0,
        },
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::StopTimeMissing,
            display_end: 10.0,
            deadline: 40.0,
        },
    ];

    let (_, events) = capture_logs(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 3, "事象 1 件につき 1 行: {events:?}");

    let stopped = lines[0];
    assert_eq!(stopped.level, Level::INFO);
    assert_eq!(stopped.expect_field("origin"), "\"stopped_at\"");
    assert_eq!(
        stopped.expect_field("stopped_at"),
        "4.5",
        "起点の時刻も載せる"
    );
    assert_eq!(stopped.expect_field("display_end"), "10.0");
    assert_eq!(stopped.expect_field("deadline"), "34.5");

    let display_end = lines[1];
    assert_eq!(display_end.level, Level::INFO);
    assert_eq!(display_end.expect_field("origin"), "\"display_end\"");
    assert_eq!(display_end.field("stopped_at"), None);

    let missing = lines[2];
    assert_eq!(
        missing.level,
        Level::WARN,
        "止まった時刻が届かなかったのは縮退（終端を起点にして表示を保持する）"
    );
    assert_eq!(missing.expect_field("origin"), "\"stop_time_missing\"");
    assert_eq!(missing.expect_field("display_end"), "10.0");
    assert_eq!(missing.expect_field("deadline"), "40.0");
    assert_eq!(missing.field("stopped_at"), None);
}

/// 時間切れで隠したのにトークの番号が無ければ、知らせを送れなかった理由を警告で 1 行残す
/// （design の Error Strategy「知らせを送れない」）。
#[test]
fn timeout_notice_without_talk_id_is_written_as_a_warning() {
    let logs = vec![VisibilityLogEvent::TimeoutNoticeWithoutTalkId];

    let (_, events) = capture_logs(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 1, "事象 1 件につき 1 行: {events:?}");
    assert_eq!(lines[0].level, Level::WARN);
    assert_eq!(
        lines[0].expect_field("event"),
        "\"balloon_timeout_notice_failed\""
    );
    assert_eq!(lines[0].expect_field("reason"), "\"no_talk_id\"");
}
