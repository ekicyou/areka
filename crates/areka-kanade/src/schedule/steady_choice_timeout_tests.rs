use super::test_support::{
    assert_no_second_change, choice_input_of, config, expect_get_call, expect_ledger, status_wire,
    steady_some, steady_with_ledger,
};
use super::*;
use crate::msg::ShioriCall;
use crate::schedule::step;
use crate::talk::TalkEndReason;

// ============================================================
// 選択肢タイムアウト（タスク 4.5・C4 規則 5／6・Req7.3〜7.5・DD-10／DD-11）
// ============================================================
//
// 完了状態の要求は「**注入時刻のみ**で期限到達・イベント発行・空応答時の解除までが再現し、
// 実時間待機に依存しない」ことである。よって本群は一切スリープせず、`Input::Tick { now }` の
// 注入値と帳簿の `deadline` の比較だけで全分岐を踏む（`deterministic-test-coverage-mandate`）。

use crate::schedule::log_capture::{assert_logged, capture, logged_once};
use tracing::Level;

/// 選択待ち（`Waiting`）帳簿つきの `Steady{Some}` を、起動スクリプトと期限を指定して構築する。
///
/// `deadline: None` は無期限（`0`／`-1` 指令が [`choice_deadline`] で写った形・Req7.6）。
fn steady_waiting(
    talk_id: TalkId,
    next_id: u64,
    script: &str,
    deadline: Option<MonotonicMs>,
) -> State {
    let mut s = steady_some(talk_id, next_id);
    match &mut s.phase {
        Phase::Steady { talk: Some(active) } => active.script = script.to_string(),
        _ => unreachable!("steady_some は Steady{{Some}} を返す"),
    }
    s.choice = Some(ChoiceState {
        talk_id,
        candidates: vec!["OnMenu".to_string()],
        deadline,
        phase: ChoicePhase::Waiting,
    });
    s
}

/// `step` を捕捉つきで駆動する（遷移結果と捕捉ログを同時に取る）。
fn step_capturing(
    state: State,
    input: Input,
    config: &KanadeConfig,
) -> (
    State,
    Vec<Action>,
    Vec<crate::schedule::log_capture::CapturedEvent>,
) {
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(state, input, config));
    });
    let (next, actions) = out.expect("step は必ず結果を返す");
    (next, actions, ev)
}

// --- A. 期限到達（規則 5・Req7.3） ---

/// Req7.3・DD-10: 注入時刻が期限**ちょうど**（`now == deadline`）で到達し、`OnChoiceTimeout` を
/// Ref0＝起動スクリプトで発行する。帳簿は `TimeoutInFlight` へ進み、当該 Tick は pump を出さない。
#[test]
fn choice_timeout_fires_at_deadline_with_script_ref0_and_suppresses_pump() {
    let script = r"\0えらんでね\q[はい,OnMenu]\e";
    let s = steady_waiting(TalkId(3), 6, script, Some(MonotonicMs(32_000)));
    let (next, actions, ev) = step_capturing(
        s,
        Input::Tick {
            now: MonotonicMs(32_000),
        },
        &config(),
    );
    assert_eq!(actions.len(), 1, "期限到達 Tick は GET を 1 件だけ発行する");
    let (id, refs) = expect_get_call(&actions[0]);
    assert_eq!(
        id, "OnChoiceTimeout",
        "期限到達で OnChoiceTimeout を発行（Req7.3）"
    );
    assert_eq!(
        refs,
        vec![script.to_string()],
        "Ref0＝タイムアウトした選択肢を含むトークの起動スクリプト（Req3.4・DD-10）"
    );
    // 当該周期では通常の周期送出を行わない（規則 5）。
    assert_no_second_change(&actions);
    assert!(
        matches!(expect_ledger(&next).phase, ChoicePhase::TimeoutInFlight),
        "期限到達で帳簿は TimeoutInFlight へ進む"
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "タイムアウト発行は Phase を触らない（DD-3）"
    );
    assert_eq!(
        next.last_now,
        Some(MonotonicMs(32_000)),
        "発行有無に依らず last_now は進む（既存規律）"
    );
    // 4.4 申し送り: 5 番目の呼出点にも choosing が載る（TimeoutInFlight も選択待ち継続中）。
    assert_eq!(
        status_wire(&actions[0]),
        Some("talking,choosing".to_string()),
        "OnChoiceTimeout GET の Status に choosing が載る（C5・裁定 6）"
    );
    let fired = logged_once(&ev, Level::INFO, "choice_timeout_fired");
    assert_eq!(
        fired.fields.get("talk_id").map(String::as_str),
        Some("3"),
        "発火ログは対象 talk_id を載せる。\n捕捉={ev:#?}"
    );
}

