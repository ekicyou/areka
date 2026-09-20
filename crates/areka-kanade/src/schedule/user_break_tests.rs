// =============================================================================
// 利用者の中断（`Input::UserBreak`）の受理規則の決定論テスト
// =============================================================================
//
// 入口は `schedule::step` の横断の腕で、殻（`actor.rs`）が `KanadeMsg::UserBreak` をそのまま
// 写して渡す。ここでは `step` を直に呼び、返る状態と指示を見る。

use super::super::{
    Action, ActiveTalk, ChoicePhase, ChoiceState, Input, Phase, State, TermCause, log_capture, step,
};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriCall};
use crate::talk::{TalkDone, TalkEndReason, TalkId};
use log_capture::{CapturedEvent, capture, logged_once};
use tracing::Level;

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// 再生中のトークが無い場面を `phase` だけ差し替えて組む。
fn not_playing(phase: Phase) -> State {
    State {
        phase,
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 6,
        ..State::initial()
    }
}

/// 再生中のトーク（起動スクリプト付き）。
fn active(talk_id: TalkId) -> ActiveTalk {
    ActiveTalk {
        talk_id,
        origin: "OnTest",
        script: r"\0ながいおはなし\e".to_string(),
    }
}

/// 再生中のトークが在る場面を `phase` だけ差し替えて組む。
fn playing(phase: Phase) -> State {
    State {
        phase,
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 6,
        ..State::initial()
    }
}

/// 「再生中のトークが在る」3 つの場面（`fn current_talk_id` が `Some` を返す全て）。
///
/// 場面で結論を変えない（要件 2.5）ことを確かめるため、判断分岐 ⑴ のテストはこの 3 つを一律に踏む。
fn playing_states(talk_id: TalkId) -> Vec<(&'static str, State)> {
    vec![
        (
            "Steady{Some}",
            playing(Phase::Steady {
                talk: Some(active(talk_id)),
            }),
        ),
        (
            "BootVersion{Some}",
            playing(Phase::BootVersion {
                talk: Some(active(talk_id)),
            }),
        ),
        (
            "CloseTalkWait",
            playing(Phase::CloseTalkWait {
                talk_id,
                deadline: Some(MonotonicMs(30_500)),
            }),
        ),
    ]
}

/// `step` を捕捉つきで駆動する（遷移結果と捕捉ログを同時に取る）。
fn step_capturing(
    state: State,
    input: Input,
    config: &KanadeConfig,
) -> (State, Vec<Action>, Vec<CapturedEvent>) {
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(state, input, config));
    });
    let (next, actions) = out.expect("step は必ず結果を返す");
    (next, actions, ev)
}

/// 指示の列に `OnChoiceTimeout` の GET が 1 件も無いことを表明する（要件 2.6・3.9）。
fn assert_no_choice_timeout(actions: &[Action], at: &str) {
    assert!(
        !actions.iter().any(|a| matches!(
            a,
            Action::ShioriRequest(ShioriCall::Get { id, .. }) if id.as_str() == "OnChoiceTimeout"
        )),
        "{at}: 中断を受け入れた後に選択肢の時間切れを送ってはならない（要件 2.6）"
    );
}

/// 指示の列に SHIORI への要求が 1 件も無いことを表明する（要件 3.9・9.1）。
fn assert_no_shiori_request(actions: &[Action], at: &str) {
    assert_eq!(
        actions
            .iter()
            .filter(|a| matches!(a, Action::ShioriRequest(_)))
            .count(),
        0,
        "{at}: 中断を理由とするイベントを SHIORI へ 1 件も送らない（要件 3.9）"
    );
}

// --- 判断分岐 ⑵: 止める再生が無い（要件 2.2・6.2） ---

