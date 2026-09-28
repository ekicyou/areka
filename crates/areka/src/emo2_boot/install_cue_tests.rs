//! `InstallCueSink` の自己選別と送り出しの決定論テスト（要件 1.5・1.6・1.7・11.6）。
//!
//! 確かめること: 台本の文字列 `\![execute,install,path,絶対パス]` が受け口から出どころ「台本」の
//! 生の要求 1 件になり、窓口の取り出しで受付を通って依頼 1 件（出どころ「台本」・書庫 1 本）として
//! 待ち行列に入ること・相対パスと空が `install_cue_bad_path`、`path` 以外（`url` を含む）が
//! `install_cue_unsupported` の `warn!` 1 件で 0 件になること・パスの後ろの引数は `warn!` 1 件を
//! 残して読まないこと・担当外は 0 件で警告しないこと・受信端が落ちていても落ちずに記録すること。

use super::*;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use areka_sakura::sysvar::SystemVarSnapshot;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CuePlayer, CueSink, TalkCue};
use log_capture_kit::{CapturedEvent, capture};

use crate::install::{self, InstallOrigin, desk};

// ---------------------------------------------------------------- 道具立て

/// 絶対パスの書庫（どのテストも fs を読まない＝在る必要はない）。
const ABSOLUTE: &str = r"C:\areka-install-cue-test\in\hana.nar";

/// `\![name,tokens...]` の汎用キャリア cue を組む。
fn carrier_cue(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

fn sink() -> (InstallCueSink, Receiver<RawInstallRequest>) {
    let (tx, rx) = channel();
    (InstallCueSink::new(tx), rx)
}

/// 1 つの cue を流し、送られた要求と記録を返す。
fn emit_one(name: &str, tokens: &[&str]) -> (Vec<RawInstallRequest>, Vec<CapturedEvent>) {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| sink.emit(carrier_cue(name, tokens)));
    (rx.try_iter().collect(), events)
}

/// 台本の文字列を本番と同じ解析と組み立てで cue にし、受け口へ最後まで配る。
fn play_script(script: &str, sink: InstallCueSink) {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(sink));
    player.tick(compiled.sheet.absolute_end_time());
}

fn warns(events: &[CapturedEvent]) -> Vec<Option<&str>> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| e.field_str("event"))
        .collect()
}

fn script_raw(path: &str) -> RawInstallRequest {
    RawInstallRequest {
        path: PathBuf::from(path),
        origin: InstallOrigin::Script,
    }
}

// ---------------------------------------------------------------- 受理

/// 台本の文字列の `\![execute,install,path,絶対パス]` は、パスを無変形で運ぶ出どころ「台本」の
/// 生の要求 1 件になる（要件 1.5）。
#[test]
fn script_string_becomes_one_script_request_with_the_path() {
    let (sink, rx) = sink();
    let ((), events) =
        capture(|| play_script(&format!(r"\![execute,install,path,{ABSOLUTE}]\e"), sink));
    assert_eq!(
        rx.try_iter().collect::<Vec<_>>(),
        vec![script_raw(ABSOLUTE)]
    );
    assert_eq!(warns(&events), Vec::<Option<&str>>::new(), "警告なし");
}

/// 台本の文字列から受け口 → 窓口の取り出し → 受付まで通すと、窓口の待ち行列に出どころ「台本」・
/// 書庫 1 本の依頼がちょうど 1 件入る（要件 1.5・11.6）。
///
/// 待ち行列の中身は `install` の外から読めないので、受付が積んだときに残す
/// `info!(install_order_queued)`（出どころと書庫の本数）で判定する。依頼の中身が生の要求の
/// パスそのままであることは上のテストと窓口のテストが固定している。
#[test]
fn script_string_reaches_the_desk_queue_as_one_script_order() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    install::register(&mut world);
    let sink = InstallCueSink::new(desk::raw_sender(&world));

    let ((), events) = capture(|| {
        play_script(&format!(r"\![execute,install,path,{ABSOLUTE}]\e"), sink);
        desk::drain(&mut world);
    });

    let queued: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_order_queued"))
        .collect();
    assert_eq!(queued.len(), 1, "依頼はちょうど 1 件: {events:?}");
    assert_eq!(queued[0].field("origin"), Some("Script"), "出どころは台本");
    assert_eq!(queued[0].field("count"), Some("1"), "書庫は 1 本");
    assert_eq!(
        warns(&events),
        Vec::<Option<&str>>::new(),
        "断りも警告もない"
    );
}