/// 境界: `now > deadline`（Tick が期限を跨いだ）でも到達（`>=` 判定）。
#[test]
fn choice_timeout_fires_when_tick_overshoots_deadline() {
    let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
    let (next, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(33_000),
        },
        &config(),
    );
    let (id, _) = expect_get_call(&actions[0]);
    assert_eq!(id, "OnChoiceTimeout");
    assert!(matches!(
        expect_ledger(&next).phase,
        ChoicePhase::TimeoutInFlight
    ));
}

/// 境界: `now < deadline`（1ms 手前）では発行せず、通常の周期送出を続ける。
#[test]
fn choice_timeout_does_not_fire_before_deadline() {
    let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
    let (next, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(31_999),
        },
        &config(),
    );
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::ShioriRequest(ShioriCall::Notify { id, .. }) => {
            assert_eq!(
                id.as_str(),
                "OnSecondChange",
                "期限前は通常の周期送出（NOTIFY）"
            )
        }
        _ => panic!("期限前は選択待ち中の通常 pump（NOTIFY）が出る"),
    }
    assert!(
        matches!(expect_ledger(&next).phase, ChoicePhase::Waiting),
        "期限前は選択待ちのまま"
    );
}

/// Req7.6: 無期限（`deadline == None`）は計測を開始せず、どれだけ時刻が進んでも発行しない。
/// 選択待ちは継続し、pump は通常どおり発行される。
#[test]
fn choice_timeout_never_fires_for_indefinite_deadline() {
    let cfg = config();
    let mut s = steady_waiting(TalkId(3), 6, r"\e", None);
    for now in [1_000_u64, 60_000, u64::MAX] {
        let (next, actions) = step(
            s,
            Input::Tick {
                now: MonotonicMs(now),
            },
            &cfg,
        );
        assert_eq!(actions.len(), 1, "無期限でも周期送出は続く（Req7.6）");
        match &actions[0] {
            Action::ShioriRequest(ShioriCall::Notify { id, status, .. }) => {
                assert_eq!(id.as_str(), "OnSecondChange");
                assert_eq!(
                    status.render(),
                    Some("talking,choosing".to_string()),
                    "無期限の選択待ちは継続する（choosing が載り続ける）"
                );
            }
            _ => panic!("無期限では OnChoiceTimeout を発行しない（Req7.6）"),
        }
        assert!(matches!(expect_ledger(&next).phase, ChoicePhase::Waiting));
        s = next;
    }
}

