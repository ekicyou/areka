use super::log_capture::{assert_logged, capture, logged_once, CapturedEvent};
use super::*;
use crate::msg::{CloseReason, ShioriFailure};
use crate::talk::TalkDone;
use tracing::Level;

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn state_in(phase: Phase) -> State {
    State {
        phase,
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        pending_close: None,
        choice: None,
        choice_prev_talk: None,
    }
}

fn steady_with_talk(talk_id: TalkId) -> Phase {
    Phase::Steady {
        talk: Some(ActiveTalk {
            talk_id,
            origin: "steady",
            script: String::new(),
        }),
    }
}

/// `step()` を捕捉付きで駆動し、発行イベント列を返す（state は move）。
fn run_step(phase: Phase, input: Input) -> Vec<CapturedEvent> {
    let cfg = config();
    capture(|| {
        let _ = step(state_in(phase), input, &cfg);
    })
}

// ============================================================
// 失敗アーム（level = ERROR）
// ============================================================

#[test]
fn error_shiori_down_logs() {
    let ev = run_step(
        steady_with_talk(TalkId(5)),
        Input::ShioriDown {
            reason: "helper crashed".to_string(),
        },
    );
    assert_logged(&ev, Level::ERROR, "shiori_down");
}

#[test]
fn error_shiori_failed_logs() {
    // 応答待ちフェーズ（BootType）+ Failed → 横断アーム shiori_failed。
    let ev = run_step(
        Phase::BootType,
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "test",
        },
    );
    assert_logged(&ev, Level::ERROR, "shiori_failed");
}

#[test]
fn error_unknown_talk_done_logs() {
    // 突合対象 talk (5) と異なる talk_id (999) の TalkDone → unknown_talk_done。
    let ev = run_step(
        steady_with_talk(TalkId(5)),
        Input::TalkDone(TalkDone {
            talk_id: TalkId(999),
            reason: TalkEndReason::Ended,
        }),
    );
    assert_logged(&ev, Level::ERROR, "unknown_talk_done");
}

#[test]
fn error_unload_failed_logs() {
    // Unloading 中の Failed 応答 → unload_failed（終了系列は継続）。
    let ev = run_step(
        Phase::Unloading {
            cause: TermCause::Quit,
        },
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
            origin: "test",
        },
    );
    assert_logged(&ev, Level::ERROR, "unload_failed");
}

#[test]
fn error_close_deadline_exceeded_logs() {
    // CloseTalkWait で deadline 超過 Tick → close_deadline_exceeded。
    let cfg = config();
    let s = State {
        phase: Phase::CloseTalkWait {
            talk_id: TalkId(7),
            deadline: Some(MonotonicMs(1_000)),
        },
        last_now: Some(MonotonicMs(900)),
        next_talk_id: 8,
        pending_close: None,
        choice: None,
        choice_prev_talk: None,
    };
    let ev = capture(|| {
        let _ = step(s, Input::Tick { now: MonotonicMs(2_000) }, &cfg);
    });
    assert_logged(&ev, Level::ERROR, "close_deadline_exceeded");
}

// ============================================================
// 防御 / 無視アーム（level = WARN）— mod.rs
// ============================================================

#[test]
fn warn_force_quit_logs() {
    let ev = run_step(
        Phase::BootMain,
        Input::ForceQuit {
            reason: CloseReason::System,
        },
    );
    assert_logged(&ev, Level::WARN, "force_quit");
}

#[test]
fn warn_boot_ignored_logs() {
    // 非 Idle での Boot → mod.rs boot_ignored。
    let ev = run_step(Phase::BootMain, Input::Boot);
    assert_logged(&ev, Level::WARN, "boot_ignored");
}

#[test]
fn warn_unexpected_reply_logs() {
    // 応答待ちでない Phase（Idle）への ShioriReply → mod.rs unexpected_reply。
    let ev = run_step(
        Phase::Idle,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
    );
    assert_logged(&ev, Level::WARN, "unexpected_reply");
}

