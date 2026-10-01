//! 台詞の切れ目の口のテスト（areka-P0-shell-balloon-switch 要件 1.14・2.3・3.1・5.4・5.7・8.11）。
//!
//! 印の無い依頼（`Input::AwaitTalkGap { raise: None }`）を `step` 経由で駆動し、始め方（定常でない・
//! 終了の保留あり → 定常でない／それ以外 → 見張り）と、毎 `step` の後の見極めの結果
//! （達した・終了で達しない・ゴースト切替で達しない）が 1 度決まったら変わらないことを固定する。
//! 許可表に無い印は送らずに「送らなかった」になることも見る。
//!
//! 印の付いた依頼（`OnShellChanging`・要件 2.1・2.2・5.1・5.2・8.11・12.8）は、印のイベントの GET の
//! 送出と、その応答の台詞の終わり方（最後まで・置き換え・台詞なし）・利用者の中断の例外（`\-` の
//! 予約があっても終了系列へ進めない）・自ら `\-` に達したときの終了を固定する。

use super::*;
use crate::change::{
    ChangeOrigin, ChangeRequest, ChangeTarget, GapLeft, RaiseOutcome, ShioriMethod,
};
use crate::msg::{
    ChoiceInput, CloseReason, KanadeConfig, MonotonicMs, ShioriCall, ShioriFailure, ShioriOutcome,
};
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
use crate::schedule::steady::test_support::base_state;
use crate::schedule::{ActiveTalk, ChoicePhase, ChoiceState, Input, Phase, TermCause, step};
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

// ============================================================
// 印の付いた依頼（`OnShellChanging`）
// ============================================================

/// 印のイベントの名前。
const SHELL_CHANGING: &str = "OnShellChanging";

/// 印の台詞の番号（`steady_none` の採番 5 から始まる）。
const MARKED: TalkId = TalkId(5);

/// 印の台詞にも印の無い台詞にも使う同じ台本（終了の予約の有無は完了通知が運ぶ）。
const SCRIPT: &str = "SCRIPT_WITH_QUIT";

fn shell_changing() -> GapRaise {
    GapRaise {
        id: SHELL_CHANGING.to_string(),
        references: vec![
            "B".to_string(),
            "ポスト".to_string(),
            "SHELL_B_PATH".to_string(),
        ],
        method: ShioriMethod::Get,
    }
}

/// 印の付いた依頼を 1 件 `step` へ渡す。
fn await_marked(s: State) -> (State, Vec<Action>) {
    step(
        s,
        Input::AwaitTalkGap {
            raise: Some(shell_changing()),
        },
        &cfg(),
    )
}

fn reply(s: State, outcome: ShioriOutcome, origin: &'static str) -> (State, Vec<Action>) {
    step(s, Input::ShioriReply { outcome, origin }, &cfg())
}

fn done_of(
    s: State,
    talk_id: TalkId,
    reason: TalkEndReason,
    quit_reserved: bool,
) -> (State, Vec<Action>) {
    let done = TalkDone {
        talk_id,
        reason,
        quit_reserved,
    };
    step(s, Input::TalkDone(done), &cfg())
}

fn value() -> ShioriOutcome {
    ShioriOutcome::Value(SCRIPT.to_string())
}

fn is_playing(s: &State, id: TalkId) -> bool {
    matches!(&s.phase, Phase::Steady { talk: Some(active) } if active.talk_id == id)
}

/// GET の Action から (イベント名, Reference 列) を取り出す。
fn get_call(action: &Action) -> (String, Vec<String>) {
    match action {
        Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => {
            (id.as_str().to_string(), references.clone())
        }
        _ => panic!("GET の ShioriRequest のはず"),
    }
}

/// 印の依頼を送り、その応答の台本で印の台詞（[`MARKED`]）を始めた状態。
fn marked_talk_playing() -> State {
    let (s, _) = await_marked(steady_none());
    let (s, actions) = reply(s, value(), SHELL_CHANGING);
    assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    assert!(is_playing(&s, MARKED));
    assert_waiting(&s, "印の台詞の再生中");
    s
}