/// パスより後ろの引数は読まず `warn!` を 1 件残す（要求は出す・design judge の箇条）。
#[test]
fn arguments_after_the_path_are_ignored_with_one_warning() {
    let (sent, events) = emit_one("execute", &["install", "path", ABSOLUTE, "--extra"]);
    assert_eq!(sent, vec![script_raw(ABSOLUTE)], "パスまでは読む");
    assert_eq!(warns(&events), vec![Some("install_cue_extra_ignored")]);
}

// ---------------------------------------------------------------- 断る

/// 相対パス・空・パスの欠けは `install_cue_bad_path` の `warn!` 1 件で送らない（要件 1.6）。
#[test]
fn relative_or_empty_path_warns_once_and_sends_nothing() {
    let cases: [&[&str]; 3] = [
        &["install", "path", r"Downloads\hana.nar"],
        &["install", "path", ""],
        &["install", "path"],
    ];
    for tokens in cases {
        let (sent, events) = emit_one("execute", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 送らない");
        assert_eq!(
            warns(&events),
            vec![Some("install_cue_bad_path")],
            "{tokens:?}: 警告 1 件"
        );
    }
}

/// 台本の文字列の相対パスも、窓口まで通して依頼 0 件・`warn!` 1 件（要件 1.6）。
#[test]
fn relative_path_in_a_script_string_reaches_no_order() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    install::register(&mut world);
    let sink = InstallCueSink::new(desk::raw_sender(&world));

    let ((), events) = capture(|| {
        play_script(r"\![execute,install,path,in\hana.nar]\e", sink);
        desk::drain(&mut world);
    });

    assert!(
        events
            .iter()
            .all(|e| e.field_str("event") != Some("install_order_queued")),
        "依頼は 0 件: {events:?}"
    );
    assert_eq!(warns(&events), vec![Some("install_cue_bad_path")]);
}

/// 2 番目の引数が `path` 以外（`url`・知らない語・無し）は `install_cue_unsupported` の `warn!`
/// 1 件で送らない（要件 1.7）。
#[test]
fn not_path_warns_once_and_sends_nothing() {
    let cases: [&[&str]; 3] = [
        &["install", "url", "https://example.com/hana.nar"],
        &["install", "file", ABSOLUTE],
        &["install"],
    ];
    for tokens in cases {
        let (sent, events) = emit_one("execute", tokens);
        assert!(sent.is_empty(), "{tokens:?}: 送らない");
        assert_eq!(
            warns(&events),
            vec![Some("install_cue_unsupported")],
            "{tokens:?}: 警告 1 件"
        );
    }
}

// ---------------------------------------------------------------- 担当外

/// `execute` の他の第 1 引数・裸の `execute`・他の名前は 0 件で警告も出さない。
#[test]
fn not_ours_is_benign_skip_without_warning() {
    let cases: [(&str, &[&str]); 3] = [
        ("execute", &["http-get", "https://example.com"]),
        ("execute", &[]),
        ("open", &["install", "path", ABSOLUTE]),
    ];
    for (name, tokens) in cases {
        let (sent, events) = emit_one(name, tokens);
        assert!(sent.is_empty(), "{name} {tokens:?}: 担当外は送らない");
        assert!(warns(&events).is_empty(), "{name} {tokens:?}: 警告しない");
    }
}

/// 開封できない自分宛（`execute`）の荷物は警告を残し、送らない。
#[test]
fn unopenable_execute_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let ((), events) = capture(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: "execute".into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0);
    assert_eq!(warns(&events), vec![Some("install_cue_unopenable")]);
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let ((), events) =
        capture(|| sink.emit(carrier_cue("execute", &["install", "path", ABSOLUTE])));
    assert_eq!(warns(&events), vec![Some("install_cue_send_failed")]);
}