/// 規則 5: 発行した Tick でのみ pump を止める——**次 Tick からは再開**する
/// （応答待ち中も slot 占有は継続＝NOTIFY・choosing 継続）。
#[test]
fn pump_resumes_on_the_tick_after_choice_timeout_fired() {
    let cfg = config();
    let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
    let (fired, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(32_000),
        },
        &cfg,
    );
    assert_eq!(expect_get_call(&actions[0]).0, "OnChoiceTimeout");
    let (next, resumed) = step(
        fired,
        Input::Tick {
            now: MonotonicMs(33_000),
        },
        &cfg,
    );
    assert_eq!(resumed.len(), 1, "次 Tick から周期送出が再開する（規則 5）");
    match &resumed[0] {
        Action::ShioriRequest(ShioriCall::Notify { id, status, .. }) => {
            assert_eq!(id.as_str(), "OnSecondChange");
            assert_eq!(
                status.render(),
                Some("talking,choosing".to_string()),
                "応答待ち（TimeoutInFlight）中も選択待ちは継続中（C5）"
            );
        }
        _ => panic!("次 Tick は通常の pump（NOTIFY）"),
    }
    assert!(
        matches!(expect_ledger(&next).phase, ChoicePhase::TimeoutInFlight),
        "再開した pump は帳簿を触らない（二重発行もしない）"
    );
    assert_eq!(
        resumed
            .iter()
            .filter(|a| matches!(a, Action::ShioriRequest(ShioriCall::Get { .. })))
            .count(),
        0,
        "OnChoiceTimeout を二重発行しない"
    );
}

// --- B. タイムアウト応答（規則 6・Req7.4／7.5） ---

/// Req7.4: 応答スクリプトは**既存の起動経路**で置換再生する（新 talk_id 採番・slot 差替・
/// 旧 talk_id は 1 世代保持）。解決指示（`ResolveChoice`）は発行しない——旧トークは
/// dispatcher の Close-then-spawn で終了する（F3）。
#[test]
fn choice_timeout_value_replaces_talk_via_existing_start_path() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
    let (next, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(r"\0時間切れ\e".to_string()),
            origin: "OnChoiceTimeout",
        },
        &config(),
    );
    match actions.as_slice() {
        [
            Action::StartTalk(StartTalk {
                talk_id, script, ..
            }),
        ] => {
            assert_eq!(*talk_id, TalkId(6), "新 talk_id を採番する（Req7.4／4.1）");
            assert_eq!(script, r"\0時間切れ\e");
        }
        _ => panic!("タイムアウト Value は StartTalk のみを発行する（F3）"),
    }
    match next.phase {
        Phase::Steady {
            talk:
                Some(ActiveTalk {
                    talk_id,
                    origin,
                    ref script,
                }),
        } => {
            assert_eq!(talk_id, TalkId(6), "slot は新 talk へ差し替わる");
            assert_eq!(origin, "OnChoiceTimeout", "応答の出所を転記する");
            assert_eq!(script, r"\0時間切れ\e", "起動 script を保持（DD-10）");
        }
        _ => panic!("expected Steady{{Some}} replaced"),
    }
    assert_eq!(next.next_talk_id, 7);
    assert!(next.choice.is_none(), "置換起動で帳簿は消える（Req7.4）");
    assert_eq!(
        next.choice_prev_talk,
        Some(TalkId(3)),
        "タイムアウト Value も choice 起因の slot 差替＝旧 talk_id を 1 世代保持（遷移規則 9）"
    );
}

/// Req7.5・DD-11: 204 は選択待ちを解除し `CancelChoice` を発行する（Close funnel の入口）。
/// 独自のバリア状態・小細工（skip_barrier 等）は使わない——発行するのは正規の型付き指示のみ。
#[test]
fn choice_timeout_no_content_cancels_choice() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
    let (next, actions, ev) = step_capturing(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceTimeout",
        },
        &config(),
    );
    match actions.as_slice() {
        [Action::CancelChoice { talk_id }] => {
            assert_eq!(*talk_id, TalkId(3), "解除対象は選択待ちの talk（Req7.5）")
        }
        _ => panic!("204 は CancelChoice のみを発行する（DD-11・F3）"),
    }
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::StartTalk(_) | Action::ResolveChoice { .. })),
        "204 は起動も解決も行わない（解除して終了させる・Req7.5）"
    );
    assert!(next.choice.is_none(), "解除で帳簿は消える（Req6.2／7.5）");
    assert_eq!(next.next_talk_id, 6, "204 は採番しない");
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(
            talk_id,
            TalkId(3),
            "slot は維持したまま Close funnel の完了（TalkDone{{Interrupted}}）を待つ（DD-11）"
        ),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert_logged(&ev, Level::INFO, "choice_timeout_cancelled");
}