#[test]
fn marked_request_sends_the_event_by_get_with_the_references_as_given() {
    let mut out = None;
    let ev = capture(|| out = Some(await_marked(steady_none())));
    let (s, actions) = out.expect("step は必ず結果を返す");
    assert_eq!(actions.len(), 1, "印のイベントをちょうど 1 件送る");
    assert_eq!(
        get_call(&actions[0]),
        (SHELL_CHANGING.to_string(), shell_changing().references),
        "Reference は渡したまま"
    );
    assert_waiting(&s, "印の応答待ち");
    assert_not_logged(&ev, "raise_event_not_allowed");
}

#[test]
fn marked_request_outside_steady_or_with_pending_close_sends_nothing() {
    let cases: Vec<(&str, State)> = vec![
        (
            "BootMain",
            State {
                phase: Phase::BootMain,
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
        let (mut s, actions) = await_marked(s);
        assert!(
            actions.is_empty(),
            "{label}: 印のイベントを送らない（要件 5.7）"
        );
        assert_eq!(
            take_outcome(&mut s),
            Some(TalkGap::Left {
                reason: GapLeft::NotSteady
            }),
            "{label}"
        );
    }
}

#[test]
fn marked_talk_played_to_the_end_is_reached_completed() {
    // 何も再生していないときの印の台詞。
    let (mut s, _) = done_of(marked_talk_playing(), MARKED, TalkEndReason::Ended, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::Completed)
        })
    );

    // 再生中に送った印の応答は今のトークを置き換え、その印の台詞の終わりで「最後まで」。
    let (s, _) = await_marked(steady_some());
    let (s, actions) = reply(s, value(), SHELL_CHANGING);
    assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    assert!(is_playing(&s, MARKED));
    assert_waiting(&s, "置き換えて始めた印の台詞の再生中");
    let (mut s, _) = done_of(s, MARKED, TalkEndReason::Ended, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::Completed)
        })
    );
}

#[test]
fn marked_talk_replaced_by_another_reply_is_reached_replaced_at_the_end_of_the_replacing_talk() {
    let s = marked_talk_playing();
    // 再生中のダブルクリックの応答が印の台詞を置き換える（旧トークの完了は dispatcher が捨てて届かない）。
    let (s, _) = reply(
        s,
        ShioriOutcome::Value("OTHER".to_string()),
        "OnMouseDoubleClick",
    );
    let replacing = TalkId(6);
    assert!(is_playing(&s, replacing));
    assert_waiting(&s, "置き換えた台詞の再生中");
    let (mut s, _) = done_of(s, replacing, TalkEndReason::Ended, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::Replaced)
        }),
        "置き換えた台詞の終わりで「置き換え」"
    );
}

#[test]
fn replacement_by_a_choice_is_detected_apart_from_the_one_generation_stale_slot() {
    let s = marked_talk_playing();
    let (s, _) = step(
        s,
        Input::ChoiceWaiting {
            talk_id: MARKED,
            choice_ids: vec!["OnMenu".to_string()],
            display_end: MonotonicMs(1_000),
            timeout_directive_secs: None,
        },
        &cfg(),
    );
    let (s, actions) = step(
        s,
        Input::Choice(ChoiceInput {
            id: "OnMenu".to_string(),
            label: "メニュー".to_string(),
            scope: 0,
            references: Vec::new(),
        }),
        &cfg(),
    );
    assert_eq!(actions.len(), 1, "選択はカスケードの GET を 1 件送る");
    get_call(&actions[0]);
    let (s, _) = reply(
        s,
        ShioriOutcome::Value("CHOSEN".to_string()),
        "OnChoiceSelectEx",
    );
    let replacing = TalkId(6);
    assert!(is_playing(&s, replacing));
    assert_eq!(
        s.choice_prev_talk,
        Some(MARKED),
        "選択の置き換えは印の台詞の番号を 1 世代だけ控える"
    );

    // 控えに当たる印の台詞の遅れた完了は「最後まで」にしない。
    let mut out = None;
    let ev = capture(|| out = Some(done_of(s, MARKED, TalkEndReason::Ended, false)));
    let (s, _) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::INFO, "talk_done_stale_choice");
    assert_waiting(&s, "遅れた旧トークの完了の後");

    let (mut s, _) = done_of(s, replacing, TalkEndReason::Ended, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::Replaced)
        })
    );
}

