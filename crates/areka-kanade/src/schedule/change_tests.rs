//! 切替の相のテスト（areka-P0-ghost-shell-balloon-switch 要件 2.1〜2.6・5.5・7.5・10.2・10.3・11.6）。
//!
//! 受理と `OnGhostChanging` の Reference・受理しない場面・送り出しの握手（台本／204 → `OnClose`）と、
//! 台詞が最後まで流れる・`\-` で終わる・期限を超える、のいずれでも降ろす相へ進み、切替の帳簿
//! （停止通知の切替の中身の源）に `OnGhostChanging` の台本が残ることを `step` 経由で固定する。
//! 再生中に届いた要求の保留（最後まで・中断で消化／終了の保留が勝つ／`\-` の予約・到達で捨てる）も固定する。

use super::*;
use crate::change::{ChangeOrigin, ChangeTarget};
use crate::msg::{CloseReason, ShioriCall};
use crate::schedule::log_capture::{capture, logged_once};
use crate::schedule::steady::test_support::base_state;
use crate::schedule::{ActiveTalk, step};
use crate::talk::{TalkDone, TalkEndReason};
use tracing::Level;

const D: u64 = 30_000;

fn cfg() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn req(origin: ChangeOrigin, raise_event: bool) -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: r"C:\areka\ghost\r_post".to_string(),
        },
        origin,
        raise_event,
    }
}

fn steady() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..base_state()
    }
}

fn reply(s: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    step(
        s,
        Input::ShioriReply {
            outcome,
            origin: "test",
        },
        &cfg(),
    )
}

fn talk_done(s: State, talk_id: TalkId, reason: TalkEndReason) -> (State, Vec<Action>) {
    let done = TalkDone {
        talk_id,
        reason,
        quit_reserved: false,
    };
    step(s, Input::TalkDone(done), &cfg())
}

/// GET を 1 件だけ返したことを確かめ、(イベント名, Reference 列) を取り出す。
fn only_get(actions: &[Action]) -> (String, Vec<String>) {
    match actions {
        [Action::ShioriRequest(ShioriCall::Get { id, references, .. })] => {
            (id.as_str().to_string(), references.clone())
        }
        _ => panic!("GET がちょうど 1 件のはず"),
    }
}

fn script_of(s: &State) -> Option<&str> {
    s.change
        .as_ref()
        .expect("切替の帳簿が在るはず")
        .script
        .as_deref()
}