/// Req4.5／7.5: タイムアウト GET の失敗も 204 と同一＝解除で継続（終了系列へ倒れない）。
///
/// カスケード段の檻と同じく `steady::step` の直接駆動で steady 側の処理を固定する（層を分けた
/// 単体檻）。`step()` 経由の end-to-end（DD-12 の免除アームが効くこと）は
/// [`timeout_failed_via_step_cancels_without_unloading`] が固定する。
#[test]
fn choice_timeout_failed_is_treated_as_no_content_cancel() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
    let mut out = None;
    let ev = capture(|| {
        out = Some(super::step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(crate::msg::ShioriFailure::Timeout(
                    "30s".to_string(),
                )),
                origin: "OnChoiceTimeout",
            },
            &config(),
        ));
    });
    let (next, actions) = out.expect("step は必ず結果を返す");
    assert!(
        !matches!(next.phase, Phase::Unloading { .. }),
        "選択由来の失敗で終了系列へ倒れない（Req4.5）"
    );
    match actions.as_slice() {
        [Action::CancelChoice { talk_id }] => assert_eq!(*talk_id, TalkId(3)),
        _ => panic!("失敗は 204 と同一＝CancelChoice のみ（Req7.5）"),
    }
    assert!(next.choice.is_none());
    assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
    assert_logged(&ev, Level::INFO, "choice_timeout_cancelled");
}

// --- C. 解除後の棄却（Req7.5 後半・DD-11 の正規到達点） ---

/// Req7.5: 解除後に到着する当該選択待ち宛の選択確定は棄却する。
///
/// 経路は正規（小細工なし）: 期限到達 → `OnChoiceTimeout` → 204 → `CancelChoice` →
/// （dispatcher が Close を転送し talk が返す）`TalkDone{Interrupted}` → `Steady{None}` 復帰。
/// この `Interrupted` こそ DD-11 が本設計で**正規到達点**にした経路である（mod.rs 防御アーム）。
#[test]
fn choice_after_timeout_cancel_is_rejected() {
    let cfg = config();
    // 1) 期限到達 → OnChoiceTimeout。
    let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
    let (s1, fired) = step(
        s,
        Input::Tick {
            now: MonotonicMs(32_000),
        },
        &cfg,
    );
    assert_eq!(expect_get_call(&fired[0]).0, "OnChoiceTimeout");
    // 2) 204 → CancelChoice（選択待ち解除）。
    let (s2, cancelled) = step(
        s1,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceTimeout",
        },
        &cfg,
    );
    assert!(matches!(
        cancelled.as_slice(),
        [Action::CancelChoice { .. }]
    ));
    // 3) Close funnel の完了通知（正規の TalkDone{Interrupted}）→ Steady{None} 復帰。
    let (s3, done_actions, ev) = step_capturing(
        s2,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Interrupted,
        }),
        &cfg,
    );
    assert_logged(&ev, Level::INFO, "talk_done_interrupted_as_non_quit");
    assert!(
        matches!(s3.phase, Phase::Steady { talk: None }),
        "CancelChoice→Close→Interrupted で Steady{{None}} へ復帰する（DD-11）"
    );
    assert!(done_actions.is_empty());
    // 4) 以降に届く当該選択待ち宛の選択確定は棄却される（Req7.5）。
    let (s4, late, ev2) = step_capturing(
        s3,
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &cfg,
    );
    assert!(
        late.is_empty(),
        "解除後の選択確定は Action を発行しない（Req7.5）"
    );
    assert!(s4.choice.is_none(), "棄却は帳簿を復活させない");
    assert_logged(&ev2, Level::WARN, "choice_rejected_no_wait");
}

