//! `\![get,property]` の返しのイベントの運行表の腕のテスト（shell-scale-linkage 要件 1.2・1.3・1.7・2.1〜2.3・7.1）。
//!
//! 定常でだけ逐語の名前の GET を 1 つ出し（Reference は欠番を詰めず台本の順）、起動の段・終了の段と
//! 名前か Reference の検査に落ちたものは `warn!` 1 件で捨てて動作 0 であること、応答の台本が
//! ほかのイベントの応答と同じ規則で再生され、話している最中なら今の台詞を置き換えることを
//! `step` 経由で固定する。

use super::*;
use crate::msg::EventId;
use crate::schedule::events::PROPERTY_EVENT_ORIGIN;
use crate::schedule::translate_test_support::strings;
use crate::talk::ScriptOrigin;

fn property(s: State, id: &str, references: &[&str]) -> (State, Vec<Action>) {
    let input = Input::PropertyEvent {
        id: id.to_string(),
        references: strings(references),
    };
    step(s, input, &cfg())
}

fn steady_talking() -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id: TalkId(4),
                origin: "OnMouseClick",
                script: "\\0いまの台詞\\e".to_string(),
            }),
        },
        ..steady()
    }
}

/// 捨てる場面: `warn!` は `event` の 1 件だけ・動作 0・相は変えない。
fn assert_dropped(s: State, id: &str, references: &[&str], event: &str) {
    let label = phase_label(&s.phase);
    let mut out = None;
    let ev = capture(|| out = Some(property(s, id, references)));
    let (s, actions) = out.expect("step は必ず結果を返す");
    logged_once(&ev, Level::WARN, event);
    assert_eq!(warn_count(&ev), 1, "{label} {id:?}: warn! はちょうど 1 件");
    assert!(actions.is_empty(), "{label} {id:?}: 送らない");
    assert_eq!(phase_label(&s.phase), label, "{label}: 相は変えない");
}

#[test]
fn steady_sends_one_get_with_verbatim_name_and_references_in_script_order() {
    let refs = ["150,150", "", "144"];
    let (s, actions) = property(steady(), "OnPastaShellScale", &refs);
    match actions.as_slice() {
        [Action::ShioriRequest(ShioriCall::Get { id, references, .. })] => {
            assert_eq!(id, &EventId::Property("OnPastaShellScale".to_string()));
            assert_eq!(references, &refs, "欠番を詰めずに台本の順のまま");
        }
        _ => panic!("GET がちょうど 1 件のはず"),
    }
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
}

#[test]
fn name_not_starting_with_on_is_sent_verbatim() {
    let (_, actions) = property(steady(), "PastaShellScale", &["100,100"]);
    let (id, references) = only_get(&actions);
    assert_eq!(
        (id.as_str(), references.as_slice()),
        ("PastaShellScale", &["100,100".to_string()][..])
    );
}

#[test]
fn boot_and_shutdown_phases_drop_with_one_warn() {
    let phases = [
        Phase::BootMain,
        Phase::ClosePending,
        Phase::Unloading {
            cause: TermCause::Quit,
        },
    ];
    for phase in phases {
        let s = State { phase, ..steady() };
        assert_dropped(s, "OnX", &["a"], "property_event_not_steady");
    }
}

#[test]
fn raise_and_property_events_are_dropped_while_close_or_change_is_pending() {
    // 終了・切替の保留中は、汎用の通知の入口も返しの入口も送らずに捨てる（要件 9.3）。
    let pending = |reason: &str| match reason {
        "close_pending" => State {
            pending_close: Some(CloseReason::System),
            ..steady_talking()
        },
        _ => State {
            pending_change: Some(change_request()),
            ..steady_talking()
        },
    };
    for reason in ["close_pending", "change_pending"] {
        let raise = Input::RaiseEvent {
            id: "OnBoot".to_string(),
            references: vec!["a".to_string()],
            method: crate::change::ShioriMethod::Notify,
        };
        let entries = [
            (raise, "raise_event_not_steady"),
            (
                Input::PropertyEvent {
                    id: "OnX".to_string(),
                    references: vec!["a".to_string()],
                },
                "property_event_not_steady",
            ),
        ];
        for (input, event) in entries {
            let mut out = None;
            let ev = capture(|| out = Some(step(pending(reason), input, &cfg())));
            let (_, actions) = out.expect("step は必ず結果を返す");
            let line = logged_once(&ev, Level::WARN, event);
            assert_eq!(
                warn_count(&ev),
                1,
                "{reason} {event}: warn! はちょうど 1 件"
            );
            assert_eq!(
                line.fields.get("reason").map(String::as_str),
                Some(reason),
                "{event}: 理由を記録に載せる"
            );
            assert!(actions.is_empty(), "{reason} {event}: 送らない");
        }
    }
}

#[test]
fn invalid_name_or_reference_drops_with_one_warn() {
    for (id, refs) in [
        ("", &["a"][..]),
        (" \t", &["a"][..]),
        ("On\rX", &[][..]),
        ("On\nX", &[][..]),
        ("On\0X", &[][..]),
        ("OnX", &["a\r"][..]),
        ("OnX", &["", "a\nb"][..]),
        ("OnX", &["a\0"][..]),
    ] {
        assert_dropped(steady(), id, refs, "property_event_invalid");
    }
}

#[test]
fn reply_plays_like_other_events_and_replaces_the_active_talk() {
    // 黙っているとき: 応答の台本を新しいトークとして再生する（出どころは internal,property）。
    let (s, actions) = property(steady(), "OnX", &[]);
    assert_eq!(only_get(&actions).0, "OnX");
    let (s, actions) = reply_from(s, "\\0返し\\e");
    match actions.as_slice() {
        [Action::StartTalk(t)] => {
            assert_eq!(t.talk_id, TalkId(5));
            assert_eq!(t.origin, ScriptOrigin::PROPERTY);
        }
        _ => panic!("再生の開始がちょうど 1 件のはず"),
    }
    assert!(
        matches!(s.phase, Phase::Steady { talk: Some(ref t) } if t.origin == PROPERTY_EVENT_ORIGIN)
    );

    // 話している最中: 今の台詞を置き換えて再生する（捨てない・待たない）。
    let (s, actions) = property(steady_talking(), "OnX", &[]);
    assert_eq!(only_get(&actions).0, "OnX");
    let (s, actions) = reply_from(s, "\\0置き換え\\e");
    assert!(
        matches!(actions.as_slice(), [Action::StartTalk(t)] if t.talk_id == TalkId(5)),
        "再生中でも応答の台本で置き換えて再生する"
    );
    match s.phase {
        Phase::Steady { talk: Some(t) } => {
            assert_eq!((t.talk_id, t.origin), (TalkId(5), PROPERTY_EVENT_ORIGIN));
        }
        _ => panic!("定常の再生中のはず"),
    }
}

/// 殻が返しのイベントの往復に載せる名札で応答を入れ直す（翻訳は 204 で素通し）。
fn reply_from(s: State, script: &str) -> (State, Vec<Action>) {
    let outcome = ShioriOutcome::Value(script.to_string(), areka_talk::SecurityLevel::Local);
    let origin = PROPERTY_EVENT_ORIGIN;
    let next = step(s, Input::ShioriReply { outcome, origin }, &cfg());
    pass_translate(next, &cfg())
}