/// 受理して `OnGhostChanging` に台本で答えた直後（`ChangeTalkWait`）。
fn talking(script: &str) -> (State, TalkId) {
    let (s, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let (s, actions) = reply(s, ShioriOutcome::Value(script.to_string()));
    let Phase::ChangeTalkWait { talk_id, deadline } = s.phase else {
        panic!("OnGhostChanging の台本で ChangeTalkWait へ進むはず");
    };
    assert_eq!(talk_id, TalkId(5));
    assert_eq!(
        deadline,
        Some(MonotonicMs(1_000 + D)),
        "期限は終了の握手と同じ上限"
    );
    assert!(
        matches!(actions.as_slice(), [Action::StartTalk(t)] if t.talk_id == talk_id && t.script == script),
        "台本をそのまま再生する"
    );
    (s, talk_id)
}

/// 受理して `OnGhostChanging` が 204、続く `OnClose` に台本で答えた直後（`ChangeCloseTalkWait`）。
fn close_talking() -> (State, TalkId) {
    let (s, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let (s, actions) = reply(s, ShioriOutcome::NoContent);
    assert!(matches!(s.phase, Phase::ChangeClosePending));
    assert_eq!(
        only_get(&actions),
        ("OnClose".to_string(), vec!["system".to_string()]),
        "204 なら続けて OnClose（Ref0＝system）を GET"
    );
    let (s, actions) = reply(s, ShioriOutcome::Value("bye".to_string()));
    let Phase::ChangeCloseTalkWait { talk_id, deadline } = s.phase else {
        panic!("OnClose の台本で ChangeCloseTalkWait へ進むはず");
    };
    assert_eq!(deadline, Some(MonotonicMs(1_000 + D)));
    assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    assert_eq!(script_of(&s), None, "OnGhostChanging が 204 なら台本は無し");
    (s, talk_id)
}

fn assert_unloading(s: &State, actions: &[Action], expect: &str) {
    let cause = match &s.phase {
        Phase::Unloading {
            cause: TermCause::Quit,
        } => "Quit",
        Phase::Unloading {
            cause: TermCause::DeadlineExceeded,
        } => "DeadlineExceeded",
        Phase::Unloading {
            cause: TermCause::CloseSilent,
        } => "CloseSilent",
        _ => "other",
    };
    assert_eq!(cause, expect, "降ろす相へ進む");
    assert!(
        matches!(actions, [Action::ShioriUnload]),
        "降ろす要求だけが出る（終了の通知・追加のイベントは出ない）"
    );
}

#[test]
fn accept_sends_on_ghost_changing_with_refs_for_both_origins() {
    for (origin, word) in [
        (ChangeOrigin::Manual, "manual"),
        (ChangeOrigin::Automatic, "automatic"),
    ] {
        let (s, actions) = step(steady(), Input::ChangeGhost(req(origin, true)), &cfg());
        assert!(matches!(s.phase, Phase::ChangePending));
        assert_eq!(
            only_get(&actions),
            (
                "OnGhostChanging".to_string(),
                vec![
                    "ポスト".to_string(),
                    word.to_string(),
                    "R_POST_and_KOMAINU".to_string(),
                    r"C:\areka\ghost\r_post".to_string(),
                ]
            )
        );
        let change = s.change.as_ref().expect("受理で帳簿が立つ");
        assert_eq!(change.req, req(origin, true));
        assert_eq!(change.script, None);
    }
}

#[test]
fn reject_outside_idle_steady_warns_notifies_and_keeps_state() {
    let mut pending_close = steady();
    pending_close.pending_close = Some(CloseReason::System);
    let (accepted, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let cases = [
        State {
            phase: Phase::BootMain,
            ..steady()
        },
        pending_close,
        State {
            phase: Phase::ClosePending {
                reason: CloseReason::System,
            },
            ..steady()
        },
        accepted,
    ];
    for s in cases {
        let label = phase_label(&s.phase);
        let had_change = s.change.is_some();
        let had_pending_close = s.pending_close.is_some();
        let mut out = None;
        let ev = capture(|| {
            out = Some(step(
                s,
                Input::ChangeGhost(req(ChangeOrigin::Automatic, true)),
                &cfg(),
            ))
        });
        let (next, actions) = out.expect("step は必ず結果を返す");
        logged_once(&ev, Level::WARN, "change_rejected_phase");
        assert!(
            matches!(
                actions.as_slice(),
                [Action::Notice(KanadeNotice::ChangeCancelled {
                    reason: CancelReason::Rejected
                })]
            ),
            "{label}: 受理しなかったことを通知だけで返す"
        );
        assert_eq!(phase_label(&next.phase), label, "相は変わらない");
        assert_eq!(
            next.change.is_some(),
            had_change,
            "{label}: 帳簿は変わらない"
        );
        assert_eq!(next.pending_close.is_some(), had_pending_close);
        assert!(next.pending_change.is_none());
        assert_eq!(next.next_talk_id, 5, "{label}: 採番は進まない");
    }
}

#[test]
fn changing_script_played_to_end_unloads_with_script_in_ledger() {
    let (s, talk_id) = talking(r"\0またね\e");
    let (s, actions) = talk_done(s, talk_id, TalkEndReason::Ended);
    assert_unloading(&s, &actions, "Quit");
    assert_eq!(script_of(&s), Some(r"\0またね\e"));
    // 降ろし終えて止まっても帳簿は残る（停止通知の切替の中身の源）。
    let (s, actions) = reply(s, ShioriOutcome::Unloaded);
    assert!(matches!(s.phase, Phase::Stopped));
    assert!(matches!(actions.as_slice(), [Action::StopSelf]));
    assert_eq!(script_of(&s), Some(r"\0またね\e"));
}

#[test]
fn changing_script_ending_with_quit_tag_unloads() {
    let (s, talk_id) = talking(r"\0またね\-");
    let (s, actions) = talk_done(s, talk_id, TalkEndReason::Quit);
    assert_unloading(&s, &actions, "Quit");
    assert_eq!(script_of(&s), Some(r"\0またね\-"));
}

#[test]
fn changing_script_past_deadline_unloads() {
    let (s, _) = talking("long");
    let (s, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_000 + D - 1),
        },
        &cfg(),
    );
    assert!(matches!(s.phase, Phase::ChangeTalkWait { .. }) && actions.is_empty());
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(
            s,
            Input::Tick {
                now: MonotonicMs(1_000 + D),
            },
            &cfg(),
        ))
    });
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::ERROR, "change_deadline_exceeded");
    assert_unloading(&s, &actions, "DeadlineExceeded");
    assert_eq!(script_of(&s), Some("long"));
}

