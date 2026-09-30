//! 台詞の切れ目の口のテスト（areka-P0-shell-balloon-switch 要件 1.14・2.3・3.1・5.4・5.7・8.11）。
//!
//! 印の無い依頼（`Input::AwaitTalkGap { raise: None }`）を `step` 経由で駆動し、始め方（定常でない・
//! 終了の保留あり → 定常でない／それ以外 → 見張り）と、毎 `step` の後の見極めの結果
//! （達した・終了で達しない・ゴースト切替で達しない）が 1 度決まったら変わらないことを固定する。
//! 許可表に無い印は送らずに「送らなかった」になることも見る。

use super::*;
use crate::change::{
    ChangeOrigin, ChangeRequest, ChangeTarget, GapLeft, RaiseOutcome, ShioriMethod,
};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs};
use crate::schedule::log_capture::{capture, logged_once};
use crate::schedule::steady::test_support::base_state;
use crate::schedule::{ActiveTalk, Input, Phase, TermCause, step};
use crate::talk::{TalkDone, TalkEndReason, TalkId};
use tracing::Level;

/// 再生中のトークの番号（`steady_some` に渡す）。
const TALK: TalkId = TalkId(4);

fn cfg() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// `Steady{talk: None}`（終了の保留なし）。
fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..base_state()
    }
}

/// `Steady{talk: Some(TALK)}`（終了の保留なし）。
fn steady_some() -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id: TALK,
                origin: "OnSecondChange",
                script: String::new(),
            }),
        },
        ..steady_none()
    }
}

/// 印の無い依頼を 1 件 `step` へ渡す。
fn await_gap(s: State) -> (State, Vec<Action>) {
    step(s, Input::AwaitTalkGap { raise: None }, &cfg())
}

fn talk_done(s: State, reason: TalkEndReason, quit_reserved: bool) -> State {
    let done = TalkDone {
        talk_id: TALK,
        reason,
        quit_reserved,
    };
    step(s, Input::TalkDone(done), &cfg()).0
}

/// 見張りが在り、結果がまだ決まっていないこと。
fn assert_waiting(s: &State, label: &str) {
    let watch = s.talk_gap.as_ref().expect("見張りが在る");
    assert!(watch.outcome.is_none(), "{label}: まだ決まっていない");
}

fn change_request() -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: r"C:\areka\ghost\r_post".to_string(),
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    }
}

#[test]
fn unmarked_request_in_idle_steady_reaches_at_once() {
    let (mut s, actions) = await_gap(steady_none());
    assert!(actions.is_empty(), "印が無ければ SHIORI へ何も送らない");
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached { marked: None }),
        "Steady{{talk: None}} なら即「達した」"
    );
    assert!(s.talk_gap.is_none(), "取り出したら見張りは終わる");
    assert_eq!(take_outcome(&mut s), None, "結果は 1 回だけ取り出せる");
}

#[test]
fn unmarked_request_while_playing_reaches_at_the_end_of_the_talk() {
    let (mut s, actions) = await_gap(steady_some());
    assert!(actions.is_empty());
    assert_waiting(&s, "再生中");
    assert_eq!(take_outcome(&mut s), None, "決まる前は何も取り出せない");
    assert!(s.talk_gap.is_some(), "決まる前に取り出しても見張りは続く");

    // 再生中の Tick（pump は再生中でも回る）では決まらない。
    let s = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_500),
        },
        &cfg(),
    )
    .0;
    assert_waiting(&s, "Tick の後");

    let mut s = talk_done(s, TalkEndReason::Ended, false);
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached { marked: None })
    );
}

#[test]
fn user_break_without_quit_reservation_reaches() {
    let (s, _) = await_gap(steady_some());
    let (s, actions) = step(s, Input::UserBreak { scope: 0 }, &cfg());
    assert!(
        matches!(actions.as_slice(), [Action::CancelChoice { talk_id }] if *talk_id == TALK),
        "中断は今日どおり止める指示を 1 件出す"
    );
    assert_waiting(&s, "止めた応答を待つ間");

    let mut s = talk_done(s, TalkEndReason::Interrupted, false);
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached { marked: None }),
        "`\\-` の予約の無い中断の後は「達した」（要件 5.4）"
    );
}