/// 再生中のトークが無ければ、中断の要求は何も止めず、状態も指示も変えない（要件 2.2）。
/// 場面で振り分けない（要件 2.5）ので、定常でない場面でも同じ結果になる。
#[test]
fn user_break_without_playing_talk_changes_nothing() {
    let phases: [(&str, fn() -> Phase); 4] = [
        ("Steady{None}", || Phase::Steady { talk: None }),
        ("Idle", || Phase::Idle),
        ("Stopped", || Phase::Stopped),
        // `OnClose` の応答待ち。再生中のトークが無いので同じ腕に落ちる（設計「on_user_break の規則」）。
        ("ClosePending", || Phase::ClosePending {
            reason: CloseReason::System,
        }),
    ];
    for (label, phase) in phases {
        let (next, actions, ev) = step_capturing(
            not_playing(phase()),
            Input::UserBreak { scope: 1 },
            &config(),
        );
        assert!(actions.is_empty(), "{label}: 指示を 1 つも出さない");
        assert_eq!(
            std::mem::discriminant(&next.phase),
            std::mem::discriminant(&phase()),
            "{label}: 場面を変えない"
        );
        assert!(
            next.user_break_talk.is_none(),
            "{label}: 中断の相手を記録しない"
        );
        assert!(next.choice.is_none(), "{label}: 選択の帳簿を作らない");
        assert!(
            next.pending_close.is_none(),
            "{label}: 終了の保留を作らない"
        );
        assert_eq!(next.next_talk_id, 6, "{label}: 採番しない");
        assert_eq!(
            next.last_now,
            Some(MonotonicMs(500)),
            "{label}: 時刻を動かさない"
        );
        let logged = logged_once(&ev, Level::DEBUG, "balloon_break_no_talk");
        assert_eq!(
            logged.fields.get("reason").map(String::as_str),
            Some("not_playing"),
            "{label}: 理由は「再生していない」（要件 6.2）。\n捕捉={ev:#?}"
        );
        assert_eq!(
            logged.fields.get("scope").map(String::as_str),
            Some("1"),
            "{label}: スコープ番号を載せる（要件 6.2）。\n捕捉={ev:#?}"
        );
    }
}

// --- 判断分岐 ⑴: 再生中なら止める（要件 2.1・3.1・3.2・6.1・9.1） ---

/// 再生中のトークが在れば、場面を問わず停止の指示を**ちょうど 1 件**返し、相手を帳簿に控える。
/// 返す指示は停止の 1 種だけで、SHIORI への要求は 1 件も積まない（要件 3.9・9.1）。
#[test]
fn user_break_while_playing_cancels_the_current_talk_in_every_phase() {
    for (label, state) in playing_states(TalkId(3)) {
        let before = std::mem::discriminant(&state.phase);
        let (next, actions, ev) = step_capturing(state, Input::UserBreak { scope: 0 }, &config());
        match actions.as_slice() {
            [Action::CancelChoice { talk_id }] => assert_eq!(
                *talk_id,
                TalkId(3),
                "{label}: 停止の相手は現行のトーク（要件 2.1・3.2）"
            ),
            _ => panic!(
                "{label}: 返す指示は停止の 1 件だけ（要件 3.2）。実際の件数={}",
                actions.len()
            ),
        }
        assert_no_shiori_request(&actions, label);
        assert_eq!(
            next.user_break_talk,
            Some(TalkId(3)),
            "{label}: 中断を出した相手を帳簿に控える（要件 2.4 の下地）"
        );
        assert_eq!(
            std::mem::discriminant(&next.phase),
            before,
            "{label}: 受理そのものは場面を動かさない（完了通知で動く）"
        );
        let accepted = logged_once(&ev, Level::INFO, "balloon_break_accepted");
        assert_eq!(
            accepted.fields.get("talk_id").map(String::as_str),
            Some("3"),
            "{label}: 受理の記録は相手の talk_id を載せる（要件 6.1）。\n捕捉={ev:#?}"
        );
        assert_eq!(
            accepted.fields.get("scope").map(String::as_str),
            Some("0"),
            "{label}: 受理の記録はスコープ番号を載せる（要件 6.1）。\n捕捉={ev:#?}"
        );
    }
}

/// 要件 2.4: 同じ操作の 2 件目は止める対象が無いものとして扱い、二重に止めない。
#[test]
fn second_user_break_for_the_same_talk_is_ignored() {
    let cfg = config();
    let (accepted, first) = step(
        playing(Phase::Steady {
            talk: Some(active(TalkId(3))),
        }),
        Input::UserBreak { scope: 0 },
        &cfg,
    );
    assert_eq!(first.len(), 1, "1 件目は停止の指示を出す");
    let (next, second, ev) = step_capturing(accepted, Input::UserBreak { scope: 0 }, &cfg);
    assert!(
        second.is_empty(),
        "2 件目は指示を 1 つも出さない（要件 2.4）"
    );
    assert_eq!(
        next.user_break_talk,
        Some(TalkId(3)),
        "2 件目は帳簿を書き換えない"
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "2 件目は場面を変えない"
    );
    let logged = logged_once(&ev, Level::DEBUG, "balloon_break_no_talk");
    assert_eq!(
        logged.fields.get("reason").map(String::as_str),
        Some("already_breaking"),
        "理由は「既に中断している」（設計「on_user_break の規則」）。\n捕捉={ev:#?}"
    );
}

