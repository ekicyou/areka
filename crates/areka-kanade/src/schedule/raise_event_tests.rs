//! 汎用の通知の入口のテスト（areka-P0-ghost-shell-balloon-switch 要件 7.1〜7.4・7.6・10.8）。
//!
//! 許可表に無いイベント・定常以外での依頼は `warn!` 1 件で捨てて動作 0、定常なら渡した
//! Reference のまま GET／NOTIFY を出すこと、その応答が再生中のトークを置き換えること
//! （`OnSecondChange` の応答は今日どおり置き換えない）を `step` 経由で固定する。

use super::*;
use crate::change::ShioriMethod;
use crate::schedule::log_capture::CapturedEvent;

fn raise(s: State, id: &str, references: &[&str], method: ShioriMethod) -> (State, Vec<Action>) {
    let input = Input::RaiseEvent {
        id: id.to_string(),
        references: references.iter().map(|r| r.to_string()).collect(),
        method,
    };
    step(s, input, &cfg())
}

fn warn_count(events: &[CapturedEvent]) -> usize {
    events.iter().filter(|e| e.level == Level::WARN).count()
}

fn steady_talking(origin: &'static str) -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id: TalkId(4),
                origin,
                script: "\\0いまの台詞\\e".to_string(),
            }),
        },
        ..steady()
    }
}

fn reply_from(s: State, origin: &'static str, script: &str) -> (State, Vec<Action>) {
    let outcome = ShioriOutcome::Value(script.to_string());
    let next = step(s, Input::ShioriReply { outcome, origin }, &cfg());
    // 翻訳の行動が出たら、`OnTranslate` が 204 を返したときの続きを返す。
    crate::schedule::translate_test_support::pass_translate(next, &cfg())
}

#[test]
fn event_not_in_allowed_table_is_dropped_with_one_warn() {
    let mut out = None;
    let ev = capture(|| out = Some(raise(steady(), "OnTalk", &["a"], ShioriMethod::Get)));
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::WARN, "raise_event_not_allowed");
    assert_eq!(warn_count(&ev), 1, "warn! はちょうど 1 件");
    assert!(actions.is_empty(), "許可表に無いイベントは送らない");
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
}

#[test]
fn request_outside_steady_is_dropped_with_one_warn_and_not_queued() {
    let phases = [
        Phase::BootMain,
        Phase::ClosePending {
            reason: CloseReason::System,
        },
        Phase::ChangePending,
        Phase::Unloading {
            cause: TermCause::Quit,
        },
    ];
    for phase in phases {
        let label = phase_label(&phase);
        let s = State { phase, ..steady() };
        let mut out = None;
        let ev = capture(|| out = Some(raise(s, "OnBoot", &["a"], ShioriMethod::Notify)));
        let (s, actions) = out.expect("step は必ず結果を返す");
        logged_once(&ev, Level::WARN, "raise_event_not_steady");
        assert_eq!(warn_count(&ev), 1, "{label}: warn! はちょうど 1 件");
        assert!(actions.is_empty(), "{label}: 定常以外では送らない");
        assert_eq!(phase_label(&s.phase), label, "{label}: 相は変えない");
        assert!(s.pending_change.is_none() && s.change.is_none() && s.pending_close.is_none());
    }
}

#[test]
fn steady_sends_get_and_notify_with_references_verbatim() {
    let refs = ["first", "", "third"];
    let (_, actions) = raise(steady(), "OnBoot", &refs, ShioriMethod::Get);
    let (id, references) = only_get(&actions);
    assert_eq!(id, "OnBoot");
    assert_eq!(references, refs, "欠番を詰めずに渡したまま");

    let (_, actions) = raise(
        steady_talking("OnSecondChange"),
        "OnBoot",
        &refs,
        ShioriMethod::Notify,
    );
    match actions.as_slice() {
        [Action::ShioriRequest(ShioriCall::Notify { id, references, .. })] => {
            assert_eq!(id.as_str(), "OnBoot");
            assert_eq!(references, &refs, "欠番を詰めずに渡したまま");
        }
        _ => panic!("NOTIFY がちょうど 1 件のはず"),
    }
}

#[test]
fn raised_event_reply_replaces_the_active_talk() {
    let (s, actions) = raise(
        steady_talking("OnSecondChange"),
        "OnBoot",
        &[],
        ShioriMethod::Get,
    );
    assert_eq!(only_get(&actions).0, "OnBoot");
    let (s, actions) = reply_from(s, "OnBoot", "\\0置き換え\\e");
    assert!(
        matches!(actions.as_slice(), [Action::StartTalk(t)] if t.talk_id == TalkId(5)),
        "再生中でも応答の台本で置き換えて再生する（捨てない・待たない）"
    );
    match s.phase {
        Phase::Steady { talk: Some(t) } => {
            assert_eq!((t.talk_id, t.origin), (TalkId(5), "OnBoot"));
        }
        _ => panic!("定常の再生中のはず"),
    }
}

#[test]
fn second_change_reply_during_talk_is_still_discarded() {
    let mut out = None;
    let ev = capture(|| {
        out = Some(reply_from(
            steady_talking("OnMouseMove"),
            "OnSecondChange",
            "\\0x\\e",
        ))
    });
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::WARN, "steady_value_during_talk");
    assert!(
        actions.is_empty(),
        "OnSecondChange の応答は今日どおり捨てる"
    );
    assert!(matches!(s.phase, Phase::Steady { talk: Some(t) } if t.talk_id == TalkId(4)));
}

/// NOTIFY の応答（`Notified`）は定常のどちらの形でも何もしない（`warn!` 0・動作 0・相もトークも不変）。
#[test]
fn notify_reply_in_steady_is_a_silent_no_op() {
    for talking in [false, true] {
        let s = if talking {
            steady_talking("OnMouseMove")
        } else {
            steady()
        };
        let (s, actions) = raise(s, "OnBoot", &["a"], ShioriMethod::Notify);
        assert_eq!(actions.len(), 1, "NOTIFY を 1 件送る");
        let mut out = None;
        let ev = capture(|| {
            let outcome = ShioriOutcome::Notified;
            let origin = "OnBoot";
            out = Some(step(s, Input::ShioriReply { outcome, origin }, &cfg()))
        });
        let (s, actions) = out.expect("step は必ず結果を返す");
        assert_eq!(warn_count(&ev), 0, "talking={talking}: warn! は出さない");
        assert!(actions.is_empty(), "talking={talking}: 動作 0");
        match (talking, s.phase) {
            (false, Phase::Steady { talk: None }) => {}
            (true, Phase::Steady { talk: Some(t) }) => {
                assert_eq!((t.talk_id, t.origin), (TalkId(4), "OnMouseMove"));
            }
            _ => panic!("talking={talking}: 定常のまま・トークは不変のはず"),
        }
    }
}