#[test]
fn warn_input_after_terminate_logs() {
    // 終了系列（Stopped）で受領した非横断入力（Tick）→ dispatch_phase input_after_terminate。
    let ev = run_step(Phase::Stopped, Input::Tick { now: MonotonicMs(1_000) });
    assert_logged(&ev, Level::WARN, "input_after_terminate");
}

// ============================================================
// 防御 / 無視アーム（level = WARN）— boot.rs
// ============================================================

#[test]
fn warn_boot_input_ignored_logs() {
    // boot フェーズ（BootInit）+ boot 無関係入力（Tick）→ dispatch_phase→boot::step _ アーム。
    let ev = run_step(Phase::BootInit, Input::Tick { now: MonotonicMs(1_000) });
    assert_logged(&ev, Level::WARN, "boot_input_ignored");
}

#[test]
fn warn_boot_unexpected_reply_logs() {
    // BootInit（Notified 待ち）に Value → boot::on_reply unexpected_reply。
    let ev = run_step(
        Phase::BootInit,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("unexpected".to_string()),
            origin: "test",
        },
    );
    assert_logged(&ev, Level::WARN, "boot_unexpected_reply");
}

#[test]
fn warn_boot_reply_ignored_logs() {
    // boot::on_reply の防御 `_` アーム（応答待ちでない boot Phase）。step() 経由では
    // Idle が awaits_reply=false ゆえ mod.rs 側で握り潰され到達しない。構造上発生しない
    // 防御アームゆえ boot::step を直接駆動して検証する（唯一の網羅手段）。
    let cfg = config();
    let ev = capture(|| {
        let _ = boot::step(
            state_in(Phase::Idle),
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        );
    });
    assert_logged(&ev, Level::WARN, "boot_reply_ignored");
}

// ============================================================
// 防御 / 無視アーム（level = WARN）— steady.rs
// ============================================================

#[test]
fn warn_steady_value_during_talk_logs() {
    // Steady{Some} + 非マウス Value（DD-6 防御・narrowed）→ steady_value_during_talk。
    // origin は非マウス（OnSecondChange）——マウス origin は置換アームへ抜けて warn しない
    // ため、DD-6 破棄ログの発火には非マウス origin が必要（DD-IE-2 の意味の縮小）。
    let ev = run_step(
        steady_with_talk(TalkId(5)),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("late".to_string()),
            origin: "OnSecondChange",
        },
    );
    assert_logged(&ev, Level::WARN, "steady_value_during_talk");
}

#[test]
fn warn_steady_unexpected_reply_logs() {
    // Steady{None} + Unloaded（想定外の応答）→ steady_reply_unexpected。
    let ev = run_step(
        Phase::Steady { talk: None },
        Input::ShioriReply {
            outcome: ShioriOutcome::Unloaded,
            origin: "test",
        },
    );
    assert_logged(&ev, Level::WARN, "steady_unexpected_reply");
}

#[test]
fn warn_steady_input_ignored_logs() {
    // steady::step の `_` アーム（Steady に無関係な入力）。step() 経由では Boot は mod.rs、
    // ForceQuit/ShioriDown は横断アームで捌かれ steady へ届かない。構造上発生しない防御
    // アームゆえ steady::step を直接駆動して検証する。
    let cfg = config();
    let ev = capture(|| {
        let _ = steady::step(
            state_in(Phase::Steady { talk: None }),
            Input::Boot,
            &cfg,
        );
    });
    assert_logged(&ev, Level::WARN, "steady_input_ignored");
}

#[test]
fn warn_steady_phase_unexpected_logs() {
    // steady::step に非 Steady Phase（BootMain）が届いた場合の防御。ルーティング上
    // 到達不能ゆえ steady::step を直接駆動して検証する。
    let cfg = config();
    let ev = capture(|| {
        let _ = steady::step(
            state_in(Phase::BootMain),
            Input::Tick { now: MonotonicMs(1_000) },
            &cfg,
        );
    });
    assert_logged(&ev, Level::WARN, "steady_phase_unexpected");
}

