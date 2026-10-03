//! 運行表の翻訳のテスト（タスク 3.1・3.3）: `OnTranslate` の応答の読み（[`read_reply`] の表の全行と
//! 記録の語彙・レベル）と、応答を待っている GET の元のイベントの控え（[`before`]・[`after`]）と、
//! 帳簿を直接置いた状態からの翻訳の結果の入力（[`on_done`]）。

use super::*;
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget, ShioriMethod};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriFaultKind};
use crate::schedule::change::ChangeState;
use crate::schedule::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use crate::schedule::translate_test_support::pass_translate;
use crate::schedule::{ActiveTalk, Phase, TermCause, step};
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use crate::talk::{EpilogueCommand, StartTalk, TalkId};
use tracing::Level;

fn cfg() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn choice_source() -> EventId {
    EventId::Choice("OnおやつSelect".to_string())
}

/// 捕捉つきで読み、返り値と記録を返す。
fn read(outcome: ShioriOutcome) -> (ReplyReading, Vec<CapturedEvent>) {
    let mut out = None;
    let ev = capture(|| {
        out = Some(read_reply(
            &choice_source(),
            "展開済みの元の台詞".to_string(),
            outcome,
        ))
    });
    (out.expect("read_reply は必ず値を返す"), ev)
}

/// `translate_reply` がちょうど 1 件・指定のレベルで、元のイベントの ID と種類を載せていること。
fn assert_reply_logged<'a>(ev: &'a [CapturedEvent], level: Level, kind: &str) -> &'a CapturedEvent {
    let hit = logged_once(ev, level, "translate_reply");
    assert_eq!(hit.fields.get("kind").map(String::as_str), Some(kind));
    assert_eq!(
        hit.fields.get("source").map(String::as_str),
        Some("OnおやつSelect"),
        "元のイベントの ID（選択肢の任意名は逐語）を載せる"
    );
    hit
}

fn proceeded(reading: ReplyReading) -> String {
    match reading {
        ReplyReading::Proceed(script) => script,
        ReplyReading::Failed(failure) => panic!("進むはずが輸送路の失敗になった: {failure}"),
    }
}

// ============================================================
// 応答の読みの表（design「read_reply の表」の全行）
// ============================================================

#[test]
fn value_with_text_is_adopted_as_replaced() {
    let (reading, ev) = read(ShioriOutcome::Value("\\0翻訳した台詞\\e".to_string()));
    assert_eq!(proceeded(reading), "\\0翻訳した台詞\\e");
    assert_reply_logged(&ev, Level::INFO, "replaced");
}

#[test]
fn empty_value_is_adopted_as_empty() {
    let (reading, ev) = read(ShioriOutcome::Value(String::new()));
    assert_eq!(
        proceeded(reading),
        "",
        "200 の空は空の台詞を採る（要件 4.2）"
    );
    assert_reply_logged(&ev, Level::INFO, "empty");
}

#[test]
fn no_content_keeps_the_expanded_script() {
    let (reading, ev) = read(ShioriOutcome::NoContent);
    assert_eq!(proceeded(reading), "展開済みの元の台詞");
    assert_reply_logged(&ev, Level::INFO, "no_content");
}

#[test]
fn error_response_keeps_the_expanded_script_with_one_warning() {
    let (reading, ev) = read(ShioriOutcome::Failed(ShioriFailure::Shiori(
        "500 Internal Server Error".to_string(),
    )));
    assert_eq!(proceeded(reading), "展開済みの元の台詞");
    let hit = assert_reply_logged(&ev, Level::WARN, "error_response");
    assert_eq!(
        hit.fields.get("error").map(String::as_str),
        Some("500 Internal Server Error"),
        "警告にエラー応答の内容を載せる"
    );
    let warns = ev.iter().filter(|e| e.level == Level::WARN).count();
    assert_eq!(warns, 1, "エラー応答の警告はこの 1 件だけ");
}

#[test]
fn transport_failures_are_returned_without_a_reply_record() {
    let failures = [
        ShioriFailure::Handshake("つながらない".to_string()),
        ShioriFailure::Timeout("期限切れ".to_string()),
        ShioriFailure::Ipc("通信が切れた".to_string()),
        ShioriFailure::Internal("内部".to_string()),
    ];
    for failure in failures {
        let label = failure.to_string();
        let (reading, ev) = read(ShioriOutcome::Failed(failure));
        assert!(
            matches!(reading, ReplyReading::Failed(ref f) if f.to_string() == label),
            "輸送路の失敗はそのまま返す: {label}"
        );
        // 記録は運行表が結果を受けたときの translate_failed が受け持つ（重ねて出さない）。
        assert_not_logged(&ev, "translate_reply");
    }
}