#[test]
fn farewell_after_204_unloads_on_end_quit_and_deadline() {
    let (s, talk_id) = close_talking();
    let (s, actions) = talk_done(s, talk_id, TalkEndReason::Ended);
    assert_unloading(&s, &actions, "Quit");

    let (s, talk_id) = close_talking();
    let (s, actions) = talk_done(s, talk_id, TalkEndReason::Quit);
    assert_unloading(&s, &actions, "Quit");

    let (s, _) = close_talking();
    let (s, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_000 + D),
        },
        &cfg(),
    );
    assert_unloading(&s, &actions, "DeadlineExceeded");
    assert!(s.change.is_some(), "切替の帳簿は降ろす相まで残る");
}

#[test]
fn both_204_unloads_silently() {
    let (s, _) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let (s, _) = reply(s, ShioriOutcome::NoContent);
    let (s, actions) = reply(s, ShioriOutcome::NoContent);
    assert_unloading(&s, &actions, "CloseSilent");
    assert_eq!(script_of(&s), None);
}

#[test]
fn deadline_is_armed_on_first_tick_when_entered_without_time() {
    let (s, _) = step(
        State {
            last_now: None,
            ..steady()
        },
        Input::ChangeGhost(req(ChangeOrigin::Manual, true)),
        &cfg(),
    );
    let (s, _) = reply(s, ShioriOutcome::Value("hi".to_string()));
    assert!(matches!(
        s.phase,
        Phase::ChangeTalkWait { deadline: None, .. }
    ));
    let (s, _) = step(
        s,
        Input::Tick {
            now: MonotonicMs(500),
        },
        &cfg(),
    );
    assert!(matches!(
        s.phase,
        Phase::ChangeTalkWait { deadline: Some(d), .. } if d == MonotonicMs(500 + D)
    ));
}

#[test]
fn without_raise_event_unloads_silently_without_events() {
    let (s, actions) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Automatic, false)),
        &cfg(),
    );
    assert_unloading(&s, &actions, "CloseSilent");
    assert_eq!(
        script_of(&s),
        None,
        "送らなかったので台本は無し・帳簿は立つ"
    );
}

// ---- 再生中に届いた切替の保留（要件 2.6・5.5・10.3・11.6） ----

/// 定常でトーク（talk_id 4）を再生している状態。
fn playing() -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id: TalkId(4),
                origin: "OnSecondChange",
                script: "x".to_string(),
            }),
        },
        ..steady()
    }
}

/// 再生中に切替の要求を送り、保留されたことを確かめて返す。
fn held(raise_event: bool) -> State {
    let r = req(ChangeOrigin::Manual, raise_event);
    let mut out = None;
    let ev = capture(|| out = Some(step(playing(), Input::ChangeGhost(r.clone()), &cfg())));
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::INFO, "change_pending");
    assert!(actions.is_empty(), "保留の間は何も送らない");
    assert!(matches!(s.phase, Phase::Steady { talk: Some(_) }));
    assert!(s.change.is_none(), "保留の間は受理した切替の欄は空");
    assert_eq!(s.pending_change, Some(r));
    s
}

/// 利用者の中断を受けてから `TalkDone{Interrupted}` を返す。
fn broken(s: State, quit_reserved: bool) -> (State, Vec<Action>) {
    let (s, actions) = step(s, Input::UserBreak { scope: 0 }, &cfg());
    assert!(matches!(actions.as_slice(), [Action::CancelChoice { .. }]));
    let done = TalkDone {
        talk_id: TalkId(4),
        reason: TalkEndReason::Interrupted,
        quit_reserved,
    };
    step(s, Input::TalkDone(done), &cfg())
}

