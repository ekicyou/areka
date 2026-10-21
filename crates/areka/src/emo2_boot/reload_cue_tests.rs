//! `ReloadCueSink` の自己選別と送り出しの決定論テスト（要件 1.4・1.5・10.3）。
//!
//! 確かめること: 拾う 7 綴り（`\![reload,ghost|shiori|shell|balloon|descript(,パラメータ)]`・
//! `\![reloadsurface]`）が `parse_script` の写しのまま 1 件送られること・`\![reload,makoto]`／
//! `\![reload,aigraph]`／引数なしの `\![reload]`／他の名前が 0 件で警告も出さないこと・
//! 受信端が落ちていても落ちずに記録すること。

use super::*;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};
use std::sync::mpsc::{Receiver, channel};

use super::super::reload_args::{DescriptNotes, DescriptParts, ReloadTarget};

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

fn sink() -> (ReloadCueSink, Receiver<ScriptReload>) {
    let (tx, rx) = channel();
    (ReloadCueSink::new(tx), rx)
}

/// 1 つの cue を流し、送られた要求と記録を返す。
fn emit_one(name: &str, tokens: &[&str]) -> (Vec<ScriptReload>, Vec<CapturedEvent>) {
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

fn plain(target: ReloadTarget) -> ScriptReload {
    ScriptReload {
        target,
        notes: DescriptNotes::default(),
    }
}

// ---------------------------------------------------------------- 受理

/// 拾う綴りはそれぞれ 1 件、`parse_script` の写しのまま送られる（要件 1.4）。
#[test]
fn picked_spellings_send_one_request_each() {
    let cases: [(&str, &[&str], ScriptReload); 6] = [
        ("reload", &["ghost"], plain(ReloadTarget::Ghost)),
        ("reload", &["shiori"], plain(ReloadTarget::Shiori)),
        ("reload", &["shell"], plain(ReloadTarget::Shell)),
        ("reload", &["balloon"], plain(ReloadTarget::Balloon)),
        (
            "reload",
            &["descript"],
            plain(ReloadTarget::Descript(DescriptParts::ALL)),
        ),
        ("reloadsurface", &[], plain(ReloadTarget::Shell)),
    ];
    for (name, tokens, want) in cases {
        let (sent, events) = emit_one(name, tokens);
        assert_eq!(sent, vec![want], "{name} {tokens:?}");
        assert!(warns(&events).is_empty(), "{name} {tokens:?}: 警告なし");
    }
}

/// `\![reload,descript,パラメータ]` は語の仕分け（notes）ごと運ぶ。
#[test]
fn descript_with_param_carries_parts_and_notes() {
    let (sent, _) = emit_one("reload", &["descript", "shell headline foo"]);
    let mut want_notes = DescriptNotes::default();
    want_notes.unsupported.push("headline".into());
    want_notes.unknown.push("foo".into());
    assert_eq!(
        sent,
        vec![ScriptReload {
            target: ReloadTarget::Descript(DescriptParts {
                ghost: false,
                shell: true,
                balloon: false,
            }),
            notes: want_notes,
        }]
    );
}

// ---------------------------------------------------------------- 担当外

/// makoto・aigraph・引数なしの reload・他の名前は 0 件で警告も出さない（要件 1.5・10.3）。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 5] = [
        ("reload", &["makoto"]),
        ("reload", &["aigraph"]),
        ("reload", &[]),
        ("change", &["ghost", "B"]),
        ("reloadsurfaces", &[]),
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

/// 開封できない自分宛（`reload`）の荷物は警告を残し、送らない。
#[test]
fn unopenable_reload_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: "reload".into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0);
    assert_eq!(warns(&events), vec![Some("reload_cue_unopenable")]);
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する（ゴーストが居ない場面）。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) = capture(|| sink.emit(carrier_cue("reload", &["ghost"])));
    assert_eq!(warns(&events), vec![Some("reload_cue_send_failed")]);
}

// ---------------------------------------------------------------- 受信端

/// 結線は受信端を据えるだけ。据えた受信端へ送った要求がそのまま取り出せる。
#[test]
fn wire_reload_rx_inserts_the_receiver_and_requests_arrive() {
    let mut world = bevy_ecs::world::World::new();
    let (tx, rx) = channel();

    wire_reload_rx(&mut world, rx);
    tx.send(plain(ReloadTarget::Ghost)).unwrap();

    let got: Vec<ScriptReload> = world
        .get_non_send::<ReloadRx>()
        .expect("受信端が据わる")
        .try_iter()
        .collect();
    assert_eq!(got, vec![plain(ReloadTarget::Ghost)]);
}