/// 要件 2.5: スコープ番号は「どこで起きたか」を運ぶだけで、受理の結論を変えない。
#[test]
fn scope_number_does_not_change_the_verdict() {
    let cfg = config();
    for scope in [0_u32, 1, 7, u32::MAX] {
        let (next, actions) = step(
            playing(Phase::Steady {
                talk: Some(active(TalkId(3))),
            }),
            Input::UserBreak { scope },
            &cfg,
        );
        assert!(
            matches!(actions.as_slice(), [Action::CancelChoice { talk_id }] if *talk_id == TalkId(3)),
            "scope={scope}: 再生中ならどのスコープでも止める（要件 2.5）"
        );
        assert_eq!(next.user_break_talk, Some(TalkId(3)));

        let (next, actions) = step(
            not_playing(Phase::Steady { talk: None }),
            Input::UserBreak { scope },
            &cfg,
        );
        assert!(
            actions.is_empty(),
            "scope={scope}: 再生中でなければどのスコープでも止めない（要件 2.5）"
        );
        assert!(next.user_break_talk.is_none());
    }
}

// --- 判断分岐 ⑶: 選択肢を待っている最中の中断（要件 2.6・9.4） ---

/// 選択待ちの最中に中断を受け入れたら、その時点で帳簿を消す。完了通知の前でも後でも、
/// 期限を過ぎた刻みで `OnChoiceTimeout` が SHIORI へ出ない（要件 2.6・3.9・裁定 4）。
#[test]
fn user_break_while_choice_waiting_drops_the_ledger_before_any_timeout() {
    let cfg = config();
    let mut waiting = playing(Phase::Steady {
        talk: Some(active(TalkId(3))),
    });
    waiting.choice = Some(ChoiceState {
        talk_id: TalkId(3),
        candidates: vec!["OnMenu".to_string()],
        deadline: Some(MonotonicMs(32_000)),
        phase: ChoicePhase::Waiting,
    });

    let (accepted, actions) = step(waiting, Input::UserBreak { scope: 0 }, &cfg);
    assert!(
        matches!(actions.as_slice(), [Action::CancelChoice { talk_id }] if *talk_id == TalkId(3)),
        "選択待ちの最中でも停止の指示は 1 件だけ"
    );
    assert!(
        accepted.choice.is_none(),
        "受理のその時点で選択の帳簿は消える（要件 2.6）"
    );

    // 完了通知の**前**に期限を過ぎた刻みが届いても、時間切れは出ない。
    let (before_done, tick_actions) = step(
        accepted,
        Input::Tick {
            now: MonotonicMs(40_000),
        },
        &cfg,
    );
    assert_no_choice_timeout(&tick_actions, "完了通知の前の刻み");
    assert!(before_done.choice.is_none(), "刻みは帳簿を復活させない");

    // 完了通知（中断で終わった）の**後**の刻みでも同じ。
    let (after_done, done_actions) = step(
        before_done,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Interrupted,
            quit_reserved: false,
        }),
        &cfg,
    );
    assert_no_choice_timeout(&done_actions, "完了通知そのもの");
    let (_next, late_tick) = step(
        after_done,
        Input::Tick {
            now: MonotonicMs(50_000),
        },
        &cfg,
    );
    assert_no_choice_timeout(&late_tick, "完了通知の後の刻み");
}

// --- 判断分岐 ⑺: 終了の予約（要件 3.4・3.8・7.2） ---

/// 中断を受け入れた状態（定常で `TalkId(3)` を再生中 → 中断 1 件）を組む。
fn after_user_break(cfg: &KanadeConfig) -> State {
    let (accepted, actions) = step(
        playing(Phase::Steady {
            talk: Some(active(TalkId(3))),
        }),
        Input::UserBreak { scope: 0 },
        cfg,
    );
    assert_eq!(actions.len(), 1, "前提: 中断は受け入れられている");
    accepted
}