// ============================================================
// 防御 / 無視アーム（level = WARN）— close.rs
// ============================================================

#[test]
fn warn_close_notified_unexpected_logs() {
    // ClosePending + Notified（OnClose は GET ゆえ構造上あり得ない）→ close_notified_unexpected。
    let ev = run_step(
        Phase::ClosePending {
            reason: CloseReason::User,
        },
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
    );
    assert_logged(&ev, Level::WARN, "close_notified_unexpected");
}

#[test]
fn warn_close_reply_unexpected_logs() {
    // ClosePending + Unloaded（Value/NoContent/Notified 以外）→ close_reply_unexpected。
    let ev = run_step(
        Phase::ClosePending {
            reason: CloseReason::User,
        },
        Input::ShioriReply {
            outcome: ShioriOutcome::Unloaded,
            origin: "test",
        },
    );
    assert_logged(&ev, Level::WARN, "close_reply_unexpected");
}

#[test]
fn warn_close_pending_input_ignored_logs() {
    // ClosePending + 無関係入力（CloseRequest）→ close_pending_input_ignored。
    let ev = run_step(
        Phase::ClosePending {
            reason: CloseReason::User,
        },
        Input::CloseRequest {
            reason: CloseReason::User,
        },
    );
    assert_logged(&ev, Level::WARN, "close_pending_input_ignored");
}

#[test]
fn warn_close_talk_wait_input_ignored_logs() {
    // CloseTalkWait + 無関係入力（CloseRequest）→ close_talk_wait_input_ignored。
    let ev = run_step(
        Phase::CloseTalkWait {
            talk_id: TalkId(2),
            deadline: None,
        },
        Input::CloseRequest {
            reason: CloseReason::User,
        },
    );
    assert_logged(&ev, Level::WARN, "close_talk_wait_input_ignored");
}

#[test]
fn warn_close_phase_unexpected_logs() {
    // close::step に非 close Phase（Steady）が届いた場合の防御アーム（上位 match `_`）。
    // ルーティング上到達不能ゆえ close::step を直接駆動して検証する。
    let cfg = config();
    let ev = capture(|| {
        let _ = close::step(
            state_in(Phase::Steady { talk: None }),
            Input::Tick { now: MonotonicMs(1_000) },
            &cfg,
        );
    });
    assert_logged(&ev, Level::WARN, "close_phase_unexpected");
}

// ============================================================
// 観測用ログ（level = INFO）— TalkEndReason::Interrupted の防御的非 quit 扱い
// ============================================================

// ============================================================
// 選択確定の受領検証とカスケード駆動（タスク 4.3・設計ログ語彙表）
// ============================================================

/// 任意の `State` を捕捉付きで駆動する（帳簿を直接構成する檻用）。
fn run_step_state(state: State, input: Input) -> Vec<CapturedEvent> {
    let cfg = config();
    capture(|| {
        let _ = step(state, input, &cfg);
    })
}

/// 檻用の選択確定入力。
fn choice_input_of(id: &str) -> Input {
    Input::Choice(crate::msg::ChoiceInput {
        id: id.to_string(),
        label: "メニュー".to_string(),
        scope: 0,
        references: Vec::new(),
    })
}

/// 帳簿つき `Steady{Some(5)}` を構成する。
fn state_with_ledger(candidates: &[&str], phase: ChoicePhase) -> State {
    let mut s = state_in(steady_with_talk(TalkId(5)));
    s.choice = Some(ChoiceState {
        talk_id: TalkId(5),
        candidates: candidates.iter().map(|c| c.to_string()).collect(),
        deadline: None,
        phase,
    });
    s
}