/// 選択待ち中の周期送出（choosing）が、解除後の pump からは消える（Req6.2 のタイムアウト側）。
#[test]
fn tick_after_timeout_cancel_drops_choosing() {
    let cfg = config();
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
    let (cancelled, _) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceTimeout",
        },
        &cfg,
    );
    let (_next, tick_actions) = step(
        cancelled,
        Input::Tick {
            now: MonotonicMs(40_000),
        },
        &cfg,
    );
    assert_eq!(
        status_wire(&tick_actions[0]),
        Some("talking".to_string()),
        "解除後は choosing が消える（Req6.2）"
    );
}

// ============================================================
// 帳簿の掃除・選択起因の失敗例外（タスク 4.6・C4 規則 7／8・DD-12・Req1.3／4.5／6.2）
// ============================================================

use crate::msg::ShioriFailure;
use crate::schedule::TermCause;
use crate::schedule::log_capture::{assert_no_error_logs, assert_not_logged};

/// 規則 7 の不変条件: 帳簿があるなら、その対象 talk は現行 talk と一致する。
///
/// 「帳簿の対象と現行トークが食い違う状態を残さない」ことの直接表明であり、掃除点ごとの
/// 個別 assert（`choice.is_none()`）と併せて各掃除点で突合する。
fn assert_choice_invariant(state: &State) {
    let active = match &state.phase {
        Phase::Steady { talk: Some(active) } => Some(active.talk_id),
        _ => None,
    };
    if let Some(ledger) = state.choice.as_ref() {
        assert_eq!(
            Some(ledger.talk_id),
            active,
            "帳簿の対象と現行トークが食い違う状態を残してはならない（C4 規則 7）"
        );
    }
}

// --- A. 帳簿の掃除（規則 7・Req1.3／6.2） ---

/// 規則 7: 対象トークの完了（`TalkDone{Ended}`）で帳簿を消去する。
#[test]
fn talk_done_of_target_talk_clears_choice_ledger() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    let (next, actions) = step(
        s,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Ended,
        }),
        &config(),
    );
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert!(
        next.choice.is_none(),
        "対象トークの完了で帳簿は消える（規則 7）"
    );
    assert!(actions.is_empty());
    assert_choice_invariant(&next);
}

/// 規則 7: 対象トークの `TalkDone{Quit}`（終了系列直行）でも帳簿を消去する。
#[test]
fn talk_done_quit_clears_choice_ledger() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    let (next, actions) = step(
        s,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Quit,
        }),
        &config(),
    );
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));
    assert!(
        next.choice.is_none(),
        "終了系列（Quit）へ進む際も帳簿は消える"
    );
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert_choice_invariant(&next);
}

/// 規則 7: **マウス由来**の slot 置換でも帳簿を消去し、1 世代保持も持ち越さない。
#[test]
fn mouse_slot_replacement_clears_choice_ledger_and_stale_slot() {
    let mut s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    s.choice_prev_talk = Some(TalkId(1));
    let (next, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(r"\0つつかれた\e".to_string()),
            origin: "OnMouseDoubleClick",
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(6), "マウス由来 Value は slot を置換する"),
        _ => panic!("expected Steady{{Some}} replaced"),
    }
    assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    assert!(
        next.choice.is_none(),
        "slot 置換（マウス由来含む）で帳簿は消える（規則 7）"
    );
    assert!(
        next.choice_prev_talk.is_none(),
        "次の slot 差替で 1 世代保持は消える（規則 9）"
    );
    assert_choice_invariant(&next);
}

