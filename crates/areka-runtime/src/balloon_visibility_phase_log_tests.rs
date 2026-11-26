// =============================================================================
// 判断中核が返した事象の記録の水準と欄
//
// 遷移・計測・時間切れの見送り・起点の採り方・番号の無い時間切れの知らせ
// （`balloon_timeout_notice_failed`）を、配線層がどの水準・どの欄で 1 行に写すかを固定する。
// =============================================================================

use tracing::Level;

use super::super::test_support::ms;
use super::super::{MeasurementDiscardReason, SuppressionKinds};
use super::test_support::visibility_lines;
use super::*;
use dola::TimeSpan;
use log_capture_kit::{CapturedEvent, ExpectField, capture};

/// 記録の時刻の欄を `FromStr` で読む。
fn span_field(event: &CapturedEvent, name: &str) -> TimeSpan {
    event
        .expect_field(name)
        .parse()
        .expect("時刻の欄は TimeSpan の書式")
}

/// 選択肢つきでないトークの起点の採り方 3 つは同じ欄の語で分かれ、止まった時刻を採ったときだけ起点の時刻の欄が載る。
/// 止まった時刻が届かなかったことは警告で残す。
#[test]
fn measurement_origin_is_written_with_its_level_and_fields() {
    let logs = vec![
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::StoppedAt(ms(4500)),
            display_end: ms(10000),
            deadline: ms(34500),
        },
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::DisplayEnd,
            display_end: ms(10000),
            deadline: ms(40000),
        },
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::StopTimeMissing,
            display_end: ms(10000),
            deadline: ms(40000),
        },
    ];

    let (_, events) = capture(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 3, "事象 1 件につき 1 行: {events:?}");

    let stopped = lines[0];
    assert_eq!(stopped.level, Level::INFO);
    assert_eq!(stopped.expect_field("origin"), "\"stopped_at\"");
    assert_eq!(
        span_field(stopped, "stopped_at"),
        ms(4500),
        "起点の時刻も載せる"
    );
    assert_eq!(span_field(stopped, "display_end"), ms(10000));
    assert_eq!(span_field(stopped, "deadline"), ms(34500));

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
    assert_eq!(span_field(missing, "display_end"), ms(10000));
    assert_eq!(span_field(missing, "deadline"), ms(40000));
    assert_eq!(missing.field("stopped_at"), None);
}

/// 選択肢つきのトークの起点 2 つ。止まった時刻を
/// 採ったら `origin=choice_wait_ended_at` と起点の時刻、時刻が届かなかったら警告で
/// `origin=choice_wait_end_time_missing`（起点はそのフレームの現在時刻＝満了予定から読める）。
#[test]
fn choice_talk_origins_are_written_with_their_level_and_fields() {
    let logs = vec![
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::ChoiceWaitEndedAt(ms(60000)),
            display_end: ms(2000),
            deadline: ms(90000),
        },
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::ChoiceWaitEndTimeMissing,
            display_end: ms(2000),
            deadline: ms(100000),
        },
    ];

    let (_, events) = capture(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 2, "事象 1 件につき 1 行: {events:?}");

    let ended = lines[0];
    assert_eq!(ended.level, Level::INFO);
    assert!(ended.message().contains("タイムアウト計測を開始"));
    assert_eq!(ended.expect_field("origin"), "\"choice_wait_ended_at\"");
    assert_eq!(span_field(ended, "stopped_at"), ms(60000));
    assert_eq!(span_field(ended, "display_end"), ms(2000));
    assert_eq!(span_field(ended, "deadline"), ms(90000));

    let missing = lines[1];
    assert_eq!(
        missing.level,
        Level::WARN,
        "止まった時刻が届かなかったのは縮退（フレームの現在時刻を起点にして表示を保持する）"
    );
    assert_eq!(
        missing.expect_field("origin"),
        "\"choice_wait_end_time_missing\""
    );
    assert_eq!(span_field(missing, "deadline"), ms(100000));
    assert_eq!(missing.field("stopped_at"), None);
}