/// Req1.3: 選択待ち不在・対象 talk 不一致・非 Steady の 3 経路とも warn で記録する。
#[test]
fn warn_choice_rejected_no_wait_logs() {
    // 選択待ち帳簿が無い（解決済み・未成立）。
    let ev = run_step(steady_with_talk(TalkId(5)), choice_input_of("OnMenu"));
    assert_logged(&ev, Level::WARN, "choice_rejected_no_wait");

    // 帳簿の対象 talk が現行 talk と食い違う。
    let mut s = state_in(steady_with_talk(TalkId(5)));
    s.choice = Some(ChoiceState {
        talk_id: TalkId(999),
        candidates: vec!["OnMenu".to_string()],
        deadline: None,
        phase: ChoicePhase::Waiting,
    });
    let ev = run_step_state(s, choice_input_of("OnMenu"));
    assert_logged(&ev, Level::WARN, "choice_rejected_no_wait");

    // 非 Steady フェーズ（横断アーム側）。
    let ev = run_step(Phase::BootMain, choice_input_of("OnMenu"));
    assert_logged(&ev, Level::WARN, "choice_rejected_no_wait");
}

/// Req1.4: 候補集合に無い ID の棄却は warn で記録する。
#[test]
fn warn_choice_rejected_unknown_id_logs() {
    let ev = run_step_state(
        state_with_ledger(&["OnMenu"], ChoicePhase::Waiting),
        choice_input_of("choice9"),
    );
    assert_logged(&ev, Level::WARN, "choice_rejected_unknown_id");
}

/// Req1.1: 段の進行中（`Cascading`／`TimeoutInFlight`）の二重確定は warn で記録する。
#[test]
fn warn_choice_rejected_busy_logs() {
    for phase in [
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: None,
        },
        ChoicePhase::TimeoutInFlight,
    ] {
        let ev = run_step_state(state_with_ledger(&["OnMenu"], phase), choice_input_of("OnMenu"));
        assert_logged(&ev, Level::WARN, "choice_rejected_busy");
    }
}

/// Req2.7: `script:` 前置の明示縮退は warn 記録の上で選択解決のみを行う。
#[test]
fn warn_choice_unsupported_category_logs() {
    let ev = run_step_state(
        state_with_ledger(&["script:\\e"], ChoicePhase::Waiting),
        choice_input_of("script:\\e"),
    );
    assert_logged(&ev, Level::WARN, "choice_unsupported_category");
    // 未対応カテゴリでも選択解決は実行する（会話を止めない・Req2.7）。
    assert_logged(&ev, Level::INFO, "choice_resolved");
}

/// Req1.6: 受理は info で記録し、判定した段列をフィールドに載せる。
#[test]
fn info_choice_accepted_logs_plan() {
    let ev = run_step_state(
        state_with_ledger(&["OnMenu"], ChoicePhase::Waiting),
        choice_input_of("OnMenu"),
    );
    let accepted = logged_once(&ev, Level::INFO, "choice_accepted");
    assert_eq!(
        accepted.fields.get("choice_id").map(String::as_str),
        Some("OnMenu"),
        "確定した選択肢 ID がログフィールドに載る。\n捕捉={accepted:#?}"
    );
    assert_eq!(
        accepted.fields.get("plan").map(String::as_str),
        Some("Named"),
        "判定した段列がログフィールドに載る。\n捕捉={accepted:#?}"
    );
}

/// 各段の GET 送出は trace で記録する（設計ログ語彙表）。
#[test]
fn trace_choice_cascade_stage_logs() {
    let ev = run_step_state(
        state_with_ledger(&["choice1"], ChoicePhase::Waiting),
        choice_input_of("choice1"),
    );
    assert_logged(&ev, Level::TRACE, "choice_cascade_stage");
}

/// Req5.1: `ResolveChoice` 発行は info で記録する。
#[test]
fn info_choice_resolved_logs() {
    let ev = run_step_state(
        state_with_ledger(
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: None,
            },
        ),
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelect",
        },
    );
    assert_logged(&ev, Level::INFO, "choice_resolved");
}

