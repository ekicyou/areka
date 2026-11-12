//! 再生中のトークが無いときのダブルクリック（表 C）の檻（areka-P0-balloon-lifecycle-events）。
//!
//! 最上位の `step` に中断の合図（`Input::UserBreak`）を入れて駆動する。定常でない相・終了の保留は
//! 本番の流れで作ると長くなるので、トークを 1 つ再生し終えた控えを持つ状態の相だけを差し替える。

use super::super::log_capture::{assert_not_logged, logged_once};
use super::super::{Input, Phase, TermCause};
use super::done_tests::{assert_no_balloon_event, assert_not_sent, feed_capturing, field, gets};
use super::tests::{FINAL, steady_after_talk, steady_none};
use crate::msg::CloseReason;
use crate::schedule::Action;
use tracing::Level;

/// C4: 再生中のトークが無い定常のダブルクリックで、`OnBalloonClose`（Reference0＝直前に終えた
/// トークの台本）を 1 本送る。何も止めず、中断の控えも立てない（要件 3.1・3.2・7.1）。
/// C3: 同じトークのバルーンへの 2 回目は送らず、理由を記録する（要件 4.5）。
#[test]
fn c4_idle_double_click_sends_balloon_close_then_c3_refuses_the_second() {
    let (mut s, talk) = steady_after_talk();
    let (actions, ev) = feed_capturing(
        &mut s,
        Input::UserBreak {
            scope: 1,
            talk_id: Some(talk),
        },
    );
    assert_eq!(
        gets(&actions),
        vec![("OnBalloonClose", &[FINAL.to_string()][..])],
        "指示は OnBalloonClose の GET 1 本だけ（止める指示は無い）"
    );
    assert_eq!(actions.len(), 1);
    assert!(s.shown.as_ref().is_some_and(|shown| shown.close_sent));
    assert!(s.user_break_talk.is_none(), "中断の控えは立てない");
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    let line = logged_once(&ev, Level::INFO, "balloon_event_sent");
    assert_eq!(
        (
            field(line, "id"),
            field(line, "cause"),
            field(line, "talk_id")
        ),
        (
            Some("OnBalloonClose"),
            Some("close"),
            Some(talk.0.to_string().as_str())
        ),
        "イベント名・きっかけ・トークの番号を 1 行で記録する（要件 7.1）。\n捕捉={ev:#?}"
    );
    assert_not_logged(&ev, "balloon_event_not_sent");

    let (actions, ev) = feed_capturing(
        &mut s,
        Input::UserBreak {
            scope: 1,
            talk_id: Some(talk),
        },
    );
    assert!(actions.is_empty(), "2 回目は何も送らない");
    assert_not_sent(&ev, "OnBalloonClose", "already_sent", "Steady");
}

/// C4 の守り: 控えが無ければ Reference0 を空で送り、警告を残す（設計 D9）。
#[test]
fn c4_without_shown_talk_sends_empty_script_with_a_warning() {
    let mut s = steady_none();
    let (actions, ev) = feed_capturing(
        &mut s,
        Input::UserBreak {
            scope: 0,
            talk_id: None,
        },
    );
    assert_eq!(
        gets(&actions),
        vec![("OnBalloonClose", &[String::new()][..])]
    );
    let line = logged_once(&ev, Level::WARN, "balloon_event_script_missing");
    assert_eq!(field(line, "id"), Some("OnBalloonClose"), "捕捉={ev:#?}");
}

/// C1: 相が定常でない（起動の途中・終了の握手の待ち・切替の途中・降ろす途中）なら送らず、理由と
/// 相を記録する。控えの印も立てない（要件 3.4・4.4・7.2）。
#[test]
fn c1_not_steady_phases_send_nothing_and_record_why() {
    let phases = [
        ("BootVersion", Phase::BootVersion { talk: None }),
        ("ClosePending", Phase::ClosePending),
        ("ChangePending", Phase::ChangePending),
        ("ChangeClosePending", Phase::ChangeClosePending),
        (
            "Unloading",
            Phase::Unloading {
                cause: TermCause::Quit,
            },
        ),
        ("Stopped", Phase::Stopped),
    ];
    for (label, phase) in phases {
        let (mut s, _) = steady_after_talk();
        s.phase = phase;
        let (actions, ev) = feed_capturing(
            &mut s,
            Input::UserBreak {
                scope: 0,
                talk_id: None,
            },
        );
        assert!(actions.is_empty(), "{label}: 何も送らない");
        assert_not_sent(&ev, "OnBalloonClose", "not_steady", label);
        assert!(
            s.shown.as_ref().is_some_and(|shown| !shown.close_sent),
            "{label}: 送り済みの印を立てない"
        );
    }
}

/// C2: 終了の要求を保留している間は送らず、理由を記録する（要件 3.4）。
#[test]
fn c2_close_pending_sends_nothing_and_records_why() {
    let (mut s, _) = steady_after_talk();
    s.pending_close = Some(CloseReason::System);
    let (actions, ev) = feed_capturing(
        &mut s,
        Input::UserBreak {
            scope: 0,
            talk_id: None,
        },
    );
    assert_no_balloon_event(&actions, "終了の保留中");
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::CancelChoice { .. }))
    );
    assert_not_sent(&ev, "OnBalloonClose", "close_pending", "Steady");
    assert!(s.shown.as_ref().is_some_and(|shown| !shown.close_sent));
}