/// 送ったイベント名（GET／NOTIFY）の列。
fn sent_events(actions: &[Action]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, .. })
            | Action::ShioriRequest(ShioriCall::Notify { id, .. }) => Some(id.as_str().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn held_request_is_consumed_on_end_and_on_interrupt() {
    for raise_event in [true, false] {
        type Finish = fn(State) -> (State, Vec<Action>);
        let finishes: [(&str, Finish); 3] = [
            ("ended", |s| talk_done(s, TalkId(4), TalkEndReason::Ended)),
            ("user_break", |s| broken(s, false)),
            ("interrupted", |s| {
                talk_done(s, TalkId(4), TalkEndReason::Interrupted)
            }),
        ];
        for (label, finish) in finishes {
            let (s, actions) = finish(held(raise_event));
            assert!(s.pending_change.is_none(), "{label}: 保留は消化される");
            let change = s.change.as_ref().expect("消化で受理した切替の欄が立つ");
            assert_eq!(change.req, req(ChangeOrigin::Manual, raise_event));
            if raise_event {
                assert!(matches!(s.phase, Phase::ChangePending), "{label}");
                assert_eq!(only_get(&actions).0, "OnGhostChanging", "{label}");
            } else {
                assert_unloading(&s, &actions, "CloseSilent");
                assert!(sent_events(&actions).is_empty(), "{label}");
            }
        }
    }
}

#[test]
fn without_raise_event_sends_neither_changing_nor_close() {
    let (s, actions) = step(
        steady(),
        Input::ChangeGhost(req(ChangeOrigin::Manual, false)),
        &cfg(),
    );
    let (s, more) = reply(s, ShioriOutcome::Unloaded);
    assert!(matches!(s.phase, Phase::Stopped));
    let all: Vec<Action> = actions.into_iter().chain(more).collect();
    assert!(
        sent_events(&all).is_empty(),
        "OnGhostChanging も OnClose も 0 件"
    );
    assert!(s.change.is_some(), "停止通知には切替の中身が載る");
}

#[test]
fn second_request_while_held_is_rejected() {
    let s = held(true);
    let (s, actions) = step(
        s,
        Input::ChangeGhost(req(ChangeOrigin::Automatic, true)),
        &cfg(),
    );
    assert!(matches!(
        actions.as_slice(),
        [Action::Notice(KanadeNotice::ChangeCancelled {
            reason: CancelReason::Rejected
        })]
    ));
    assert_eq!(s.pending_change, Some(req(ChangeOrigin::Manual, true)));
}

#[test]
fn pending_close_wins_over_held_change() {
    let s = held(true);
    let (s, _) = step(
        s,
        Input::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
        &cfg(),
    );
    assert!(s.pending_close.is_some());
    let (s, actions) = talk_done(s, TalkId(4), TalkEndReason::Ended);
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::Notice(KanadeNotice::ChangeCancelled {
                reason: CancelReason::CloseRequest
            })
        )),
        "切替の中止（終了要求）を通知する"
    );
    assert_eq!(sent_events(&actions), vec!["OnClose".to_string()]);
    assert!(matches!(s.phase, Phase::ClosePending { .. }));
    assert!(s.pending_change.is_none() && s.change.is_none());
}

#[test]
fn quit_reserved_break_drops_held_change_and_quits_without_handoff() {
    for raise_event in [true, false] {
        let s = held(raise_event);
        let mut out = None;
        let ev = capture(|| out = Some(broken(s, true)));
        let (s, actions) = out.expect("step は必ず結果を返す");
        logged_once(&ev, Level::INFO, "change_dropped_by_quit");
        assert_unloading(&s, &actions, "Quit");
        assert!(sent_events(&actions).is_empty(), "OnGhostChanging は 0 件");
        assert!(s.pending_change.is_none(), "保留は捨てる");
        let (s, _) = reply(s, ShioriOutcome::Unloaded);
        assert!(matches!(s.phase, Phase::Stopped));
        assert!(
            s.change.is_none(),
            "終了系列の末の停止通知に切替の中身は無し"
        );
    }
}

#[test]
fn quit_tag_reached_drops_held_change() {
    let s = held(true);
    let mut out = None;
    let ev = capture(|| out = Some(talk_done(s, TalkId(4), TalkEndReason::Quit)));
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::INFO, "change_dropped_by_quit");
    assert_unloading(&s, &actions, "Quit");
    assert!(s.pending_change.is_none() && s.change.is_none());
}

#[path = "change_cancel_tests.rs"]
mod cancel_tests;