#[test]
fn results_that_get_never_produces_are_treated_as_no_content_with_a_warning() {
    for outcome in [ShioriOutcome::Notified, ShioriOutcome::Unloaded] {
        let (reading, ev) = read(outcome);
        assert_eq!(proceeded(reading), "展開済みの元の台詞");
        assert_reply_logged(&ev, Level::WARN, "unexpected");
    }
}

// ============================================================
// 元のイベントの控え（State::reply_source）
// ============================================================

fn steady() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(500)),
        ..State::initial()
    }
}

fn stale_source() -> SourceEvent {
    SourceEvent {
        id: EventId::Static("OnSecondChange"),
        references: vec!["古い".to_string()],
    }
}

fn raise(s: State, method: ShioriMethod) -> (State, Vec<Action>) {
    let input = Input::RaiseEvent {
        id: "OnBoot".to_string(),
        references: vec!["a".to_string(), String::new()],
        method,
    };
    step(s, input, &cfg())
}

#[test]
fn get_at_the_end_of_the_batch_is_noted_with_its_references() {
    let (s, actions) = raise(steady(), ShioriMethod::Get);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        s.reply_source,
        Some(SourceEvent {
            id: EventId::Static("OnBoot"),
            references: vec!["a".to_string(), String::new()],
        })
    );
}

#[test]
fn notify_at_the_end_of_the_batch_clears_the_note() {
    let s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let (s, _) = raise(s, ShioriMethod::Notify);
    assert_eq!(s.reply_source, None);
}

#[test]
fn unload_at_the_end_of_the_batch_clears_the_note() {
    // 強制終了の一括は [NOTIFY, 降ろす往復]。最後の往復は降ろす往復。
    let s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let (s, actions) = step(
        s,
        Input::ForceQuit {
            reason: CloseReason::System,
        },
        &cfg(),
    );
    assert!(matches!(actions.last(), Some(Action::ShioriUnload)));
    assert_eq!(s.reply_source, None);
}

#[test]
fn batch_without_round_trip_leaves_the_note_alone() {
    let s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let (s, actions) = step(
        s,
        Input::ExecutionState(crate::status::ExecutionStateUpdate::NoUserBreak(true)),
        &cfg(),
    );
    assert!(actions.is_empty());
    assert_eq!(s.reply_source, Some(stale_source()));
}

#[test]
fn reply_takes_the_note_only_once() {
    let mut s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let reply = Input::ShioriReply {
        outcome: ShioriOutcome::NoContent,
        origin: "OnSecondChange",
    };
    assert_eq!(before(&mut s, &reply), Some(stale_source()));
    assert_eq!(before(&mut s, &reply), None, "控えは 1 回だけ使う");
}

#[test]
fn non_reply_input_does_not_take_the_note() {
    let mut s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let input = Input::Tick {
        now: MonotonicMs(600),
    };
    assert_eq!(before(&mut s, &input), None);
    assert_eq!(s.reply_source, Some(stale_source()));
}

#[test]
fn reply_with_no_round_trip_leaves_the_note_empty() {
    let s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnSecondChange",
        },
        &cfg(),
    );
    assert!(actions.is_empty());
    assert_eq!(s.reply_source, None, "応答の入力で取り出した控えは戻さない");
}

#[test]
fn reply_whose_batch_sends_the_next_get_notes_that_get() {
    // 起動: OnInitialize（NOTIFY）の応答の一括は username の照会（GET）。
    let (s, _) = step(State::initial(), Input::Boot, &cfg());
    assert_eq!(s.reply_source, None, "OnInitialize は NOTIFY");
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "OnInitialize",
        },
        &cfg(),
    );
    assert!(matches!(
        actions.last(),
        Some(Action::ShioriRequest(ShioriCall::Get { .. }))
    ));
    assert_eq!(
        s.reply_source,
        Some(SourceEvent {
            id: EventId::Static("username"),
            references: Vec::new(),
        })
    );
}

#[test]
fn translate_at_the_end_of_the_batch_clears_the_note() {
    let mut s = State {
        reply_source: Some(stale_source()),
        ..steady()
    };
    let actions = after(&mut s, vec![Action::Translate(request(ORIGINAL))]);
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    assert_eq!(
        s.reply_source, None,
        "翻訳の依頼は往復なので、その前の GET の控えを残さない"
    );
}

// ============================================================
// 翻訳の結果の入力（on_done）——帳簿を直接置いた状態から
// ============================================================

const TALK: TalkId = TalkId(3);
const ORIGINAL: &str = "\\0元の台詞\\e";
const FINAL: &str = "\\0最終の台詞\\e";