/// 時間切れで隠したのにトークの番号が無ければ、知らせを送れなかった理由を警告で 1 行残す。
#[test]
fn timeout_notice_without_talk_id_is_written_as_a_warning() {
    let logs = vec![VisibilityLogEvent::TimeoutNoticeWithoutTalkId];

    let (_, events) = capture(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 1, "事象 1 件につき 1 行: {events:?}");
    assert_eq!(lines[0].level, Level::WARN);
    assert_eq!(
        lines[0].expect_field("event"),
        "\"balloon_timeout_notice_failed\""
    );
    assert_eq!(lines[0].expect_field("reason"), "\"no_talk_id\"");
}

/// 判断が返した事象は水準と語彙で 1 行ずつ出る。
#[test]
fn decision_events_are_written_with_the_agreed_level_and_fields() {
    let logs = vec![
        VisibilityLogEvent::Transition {
            scope: 1,
            trigger: VisibilityTrigger::Content,
            visible: true,
        },
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Clear,
            visible: false,
        },
        VisibilityLogEvent::MeasurementStarted {
            origin: MeasurementOrigin::DisplayEnd,
            display_end: ms(4000),
            deadline: ms(34000),
        },
        VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::TalkStarted,
            deadline: ms(34000),
        },
        VisibilityLogEvent::MeasurementRestarted {
            now: ms(40000),
            deadline: ms(70000),
        },
        VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                dragging: false,
                hover: true,
            },
            deadline: ms(34000),
        },
        VisibilityLogEvent::DisplayEndSignalMissing,
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Timeout,
            visible: false,
        },
        VisibilityLogEvent::Transition {
            scope: 1,
            trigger: VisibilityTrigger::UserBreak,
            visible: false,
        },
        VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::DisplayEndAdvanced,
            deadline: ms(34000),
        },
        VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::NoVisibleScope,
            deadline: ms(34000),
        },
    ];

    let (_, events) = capture(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 11, "事象 1 件につき 1 行: {events:?}");

    assert_eq!(lines[0].level, Level::INFO);
    assert_eq!(lines[0].expect_field("scope"), "1");
    assert_eq!(lines[0].expect_field("trigger"), "\"content\"");
    assert_eq!(lines[0].expect_field("visible"), "true");

    assert_eq!(lines[1].expect_field("trigger"), "\"clear\"");
    assert_eq!(lines[1].expect_field("visible"), "false");

    assert_eq!(lines[2].level, Level::INFO);
    assert_eq!(span_field(lines[2], "display_end"), ms(4000));
    assert_eq!(span_field(lines[2], "deadline"), ms(34000));

    assert_eq!(lines[3].expect_field("reason"), "\"talk_started\"");

    assert_eq!(span_field(lines[4], "talk_time"), ms(40000));
    assert_eq!(span_field(lines[4], "deadline"), ms(70000));

    assert_eq!(lines[5].level, Level::INFO);
    assert_eq!(lines[5].expect_field("hover"), "true");
    assert_eq!(lines[5].expect_field("dragging"), "false");

    assert_eq!(
        lines[6].level,
        Level::WARN,
        "表示終了信号の欠落は縮退（表示は保持する）"
    );

    assert_eq!(lines[7].expect_field("trigger"), "\"timeout\"");
    assert_eq!(lines[8].expect_field("trigger"), "\"user_break\"");
    assert_eq!(lines[9].expect_field("reason"), "\"display_end_advanced\"");
    assert_eq!(lines[10].expect_field("reason"), "\"no_visible_scope\"");
}

/// 発行が実らなかった scope の「表示へ遷移した」は名乗らない（起きていない遷移を記録しない）。
#[test]
fn transition_of_a_show_that_did_not_take_is_not_written() {
    let logs = vec![
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Content,
            visible: true,
        },
        VisibilityLogEvent::Transition {
            scope: 1,
            trigger: VisibilityTrigger::Content,
            visible: true,
        },
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Clear,
            visible: false,
        },
    ];

    let (_, events) = capture(|| emit_visibility_logs(&logs, &[0]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 2, "scope 0 の表示だけが落ちる: {events:?}");
    assert_eq!(lines[0].expect_field("scope"), "1");
    assert_eq!(lines[1].expect_field("scope"), "0");
    assert_eq!(
        lines[1].expect_field("visible"),
        "false",
        "非表示の遷移は実際に起きるので落とさない"
    );
}
