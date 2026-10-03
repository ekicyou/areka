//! 運行表の翻訳のテスト（タスク 3.1）: `OnTranslate` の応答の読み（[`read_reply`] の表の全行と
//! 記録の語彙・レベル）と、応答を待っている GET の元のイベントの控え（[`before`]・[`after`]）。

use super::*;
use crate::change::ShioriMethod;
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs};
use crate::schedule::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use crate::schedule::{Phase, step};
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