/// 規則 7: close 握手の開始（`ClosePending` への遷移）で帳簿を消去する。
///
/// 経路は正規: active talk 中の `CloseRequest` は保留記録のみ（選択待ちは継続——ここで
/// 帳簿を消すと選択が棄却され、バリアが解けず `TalkDone` が来ないため握手が進まない）。
/// 掃除が起きるのは実際に握手へ**遷移する**点（`TalkDone` 消化→`ClosePending`）である。
#[test]
fn close_handshake_transition_clears_choice_ledger() {
    let cfg = config();
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    let (pending, actions) = step(
        s,
        Input::CloseRequest {
            reason: CloseReason::User,
        },
        &cfg,
    );
    assert!(actions.is_empty(), "active talk 中の close は保留記録のみ");
    assert!(
        pending.choice.is_some(),
        "保留記録は遷移ではない——選択待ちは継続する（バリアを解けなくしない）"
    );
    assert_choice_invariant(&pending);
    let (next, close_actions) = step(
        pending,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Ended,
        }),
        &cfg,
    );
    assert!(matches!(next.phase, Phase::ClosePending { .. }));
    assert_eq!(close_actions.len(), 1, "握手開始は OnClose GET を発行する");
    assert!(
        next.choice.is_none(),
        "close 系遷移で帳簿は消える（規則 7）"
    );
    assert_choice_invariant(&next);
}

/// 規則 7: `ForceQuit` の横断遷移で帳簿を消去する。
#[test]
fn force_quit_clears_choice_ledger() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["OnMenu"],
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: None,
        },
    );
    let (next, actions) = step(
        s,
        Input::ForceQuit {
            reason: CloseReason::System,
        },
        &config(),
    );
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Forced
        }
    ));
    assert_eq!(actions.len(), 2, "既存の ForceQuit 発行列は不変");
    assert!(next.choice.is_none(), "終了系遷移で帳簿は消える（規則 7）");
    assert_choice_invariant(&next);
}

/// 規則 7: `ShioriDown`（Fault 直行）で帳簿を消去する。
#[test]
fn shiori_down_clears_choice_ledger() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    let (next, actions) = step(
        s,
        Input::ShioriDown {
            reason: "helper crashed".to_string(),
        },
        &config(),
    );
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Fault
        }
    ));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert!(next.choice.is_none(), "終了系遷移で帳簿は消える（規則 7）");
    assert_choice_invariant(&next);
}

// --- B. 選択起因の失敗例外（規則 8・DD-12・Req4.5） ---

/// DD-12: **`step()` 経由**でカスケード段の `Failed` を受けても `Unloading{Fault}` へ倒れず、
/// 204 相当（残段ありなら次段 GET）で継続する（横断 Failed アームの免除）。
#[test]
fn cascade_failed_via_step_does_not_fall_into_unloading_fault() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: Some(CascadeNext::Select),
        },
    );
    let (next, actions, ev) = step_capturing(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "OnChoiceSelectEx",
        },
        &config(),
    );
    assert!(
        !matches!(next.phase, Phase::Unloading { .. }),
        "選択由来の SHIORI 失敗で終了系列へ倒れない（Req4.5・DD-12）"
    );
    assert!(
        !actions.iter().any(|a| matches!(a, Action::ShioriUnload)),
        "免除アームは Unload を発行しない"
    );
    let (id, refs) = expect_get_call(&actions[0]);
    assert_eq!(
        id, "OnChoiceSelect",
        "失敗は 204 と同一＝次段へ前進（Req2.3）"
    );
    assert_eq!(refs, vec!["choice1".to_string()]);
    assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
    assert_not_logged(&ev, "shiori_failed");
}

/// DD-12: 最終段の `Failed` も `step()` 経由で解決のみ（起動なし）へ倒れる。
#[test]
fn cascade_failed_at_last_stage_via_step_resolves_and_continues() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: None,
        },
    );
    let (next, actions, ev) = step_capturing(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
            origin: "OnChoiceSelect",
        },
        &config(),
    );
    assert!(!matches!(next.phase, Phase::Unloading { .. }));
    match actions.as_slice() {
        [Action::ResolveChoice { talk_id, id }] => {
            assert_eq!(*talk_id, TalkId(3));
            assert_eq!(id, "choice1");
        }
        _ => panic!("最終段の失敗は ResolveChoice のみ（Req5.3）"),
    }
    assert!(next.choice.is_none());
    assert_choice_invariant(&next);
    assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
}

