//! `UpdateCueSink` と `parse_update_command` の決定論テスト（要件 1.5〜1.8・9.4・10.8）。
//!
//! 確かめること: 入口の 4 つの形（`updatebymyself`・`update,all`・`update,shell+balloon`・
//! `updateother,--balloon=B,--shell=S`）が台本の文字列から期待の生の要求 1 件になること・
//! 更新オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりが要求 0 と `warn!` 1 件に
//! なること・`--shell=S,--plugin=P` が `P` を読み飛ばして `S` だけの要求になること・担当外は 0 件で
//! 警告しないこと・開けない自分宛の荷物と受信端の落ちた送出は `warn!` 1 件で台本を殺さないこと。

use super::*;
use std::sync::mpsc::{Receiver, channel};

use areka_sakura::sysvar::SystemVarSnapshot;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CuePlayer, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use crate::update::TargetKind::{self, Balloon, Ghost, Shell};

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

fn sink() -> (UpdateCueSink, Receiver<RawUpdateRequest>) {
    let (tx, rx) = channel();
    (UpdateCueSink::new(tx), rx)
}

/// 台本の文字列を本番と同じ解析と組み立てで cue にし、素の受信端を渡した受け口へ最後まで配る。
fn play_script(script: &str) -> (Vec<RawUpdateRequest>, Vec<CapturedEvent>) {
    let (sink, rx) = sink();
    let ((), events) = capture(|| {
        let instructions = areka_parsers::sakura::parse(script);
        let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
        let mut player = CuePlayer::from_sheet(&compiled.sheet);
        player.register_sink(Box::new(sink));
        player.tick(compiled.sheet.absolute_end_time());
    });
    (rx.try_iter().collect(), events)
}