#[test]
fn choice_timeout_release_of_the_marked_talk_is_completed() {
    let mut s = marked_talk_playing();
    s.choice = Some(ChoiceState {
        talk_id: MARKED,
        candidates: vec!["OnMenu".to_string()],
        deadline: Some(MonotonicMs(2_000)),
        phase: ChoicePhase::Waiting,
    });
    let (s, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(2_000),
        },
        &cfg(),
    );
    assert_eq!(get_call(&actions[0]).0, "OnChoiceTimeout");
    let (s, actions) = reply(s, ShioriOutcome::NoContent, "OnChoiceTimeout");
    assert!(
        matches!(actions.as_slice(), [Action::CancelChoice { talk_id }] if *talk_id == MARKED),
        "時間切れの 204 は選択待ちを解く"
    );
    let (mut s, _) = done_of(s, MARKED, TalkEndReason::Interrupted, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::Completed)
        }),
        "時間切れの解除は利用者の中断でないので「最後まで」"
    );
}

#[test]
fn no_content_reply_is_reached_no_talk() {
    // 何も再生していなければ応答の直後に達する。
    let (s, _) = await_marked(steady_none());
    let (mut s, actions) = reply(s, ShioriOutcome::NoContent, SHELL_CHANGING);
    assert!(actions.is_empty());
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::NoTalk)
        })
    );

    // 別の台詞が再生中なら、その台詞の終わりで達する。
    let (s, _) = await_marked(steady_some());
    let (s, _) = reply(s, ShioriOutcome::NoContent, SHELL_CHANGING);
    assert!(is_playing(&s, TALK), "204 は今のトークを変えない");
    assert_waiting(&s, "別の台詞の再生中");
    let (mut s, _) = done_of(s, TALK, TalkEndReason::Ended, false);
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Reached {
            marked: Some(MarkedEnd::NoTalk)
        })
    );
}

#[test]
fn user_break_of_the_marked_talk_is_cancelled_by_user_even_with_quit_reserved() {
    for quit_reserved in [true, false] {
        let (s, _) = step(marked_talk_playing(), Input::UserBreak { scope: 0 }, &cfg());
        let mut out = None;
        let ev = capture(|| {
            out = Some(done_of(
                s,
                MARKED,
                TalkEndReason::Interrupted,
                quit_reserved,
            ))
        });
        let (mut s, actions) = out.expect("step は必ず結果を返す");
        assert!(
            matches!(s.phase, Phase::Steady { talk: None }),
            "quit_reserved={quit_reserved}: 定常に戻る"
        );
        assert!(
            actions.is_empty(),
            "quit_reserved={quit_reserved}: 終了系列の指示は 0"
        );
        assert!(
            s.user_break_talk.is_none(),
            "中断の帳簿は今日どおり空になる"
        );
        assert_not_logged(&ev, "talk_done_break_quit");
        logged_once(&ev, Level::INFO, "talk_gap_marked_break");
        assert_eq!(take_outcome(&mut s), Some(TalkGap::CancelledByUser));
    }
}

#[test]
fn the_same_script_without_the_mark_goes_to_closing_when_broken() {
    // 同じ台本を印の無い応答（ダブルクリック）で始め、印の無い依頼で見張って同じ中断を掛ける。
    let (s, _) = reply(steady_none(), value(), "OnMouseDoubleClick");
    assert!(is_playing(&s, MARKED));
    let (s, _) = await_gap(s);
    let (s, _) = step(s, Input::UserBreak { scope: 0 }, &cfg());
    let (mut s, actions) = done_of(s, MARKED, TalkEndReason::Interrupted, true);
    assert!(matches!(
        s.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::Closing
        })
    );
}

#[test]
fn marked_talk_reaching_quit_by_itself_is_closing() {
    let (mut s, actions) = done_of(marked_talk_playing(), MARKED, TalkEndReason::Quit, false);
    assert!(matches!(
        s.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::Closing
        }),
        "台本が自ら終了を求めたら終了が勝つ（要件 5.2 の例外は中断だけ）"
    );
}

#[test]
fn failed_marked_reply_is_closing() {
    let (s, _) = await_marked(steady_none());
    let (mut s, _) = reply(
        s,
        ShioriOutcome::Failed(ShioriFailure::Timeout("t".to_string())),
        SHELL_CHANGING,
    );
    assert!(matches!(s.phase, Phase::Unloading { .. }));
    assert_eq!(
        take_outcome(&mut s),
        Some(TalkGap::Left {
            reason: GapLeft::Closing
        })
    );
}
