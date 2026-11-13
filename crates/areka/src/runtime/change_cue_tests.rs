//! `ChangeCueSink` の自己選別と送り出しの決定論テスト（要件 1.2・1.3・1.10・8.5）。
//!
//! 確かめること: `\![change,ghost,名]` が名前を無変形で要求 1 件（`raise_event` 偽）にすること・
//! `--option=raise-event` で真になること・`shell`／`balloon`／裸の `change`／他の名前が 0 件で
//! 警告も出さないこと・名前なしが `warn!` 1 件で 0 件・知らない option が `warn!` 1 件で要求は出す
//! こと・受信端が落ちていても落ちずに記録すること。

use super::*;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};
use std::sync::mpsc::{Receiver, channel};

// ---------------------------------------------------------------- 道具立て

/// `\![name,tokens...]` の汎用キャリア cue を組む。
fn carrier_cue(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

fn sink() -> (ChangeCueSink, Receiver<ChangeRequestRaw>) {
    let (tx, rx) = channel();
    (ChangeCueSink::new(tx), rx)
}

/// 1 つの cue を流し、送られた要求と記録を返す。
fn emit_one(name: &str, tokens: &[&str]) -> (Vec<ChangeRequestRaw>, Vec<CapturedEvent>) {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| sink.emit(carrier_cue(name, tokens)));
    (rx.try_iter().collect(), events)
}

fn warns(events: &[CapturedEvent]) -> Vec<Option<&str>> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| e.field_str("event"))
        .collect()
}

fn raw(name: &str, raise_event: bool) -> ChangeRequestRaw {
    ChangeRequestRaw {
        name: name.to_owned(),
        raise_event,
    }
}

// ---------------------------------------------------------------- 受理

/// `\![change,ghost,名]` は名前を無変形で運ぶ要求 1 件（`OnGhostChanging` は送らない・要件 1.2）。
#[test]
fn change_ghost_with_name_sends_one_request_without_raise_event() {
    let (sent, events) = emit_one("change", &["ghost", "Emily/Phase 4.5"]);
    assert_eq!(sent, vec![raw("Emily/Phase 4.5", false)], "名前は無変形");
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// `--option=raise-event` 付きは `raise_event` 真（要件 1.3）。
#[test]
fn raise_event_option_sets_raise_event() {
    let (sent, events) = emit_one("change", &["ghost", "B", "--option=raise-event"]);
    assert_eq!(sent, vec![raw("B", true)]);
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// 知らない option は `warn!` 1 件で無視し、要求は出す。
#[test]
fn unknown_option_warns_and_still_sends() {
    let (sent, events) = emit_one("change", &["ghost", "B", "--option=unknown"]);
    assert_eq!(
        sent,
        vec![raw("B", false)],
        "知らない option は切替を止めない"
    );
    assert_eq!(warns(&events), vec![Some("change_cue_unknown_option")]);
}

// ---------------------------------------------------------------- 名前なし

/// 名前の無い `\![change,ghost]`（空の名前も）は `warn!` 1 件で 0 件。
#[test]
fn missing_name_warns_and_sends_nothing() {
    for tokens in [&["ghost"][..], &["ghost", ""][..]] {
        let (sent, events) = emit_one("change", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 名前なしは送らない");
        assert_eq!(
            warns(&events),
            vec![Some("change_ghost_no_name")],
            "{tokens:?}: 名前なしは警告 1 件"
        );
    }
}

// ---------------------------------------------------------------- 担当外

/// `shell`／`balloon`・裸の `change`・他の名前は 0 件で警告も出さない（要件 1.10・8.5）。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 4] = [
        ("change", &["shell", "S"]),
        ("change", &["balloon", "X", "--option=raise-event"]),
        ("change", &[]),
        ("open", &["ghost", "B"]),
    ];
    for (name, tokens) in cases {
        let (sent, events) = emit_one(name, tokens);
        assert!(sent.is_empty(), "{name} {tokens:?}: 担当外は送らない");
        assert!(warns(&events).is_empty(), "{name} {tokens:?}: 警告しない");
    }
}

/// キャリアでない cue は送らない。
#[test]
fn non_carrier_cue_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("アヒル".into()),
        duration: 0.0,
    });
    assert_eq!(rx.try_iter().count(), 0);
}

/// 開封できない自分宛（`change`）の荷物は警告を残し、送らない。
#[test]
fn unopenable_change_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: "change".into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0);
    assert_eq!(warns(&events), vec![Some("change_cue_unopenable")]);
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) = capture(|| sink.emit(carrier_cue("change", &["ghost", "B"])));
    assert_eq!(warns(&events), vec![Some("change_cue_send_failed")]);
}