fn emit_one(name: &str, tokens: &[&str]) -> (Vec<RawUpdateRequest>, Vec<CapturedEvent>) {
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

fn other(pairs: &[(TargetKind, &str)]) -> RawUpdateRequest {
    RawUpdateRequest::Other(pairs.iter().map(|(k, n)| (*k, n.to_string())).collect())
}

// ---------------------------------------------------------------- 入口の 4 つの形

/// 4 つの入口の形が、台本の文字列から期待の生の要求 1 件になり、警告を残さない
/// （要件 1.5・1.6・1.7・9.4）。
#[test]
fn four_entry_forms_become_the_expected_request() {
    let cases = [
        (
            r"\![updatebymyself]\e",
            RawUpdateRequest::Current(vec![Ghost, Shell, Balloon]),
        ),
        (
            r"\![update,all]\e",
            RawUpdateRequest::Current(vec![Ghost, Shell, Balloon]),
        ),
        (
            r"\![update,shell+balloon]\e",
            RawUpdateRequest::Current(vec![Shell, Balloon]),
        ),
        (
            r"\![updateother,--balloon=B,--shell=S]\e",
            other(&[(Balloon, "B"), (Shell, "S")]),
        ),
    ];
    for (script, expected) in cases {
        let (sent, events) = play_script(script);
        assert_eq!(sent, vec![expected], "{script}");
        assert!(warns(&events).is_empty(), "{script}: 警告なし {events:?}");
    }
}

/// `update` の対象は並んだ順（逆順・1 つだけも）で運び、重複はそのまま・`all` は 3 つに開く（要件 1.6）。
#[test]
fn update_targets_keep_their_order() {
    let cases: [(&[&str], Vec<TargetKind>); 4] = [
        (&["balloon+ghost"], vec![Balloon, Ghost]),
        (&["ghost"], vec![Ghost]),
        (&["shell+ghost+balloon"], vec![Shell, Ghost, Balloon]),
        (&["all+ghost"], vec![Ghost, Shell, Balloon, Ghost]),
    ];
    for (params, expected) in cases {
        assert_eq!(
            parse_update_command("update", params),
            Ok(Parsed {
                request: RawUpdateRequest::Current(expected),
                ignored: vec![],
            }),
            "{params:?}"
        );
    }
}

// ---------------------------------------------------------------- 読み飛ばし

/// `--shell=S,--plugin=P` は `P` を `warn!` 1 件で読み飛ばし、`S` だけの要求になる（要件 1.7・9.4）。
#[test]
fn unknown_selector_is_skipped_with_one_warning() {
    let (sent, events) = play_script(r"\![updateother,--shell=S,--plugin=P]\e");
    assert_eq!(sent, vec![other(&[(Shell, "S")])]);
    assert_eq!(warns(&events), vec![Some("update_cue_selector_ignored")]);
    assert_eq!(
        parse_update_command("updateother", &["--shell=S", "--plugin=P"]),
        Ok(Parsed {
            request: other(&[(Shell, "S")]),
            ignored: vec!["--plugin=P".into()],
        })
    );
}

// ---------------------------------------------------------------- 断る

/// オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりは要求 0 と `warn!` 1 件
/// （要件 1.8・9.4・10.8）。
#[test]
fn refused_forms_send_nothing_and_warn_once() {
    let scripts = [
        r"\![updatebymyself,checkonly]\e",
        r"\![update,all,testonly]\e",
        r"\![update,ghost,recovery]\e",
        r"\![update,platform]\e",
        r"\![update,ghost+platform]\e",
        r"\![updateother,--plugin=P]\e",
        r"\![updateother,--shell=S,--option=x]\e",
        r"\![updateother,--shell=S,checkonly]\e",
    ];
    for script in scripts {
        let (sent, events) = play_script(script);
        assert!(sent.is_empty(), "{script}: 要求 0");
        assert_eq!(
            warns(&events),
            vec![Some("update_cue_refused")],
            "{script}: 警告 1 件"
        );
    }
}

/// 断りの種類（純粋な解析）。
#[test]
fn refusal_kinds() {
    let option = |s: &str| Err(Refusal::Option { found: s.into() });
    assert_eq!(
        parse_update_command("updatebymyself", &["checkonly"]),
        option("checkonly")
    );
    assert_eq!(
        parse_update_command("update", &["all", "--option=x"]),
        option("--option=x")
    );
    assert_eq!(
        parse_update_command("update", &["platform"]),
        Err(Refusal::UnknownTarget {
            found: "platform".into()
        })
    );
    assert_eq!(
        parse_update_command("update", &[]),
        Err(Refusal::UnknownTarget { found: "".into() })
    );
    assert_eq!(
        parse_update_command("updateother", &["--shell=S", "--option=x", "--plugin=P"]),
        option("--option=x")
    );
    assert_eq!(
        parse_update_command("updateother", &["--shell=S", "foo"]),
        option("foo"),
        "`--` で始まらない語は読めない引数として断る"
    );
    assert_eq!(
        parse_update_command("updateother", &["--plugin=P"]),
        Err(Refusal::NoTargets)
    );
    assert_eq!(
        parse_update_command("updateother", &[]),
        Err(Refusal::NoTargets)
    );
}

// ---------------------------------------------------------------- 担当外

/// 他の名前（似た名前・`execute` を含む）は 0 件で警告しない。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 3] = [
        ("execute", &["install", "path", r"C:\x.nar"]),
        ("updatex", &[]),
        ("reload", &["ghost"]),
    ];
    for (name, tokens) in cases {
        let (sent, events) = emit_one(name, tokens);
        assert!(sent.is_empty(), "{name}: 送らない");
        assert!(warns(&events).is_empty(), "{name}: 警告しない");
    }
}

/// 開封できない自分宛の荷物は `warn!` 1 件で送らない。
#[test]
fn unopenable_own_cue_warns_and_sends_nothing() {
    for name in ["updatebymyself", "update", "updateother"] {
        let (mut sink, rx) = sink();
        let ((), events) = capture(|| {
            sink.emit(TalkCue {
                at: 0.0,
                actor: ActorKey::from("0"),
                command: CueCommand::Custom {
                    command: name.into(),
                    params: DynamicValue::Null,
                },
                duration: 0.0,
            })
        });
        assert_eq!(rx.try_iter().count(), 0, "{name}");
        assert_eq!(
            warns(&events),
            vec![Some("update_cue_unopenable")],
            "{name}"
        );
    }
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを `warn!` 1 件で残す。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) = capture(|| sink.emit(carrier_cue("updatebymyself", &[])));
    assert_eq!(warns(&events), vec![Some("update_cue_send_failed")]);
}
