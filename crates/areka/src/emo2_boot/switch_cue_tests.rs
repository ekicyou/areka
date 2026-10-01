//! `SwitchCueSink` の自己選別と送り出しの決定論テスト（要件 1.2・1.3・1.4・1.15）。
//!
//! 確かめること: `\![change,shell,名]` が名前を無変形で要求 1 件（`raise_event` 偽）にすること・
//! `--option=raise-event` で真になること・バルーンの `raise-event` は `warn!` 1 件で要求は偽で出す
//! こと・`ghost`／裸の `change`／他の名前が 0 件で警告も出さないこと・名前なしが `warn!` 1 件で
//! 0 件・受信端が落ちていても落ちずに記録すること。

use super::*;
use crate::emo2_boot::shell_balloon_switch::SkinKind;
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

fn sink() -> (SwitchCueSink, Receiver<SkinRequestRaw>) {
    let (tx, rx) = channel();
    (SwitchCueSink::new(tx), rx)
}

/// 1 つの cue を流し、送られた要求と記録を返す。
fn emit_one(name: &str, tokens: &[&str]) -> (Vec<SkinRequestRaw>, Vec<CapturedEvent>) {
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

fn raw(kind: SkinKind, name: &str, raise_event: bool) -> SkinRequestRaw {
    SkinRequestRaw {
        kind,
        name: name.to_owned(),
        raise_event,
    }
}

// ---------------------------------------------------------------- シェル

/// `\![change,shell,名]` は名前を無変形で運ぶ要求 1 件（`OnShellChanging` は送らない・要件 1.2）。
#[test]
fn change_shell_with_name_sends_one_request_without_raise_event() {
    let (sent, events) = emit_one("change", &["shell", "Shell B/2"]);
    assert_eq!(
        sent,
        vec![raw(SkinKind::Shell, "Shell B/2", false)],
        "名前は無変形"
    );
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// シェルの `--option=raise-event` 付きは `raise_event` 真（要件 1.3）。
#[test]
fn shell_raise_event_option_sets_raise_event() {
    let (sent, events) = emit_one("change", &["shell", "B", "--option=raise-event"]);
    assert_eq!(sent, vec![raw(SkinKind::Shell, "B", true)]);
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// シェルの知らない option は `warn!` 1 件で無視し、要求は出す。
#[test]
fn shell_unknown_option_warns_and_still_sends() {
    let (sent, events) = emit_one("change", &["shell", "B", "--option=unknown"]);
    assert_eq!(sent, vec![raw(SkinKind::Shell, "B", false)]);
    assert_eq!(warns(&events), vec![Some("switch_cue_unknown_option")]);
}

// ---------------------------------------------------------------- バルーン

/// `\![change,balloon,名]` はバルーンの要求 1 件（警告なし・要件 1.4）。
#[test]
fn change_balloon_with_name_sends_one_request() {
    let (sent, events) = emit_one("change", &["balloon", "Balloon Y"]);
    assert_eq!(sent, vec![raw(SkinKind::Balloon, "Balloon Y", false)]);
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// バルーンの `raise-event` は効かない——`warn!` 1 件を残し、要求は偽で出す（要件 1.4）。
#[test]
fn balloon_raise_event_warns_and_sends_without_raise_event() {
    let (sent, events) = emit_one("change", &["balloon", "Y", "--option=raise-event"]);
    assert_eq!(sent, vec![raw(SkinKind::Balloon, "Y", false)]);
    assert_eq!(warns(&events), vec![Some("switch_cue_unknown_option")]);
}

// ---------------------------------------------------------------- 名前なし

/// 名前の無い `\![change,shell]`・`\![change,balloon]`（空の名前も）は `warn!` 1 件で 0 件。
#[test]
fn missing_name_warns_and_sends_nothing() {
    for tokens in [
        &["shell"][..],
        &["shell", ""][..],
        &["balloon"][..],
        &["balloon", ""][..],
    ] {
        let (sent, events) = emit_one("change", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 名前なしは送らない");
        assert_eq!(
            warns(&events),
            vec![Some("switch_cue_no_name")],
            "{tokens:?}: 名前なしは警告 1 件"
        );
    }
}

// ---------------------------------------------------------------- 担当外

/// `ghost`・裸の `change`・他の名前は 0 件で警告も出さない（`(change,ghost)` は `ChangeCueSink` の
/// 持ち場・要件 1.15）。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 4] = [
        ("change", &["ghost", "G", "--option=raise-event"]),
        ("change", &[]),
        ("open", &["shell", "B"]),
        ("change", &["Shell", "B"]),
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
    let ((), events) = capture(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("アヒル".into()),
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0);
    assert!(warns(&events).is_empty());
}

/// 開封できない `change` の荷物は送らない（自分宛か分からないので `debug!` で見送る——警告は
/// 同じ名前の `ChangeCueSink` が 1 件残す）。
#[test]
fn unopenable_change_is_skipped_without_warning() {
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
    assert!(warns(&events).is_empty());
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する（受信端を結ぶまでの本番もこの形）。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) = capture(|| sink.emit(carrier_cue("change", &["balloon", "Y"])));
    assert_eq!(warns(&events), vec![Some("switch_cue_send_failed")]);
}