/// 中断で終わった完了通知を組む。
fn interrupted(quit_reserved: bool) -> Input {
    Input::TalkDone(TalkDone {
        talk_id: TalkId(3),
        reason: TalkEndReason::Interrupted,
        quit_reserved,
    })
}

/// 利用者の中断で止めた台本が終了を予約していたら、`\-` に辿り着いたのと同じ終了へ進む（要件 3.8）。
#[test]
fn user_break_with_reserved_quit_ends_the_ghost() {
    let cfg = config();
    let (next, actions, ev) = step_capturing(after_user_break(&cfg), interrupted(true), &cfg);
    assert!(
        matches!(
            next.phase,
            Phase::Unloading {
                cause: TermCause::Quit
            }
        ),
        "予約ありの中断は終了の場面へ進む（要件 3.8）"
    );
    assert!(
        matches!(actions.as_slice(), [Action::ShioriUnload]),
        "終了の要求を 1 件だけ出す（`\\-` と同じ遷移）。実際の件数={}",
        actions.len()
    );
    assert!(
        next.user_break_talk.is_none(),
        "現行トークの完了で中断の帳簿は空になる"
    );
    let logged = logged_once(&ev, Level::INFO, "talk_done_break_quit");
    assert_eq!(
        logged.fields.get("talk_id").map(String::as_str),
        Some("3"),
        "予約どおり終了へ進んだことを相手の talk_id つきで記録する。\n捕捉={ev:#?}"
    );
}

/// 予約が無ければ、中断は従来どおり定常へ戻るだけで終了しない（要件 3.4）。
#[test]
fn user_break_without_reserved_quit_returns_to_steady() {
    let cfg = config();
    let (next, actions) = step(after_user_break(&cfg), interrupted(false), &cfg);
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "予約なしの中断は定常へ戻る（要件 3.4）"
    );
    assert!(actions.is_empty(), "終了の要求は出さない");
    assert!(next.user_break_talk.is_none(), "中断の帳簿は空になる");
}

/// 利用者の中断を出していないのに予約つきで止まったとき（選択肢の時間切れの解除など）は
/// 終了しない——終了が効くのは利用者の中断のときだけである（要件 3.8 の対偶）。
#[test]
fn reserved_quit_without_user_break_does_not_end_the_ghost() {
    let cfg = config();
    let playing_without_break = playing(Phase::Steady {
        talk: Some(active(TalkId(3))),
    });
    assert!(
        playing_without_break.user_break_talk.is_none(),
        "前提: 中断は出していない"
    );
    let (next, actions) = step(playing_without_break, interrupted(true), &cfg);
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "中断を出していなければ予約つきでも定常へ戻る（要件 3.8）"
    );
    assert!(actions.is_empty(), "終了の要求は出さない");
}

/// 要件 7.2: 中断で定常へ戻った後、次の刻みで普通に次のトークの要求が出る。
#[test]
fn next_tick_after_user_break_asks_for_the_next_talk() {
    let cfg = config();
    let (returned, _) = step(after_user_break(&cfg), interrupted(false), &cfg);
    let (_next, actions) = step(
        returned,
        Input::Tick {
            now: MonotonicMs(41_000),
        },
        &cfg,
    );
    assert!(
        matches!(
            actions.as_slice(),
            [Action::ShioriRequest(ShioriCall::Get { id, .. })] if id.as_str() == "OnSecondChange"
        ),
        "中断の後も次のトークの問い合わせは普通に出る（要件 7.2）"
    );
}

/// 不変条件: 中断の帳簿は現行トークの完了で必ず空になる（終わり方を問わない）。
#[test]
fn user_break_ledger_is_emptied_by_any_completion_of_the_current_talk() {
    let cfg = config();
    for (label, reason) in [
        ("Ended", TalkEndReason::Ended),
        ("Quit", TalkEndReason::Quit),
        ("Interrupted", TalkEndReason::Interrupted),
    ] {
        let mut state = playing(Phase::Steady {
            talk: Some(active(TalkId(3))),
        });
        state.user_break_talk = Some(TalkId(3));
        let (next, _actions) = step(
            state,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason,
                quit_reserved: false,
            }),
            &cfg,
        );
        assert!(
            next.user_break_talk.is_none(),
            "{label}: 現行トークの完了で中断の帳簿は空になる"
        );
    }
}