#[test]
fn user_break_with_quit_reservation_goes_to_closing_as_today() {
    let (s, _) = await_gap(steady_some());
    let (s, _) = step(s, Input::UserBreak { scope: 0 }, &cfg());
    let done = TalkDone {
        talk_id: TALK,
        reason: TalkEndReason::Interrupted,
        quit_reserved: true,
    };
    let (mut s, actions) = step(s, Input::TalkDone(done), &cfg());
    assert!(
        matches!(
            s.phase,
            Phase::Unloading {
                cause: TermCause::Quit
            }
        ),
        "今日どおり終了系列へ進む"
    );
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::Closing
        })
    );
}

#[test]
fn talk_reaching_quit_by_itself_is_closing() {
    let (s, _) = await_gap(steady_some());
    let mut s = talk_done(s, TalkEndReason::Quit, false);
    assert!(matches!(s.phase, Phase::Unloading { .. }));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::Closing
        })
    );
}

#[test]
fn close_request_while_watching_is_closing() {
    let (s, _) = await_gap(steady_some());
    let (mut s, _) = step(
        s,
        Input::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
        &cfg(),
    );
    assert!(
        s.pending_close.is_some(),
        "再生中の閉鎖要求は今日どおり保留される"
    );
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::Closing
        }),
        "保留が立った時点で「終了で達しない」（要件 5.7）"
    );
}

#[test]
fn pending_ghost_change_while_playing_is_ghost_change() {
    let (s, _) = await_gap(steady_some());
    let (mut s, actions) = step(s, Input::ChangeGhost(change_request()), &cfg());
    assert!(actions.is_empty() && s.pending_change.is_some());
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::GhostChange
        })
    );
}

#[test]
fn decided_outcome_is_not_changed_by_later_steps() {
    // ゴースト切替の保留で決まった後、トークが終わって切替の相へ進んでも結果は変わらない。
    let (s, _) = await_gap(steady_some());
    let (s, _) = step(s, Input::ChangeGhost(change_request()), &cfg());
    let (s, _) = step(
        s,
        Input::CloseRequest {
            reason: CloseReason::System,
        },
        &cfg(),
    );
    let mut s = talk_done(s, TalkEndReason::Ended, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::GhostChange
        }),
        "最初に決まった結果のまま"
    );
}

#[test]
fn request_outside_steady_or_with_pending_close_is_not_steady() {
    let cases: Vec<(&str, State)> = vec![
        (
            "BootMain",
            State {
                phase: Phase::BootMain,
                ..steady_none()
            },
        ),
        (
            "BootVersion",
            State {
                phase: Phase::BootVersion { talk: None },
                ..steady_none()
            },
        ),
        (
            "ClosePending",
            State {
                phase: Phase::ClosePending {
                    reason: CloseReason::System,
                },
                ..steady_none()
            },
        ),
        (
            "ChangePending",
            State {
                phase: Phase::ChangePending,
                ..steady_none()
            },
        ),
        (
            "Unloading",
            State {
                phase: Phase::Unloading {
                    cause: TermCause::Quit,
                },
                ..steady_none()
            },
        ),
        (
            "Steady の終了の保留あり",
            State {
                pending_close: Some(CloseReason::System),
                ..steady_some()
            },
        ),
    ];
    for (label, s) in cases {
        let mut out = None;
        let ev = capture(|| out = Some(await_gap(s)));
        let (mut s, actions) = out.expect("step は必ず結果を返す");
        assert!(actions.is_empty(), "{label}: 何も送らない");
        assert_eq!(
            take_outcome(&mut s),
            Some(TalkGap::Left {
                reason: GapLeft::NotSteady
            }),
            "{label}: 届いたときに定常でない（要件 1.14）"
        );
        let left = logged_once(&ev, Level::INFO, "talk_gap_left");
        assert_eq!(
            left.fields.get("reason").map(String::as_str),
            Some("not_steady"),
            "{label}"
        );
    }
}

#[test]
fn marked_request_outside_the_table_is_not_sent() {
    let raise = GapRaise {
        id: "OnTalk".to_string(),
        references: vec!["b".to_string()],
        method: ShioriMethod::Get,
    };
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(
            steady_none(),
            Input::AwaitTalkGap { raise: Some(raise) },
            &cfg(),
        ))
    });
    let (mut s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::WARN, "raise_event_not_allowed");
    assert!(actions.is_empty(), "許可表に無い印は送らない");
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::NotSent {
            outcome: RaiseOutcome::NotAllowed
        })
    );
}

#[test]
fn reached_is_logged_once_at_info() {
    let mut out = None;
    let ev = capture(|| out = Some(await_gap(steady_none())));
    logged_once(&ev, Level::INFO, "talk_gap_reached");
}