fn request(script: &str) -> TranslateRequest {
    TranslateRequest {
        script: script.to_string(),
        source: SourceEvent {
            id: EventId::Static("OnBoot"),
            references: Vec::new(),
        },
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

fn active(talk_id: TalkId) -> ActiveTalk {
    ActiveTalk {
        talk_id,
        origin: "OnBoot",
        script: ORIGINAL.to_string(),
    }
}

/// 起動の記録の後ろ書きつきの再生の開始（後ろ書きは差し替えの対象ではない）。
fn start_with_epilogue() -> StartTalk {
    StartTalk {
        talk_id: TALK,
        script: ORIGINAL.to_string(),
        epilogue: vec![EpilogueCommand {
            name: "boot".to_string(),
            tokens: vec!["記録".to_string()],
        }],
    }
}

/// `phase` に帳簿（`deferred` を預けた・トーク [`TALK`]・元のイベント `OnBoot`）を置いた状態。
fn waiting(phase: Phase, deferred: Vec<Action>) -> State {
    State {
        phase,
        translate: Some(TranslateWait {
            talk_id: TALK,
            source: EventId::Static("OnBoot"),
            deferred,
        }),
        ..State::initial()
    }
}

fn change_state(script: &str) -> ChangeState {
    ChangeState {
        req: ChangeRequest {
            target: ChangeTarget {
                sakura_name: "B".to_string(),
                name: "ghost B".to_string(),
                dir: "C:/ghost/B".to_string(),
            },
            origin: ChangeOrigin::Manual,
            raise_event: true,
        },
        script: Some(script.to_string()),
    }
}

/// 最上位の `step` に翻訳の結果を入れ、返り値と記録を返す。
fn done(s: State, result: TranslateResult) -> (State, Vec<Action>, Vec<CapturedEvent>) {
    let mut out = None;
    let ev = capture(|| out = Some(step(s, Input::TranslateDone(result), &cfg())));
    let (s, actions) = out.expect("step は必ず値を返す");
    (s, actions, ev)
}

fn active_script(phase: &Phase) -> &str {
    match phase {
        Phase::Steady { talk: Some(t) } | Phase::BootVersion { talk: Some(t) } => &t.script,
        _ => panic!("再生中の相のはず"),
    }
}

fn change_script(s: State) -> Option<String> {
    s.change.expect("切替の帳簿は残る").script
}

#[test]
fn done_ok_replaces_only_the_script_and_returns_the_boot_batch_in_order() {
    let notify = Action::ShioriRequest(super::super::events::baseware_version(
        &cfg(),
        &ExecutionSnapshot::INACTIVE,
    ));
    let s = waiting(
        Phase::BootVersion {
            talk: Some(active(TALK)),
        },
        vec![Action::StartTalk(start_with_epilogue()), notify],
    );
    let (s, actions, ev) = done(s, Ok(FINAL.to_string()));

    match actions.as_slice() {
        [
            Action::StartTalk(start),
            Action::ShioriRequest(ShioriCall::Notify { id, .. }),
        ] => {
            assert_eq!(start.talk_id, TALK);
            assert_eq!(start.script, FINAL, "台詞だけを最終の台詞に差し替える");
            assert_eq!(
                start.epilogue,
                start_with_epilogue().epilogue,
                "起動の記録の後ろ書きは触らない"
            );
            assert_eq!(id.as_str(), "basewareversion", "順序を保って返す");
        }
        _ => panic!("預けた一括 [StartTalk, NOTIFY] が返るはず"),
    }
    assert!(s.translate.is_none(), "帳簿は空になる");
    assert_eq!(
        active_script(&s.phase),
        FINAL,
        "再生中の台詞の控えを書き換える"
    );
    let hit = logged_once(&ev, Level::DEBUG, "translate_resume");
    assert_eq!(hit.fields.get("talk_id").map(String::as_str), Some("3"));
    assert_eq!(hit.fields.get("changed").map(String::as_str), Some("true"));
}

#[test]
fn done_ok_keeps_the_choice_chain_order_and_rewrites_the_steady_talk() {
    let s = waiting(
        Phase::Steady {
            talk: Some(active(TALK)),
        },
        vec![
            Action::ResolveChoice {
                talk_id: TalkId(2),
                id: "OnおやつSelect".to_string(),
            },
            Action::StartTalk(StartTalk::new(TALK, ORIGINAL)),
        ],
    );
    let (s, actions, ev) = done(s, Ok(FINAL.to_string()));
    match actions.as_slice() {
        [Action::ResolveChoice { .. }, Action::StartTalk(start)] => {
            assert_eq!(start.script, FINAL)
        }
        _ => panic!("預けた一括 [ResolveChoice, StartTalk] が順序どおり返るはず"),
    }
    assert_eq!(active_script(&s.phase), FINAL);
    logged_once(&ev, Level::DEBUG, "translate_resume");
}

#[test]
fn done_ok_with_the_same_script_reports_unchanged() {
    let s = waiting(
        Phase::Steady {
            talk: Some(active(TALK)),
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    let (_, _, ev) = done(s, Ok(ORIGINAL.to_string()));
    let hit = logged_once(&ev, Level::DEBUG, "translate_resume");
    assert_eq!(
        hit.fields.get("changed").map(String::as_str),
        Some("false"),
        "最終の台詞が SHIORI の返した台詞と同じなら changed=false"
    );
}

#[test]
fn done_ok_does_not_rewrite_a_talk_with_another_id() {
    let s = waiting(
        Phase::Steady {
            talk: Some(active(TalkId(9))),
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    let (s, _, _) = done(s, Ok(FINAL.to_string()));
    assert_eq!(
        active_script(&s.phase),
        ORIGINAL,
        "talk_id が違う控えは書き換えない"
    );
}

#[test]
fn done_ok_rewrites_the_change_script_while_ghost_changing_talks() {
    let mut s = waiting(
        Phase::ChangeTalkWait {
            talk_id: TALK,
            deadline: None,
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    s.change = Some(change_state(ORIGINAL));
    let (s, _, _) = done(s, Ok(FINAL.to_string()));
    assert_eq!(
        change_script(s).as_deref(),
        Some(FINAL),
        "OnGhostChanging の台詞の控え（切替の中身）を最終の台詞にする"
    );
}

#[test]
fn done_ok_keeps_the_change_script_for_the_farewell_on_close() {
    let mut s = waiting(
        Phase::ChangeCloseTalkWait {
            talk_id: TALK,
            deadline: None,
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    s.change = Some(change_state("\\0切替の台詞\\e"));
    let (s, actions, _) = done(s, Ok(FINAL.to_string()));
    assert!(matches!(actions.as_slice(), [Action::StartTalk(start)] if start.script == FINAL));
    assert_eq!(
        change_script(s).as_deref(),
        Some("\\0切替の台詞\\e"),
        "切替の OnClose の台詞では切替の台詞を書き換えない"
    );
}

#[test]
fn done_err_drops_the_batch_and_goes_to_fault() {
    let s = waiting(
        Phase::Steady {
            talk: Some(active(TALK)),
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    let (s, actions, ev) = done(s, Err(ShioriFailure::Ipc("通信が切れた".to_string())));
    assert!(
        matches!(actions.as_slice(), [Action::ShioriUnload]),
        "預けた一括は捨て、降ろす往復だけを返す"
    );
    assert!(
        matches!(
            &s.phase,
            Phase::Unloading {
                cause: TermCause::Fault(fault),
            } if fault.kind == ShioriFaultKind::Disconnected
        ),
        "既存の故障の遷移（Unloading の Fault）へ"
    );
    assert!(s.translate.is_none());
    let hit = logged_once(&ev, Level::ERROR, "translate_failed");
    assert_eq!(hit.fields.get("source").map(String::as_str), Some("OnBoot"));
    assert_eq!(hit.fields.get("talk_id").map(String::as_str), Some("3"));
    assert_not_logged(&ev, "translate_resume");
}

#[test]
fn done_err_empties_the_change_script_while_ghost_changing_talks() {
    let mut s = waiting(
        Phase::ChangeTalkWait {
            talk_id: TALK,
            deadline: None,
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    s.change = Some(change_state(ORIGINAL));
    let (s, actions, _) = done(s, Err(ShioriFailure::Timeout("期限切れ".to_string())));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    assert!(matches!(s.phase, Phase::Unloading { .. }));
    assert_eq!(
        change_script(s),
        None,
        "表示しなかった台詞を切替の中身として次のゴーストへ渡さない"
    );
}

#[test]
fn done_without_a_ledger_is_dropped_with_a_warning() {
    let s = State {
        phase: Phase::Steady {
            talk: Some(active(TALK)),
        },
        ..State::initial()
    };
    let (s, actions, ev) = done(s, Ok(FINAL.to_string()));
    assert!(actions.is_empty(), "帳簿の無い結果は捨てる");
    assert_eq!(active_script(&s.phase), ORIGINAL, "控えも変えない");
    logged_once(&ev, Level::WARN, "translate_done_unexpected");
}

#[test]
fn pass_translate_reinputs_the_script_as_is_and_returns_the_continuation() {
    let s = waiting(
        Phase::Steady {
            talk: Some(active(TALK)),
        },
        vec![Action::StartTalk(StartTalk::new(TALK, ORIGINAL))],
    );
    let (s, actions) = pass_translate((s, vec![Action::Translate(request(ORIGINAL))]), &cfg());
    assert!(matches!(actions.as_slice(), [Action::StartTalk(start)] if start.script == ORIGINAL));
    assert!(s.translate.is_none());

    // 翻訳の行動だけの一括でなければ、そのまま返す。
    let (_, actions) = pass_translate((s, vec![Action::ShioriUnload]), &cfg());
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
}