/// DD-12: タイムアウト GET の `Failed` も `step()` 経由で `CancelChoice` へ進む。
#[test]
fn timeout_failed_via_step_cancels_without_unloading() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
    let (next, actions, ev) = step_capturing(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "OnChoiceTimeout",
        },
        &config(),
    );
    assert!(
        !matches!(next.phase, Phase::Unloading { .. }),
        "タイムアウト GET の失敗でも終了系列へ倒れない（Req4.5・DD-12）"
    );
    match actions.as_slice() {
        [Action::CancelChoice { talk_id }] => assert_eq!(*talk_id, TalkId(3)),
        _ => panic!("タイムアウト失敗は CancelChoice のみ（Req7.5）"),
    }
    assert!(next.choice.is_none());
    assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
    assert_logged(&ev, Level::INFO, "choice_timeout_cancelled");
}

/// DD-12 の免除は **choice in-flight に限る**——選択待ち（`Waiting`）中や帳簿不在の
/// `Failed` は従来どおり `Unloading{Fault}` へ倒れる（非 choice 経路を巻き込まない）。
#[test]
fn non_choice_failed_still_falls_into_unloading_fault() {
    let cfg = config();
    // 帳簿なし（既存の pump 失敗経路）。
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "OnSecondChange",
        },
        &cfg,
    );
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Fault
        }
    ));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    // 帳簿はあるが応答待ちではない（`Waiting`）＝当該 Failed は choice 起因ではない。
    let (next, actions) = step(
        steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting),
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "OnSecondChange",
        },
        &cfg,
    );
    assert!(
        matches!(
            next.phase,
            Phase::Unloading {
                cause: TermCause::Fault
            }
        ),
        "選択待ち中の pump 失敗は免除対象ではない（規則 8 の条件は in-flight のみ）"
    );
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert!(next.choice.is_none(), "終了系遷移で帳簿は消える（規則 7）");
}

// --- C. 正常系のユーザー操作は error を出さない（完了状態） ---

/// 完了状態: 選択の happy path（受理→カスケード→Value→解決→起動→旧 talk の遅延 Done）で
/// **error レベルのログが 1 件も出ない**（1 世代 stale 防御が `unknown_talk_done` を封じる）。
#[test]
fn choice_happy_path_emits_no_error_level_logs() {
    let cfg = config();
    let mut phases = None;
    let ev = capture(|| {
        // 1) 選択待ち成立の通知 → 帳簿確立。
        let (s1, _) = step(
            steady_some(TalkId(3), 6),
            Input::ChoiceWaiting {
                talk_id: TalkId(3),
                choice_ids: vec!["OnMenu".to_string()],
                display_end: MonotonicMs(10_000),
                timeout_directive_secs: None,
            },
            &cfg,
        );
        // 2) ユーザーの選択確定 → 任意名イベントの GET。
        let (s2, staged) = step(
            s1,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &["a0"])),
            &cfg,
        );
        // 3) Value 応答 → [ResolveChoice, StartTalk]・slot 差替。
        let (s3, resolved) = step(
            s2,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0次のシーン\e".to_string()),
                origin: "OnChoiceEvent",
            },
            &cfg,
        );
        // 4) 旧 talk の遅延 Done（F1 残余レース）→ 1 世代 stale 防御で info 棄却。
        let (s4, late) = step(
            s3,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &cfg,
        );
        phases = Some((staged, resolved, late, s4));
    });
    let (staged, resolved, late, s4) = phases.expect("全 step が完了する");
    assert_eq!(expect_get_call(&staged[0]).0, "OnMenu", "任意名 1 段のみ");
    assert!(matches!(
        resolved.as_slice(),
        [Action::ResolveChoice { .. }, Action::StartTalk(_)]
    ));
    assert!(late.is_empty(), "遅延 Done は Action を発行しない");
    match s4.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(6), "遅延 Done は新 talk を殺さない"),
        _ => panic!("expected Steady{{Some(new)}} preserved"),
    }
    assert_logged(&ev, Level::INFO, "talk_done_stale_choice");
    assert_no_error_logs(&ev);
}