/// Req4.5: カスケード段の失敗は error で記録し 204 相当で継続する。
///
/// 本檻は steady 側の 204 相当処理と error 語彙**そのもの**を層局所に固定するため
/// `steady::step` を直接駆動する。`step()` 経由の end-to-end（横断 `Failed` アームの免除＝
/// DD-12 が実際に効き `Unloading{Fault}` へ倒れないこと）は
/// `tests/kanade/choice_test.rs` の統合檻が免除の正・非 choice 経路の負の両方向で固定する。
#[test]
fn error_choice_shiori_failed_as_204_logs() {
    let cfg = config();
    let s = state_with_ledger(
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: None,
        },
    );
    let ev = capture(|| {
        let _ = steady::step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "OnChoiceSelect",
            },
            &cfg,
        );
    });
    assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
}

// ============================================================
// 選択待ち帳簿の確立・棄却（タスク 4.2・設計ログ語彙表）
// ============================================================

/// 帳簿確立は info で記録し、**候補数と期限をフィールドに載せる**（設計ログ語彙表）。
///
/// 「ログに載っていること」自体が要求であるため、発火だけでなくフィールド実値まで突合する。
#[test]
fn info_choice_waiting_established_logs_candidate_count_and_deadline() {
    let ev = run_step(
        steady_with_talk(TalkId(5)),
        Input::ChoiceWaiting {
            talk_id: TalkId(5),
            choice_ids: vec!["OnMenu".to_string(), "choice1".to_string()],
            display_end: MonotonicMs(2_000),
            timeout_directive_secs: None,
        },
    );
    let established = logged_once(&ev, Level::INFO, "choice_waiting_established");
    assert_eq!(
        established.fields.get("choice_count").map(String::as_str),
        Some("2"),
        "候補数がログフィールドに載る。\n捕捉={established:#?}"
    );
    assert_eq!(
        established.fields.get("deadline_ms").map(String::as_str),
        Some("Some(32000)"),
        "写像済みの期限がログフィールドに載る。\n捕捉={established:#?}"
    );
}

/// Req7.6: 無効化指令で確立された帳簿は、期限フィールドが無期限（`None`）として観測できる。
#[test]
fn info_choice_waiting_established_logs_indefinite_deadline() {
    let ev = run_step(
        steady_with_talk(TalkId(5)),
        Input::ChoiceWaiting {
            talk_id: TalkId(5),
            choice_ids: vec!["OnMenu".to_string()],
            display_end: MonotonicMs(2_000),
            timeout_directive_secs: Some(-1.0),
        },
    );
    let established = logged_once(&ev, Level::INFO, "choice_waiting_established");
    assert_eq!(
        established.fields.get("deadline_ms").map(String::as_str),
        Some("None"),
        "無効化指令は無期限として記録される。\n捕捉={established:#?}"
    );
}

/// 通知の棄却は 3 経路（talk_id 不一致／active talk 不在／非 Steady）とも warn で記録する。
#[test]
fn warn_choice_waiting_stale_logs() {
    let cases = [
        // 現行トークと識別子が一致しない。
        (steady_with_talk(TalkId(5)), TalkId(999)),
        // 再生中でない（active talk 不在）。
        (Phase::Steady { talk: None }, TalkId(5)),
        // 非 Steady フェーズ。
        (Phase::BootMain, TalkId(5)),
    ];
    for (phase, talk_id) in cases {
        let ev = run_step(
            phase,
            Input::ChoiceWaiting {
                talk_id,
                choice_ids: vec!["OnMenu".to_string()],
                display_end: MonotonicMs(2_000),
                timeout_directive_secs: None,
            },
        );
        assert_logged(&ev, Level::WARN, "choice_waiting_stale");
    }
}

#[test]
fn info_talk_done_interrupted_as_non_quit_logs() {
    // 既知 talk の reason=Interrupted → 防御的に非 quit 扱い（Ended と同一経路）へ委譲。
    // M1 では到達しない想定だが、到達した場合にどの reason だったか観測できることを検証する。
    let ev = run_step(
        steady_with_talk(TalkId(5)),
        Input::TalkDone(TalkDone {
            talk_id: TalkId(5),
            reason: TalkEndReason::Interrupted,
        }),
    );
    assert_logged(&ev, Level::INFO, "talk_done_interrupted_as_non_quit");
}
