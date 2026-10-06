// =============================================================================
// ReadmeCueSink の自己選別と送り出しの決定論テスト（要件 9.5・4.5・4.6・8.4・
// areka-P0-open-external-tags 要件 1.5・1.7・2.6・3.2・4.7・6.3・8.6・10.3）
// =============================================================================
//
// 確かめること: 引数なしの `\![open,readme]` が要求を 1 件だけ送ること・引数付きが
// 何も送らず警告を 1 行残すこと・5 つの `open` の形と `\j` が行き先を 1 件ずつ送ること・
// 断る入力が何も送らず `open_external_rejected` を 1 行残すこと・他のコマンド名と
// `\![open,help]`・裸の `\![open]`・キャリアでない cue が何も送らず警告も出さないこと・
// 開封できない荷物は宛名（`open`・`\j`）で水準を分けること・受信端が落ちていても落ちずに
// 記録すること・送った行き先の列が台本からの取り出し（`link_destinations`）と一致すること。

use super::*;
use crate::readme::destination::{Destination, OpenKind, classify, link_destinations};
use areka_parsers::sakura::JUMP_TAG_CARRIER;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Receiver, channel};

// ---------------------------------------------------------------- 道具立て

/// `\![name,tokens...]` の汎用キャリア cue を組む（正準形＝`Custom` の String 配列）。
fn carrier_cue(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// 受け口と受信端の組を作る。
fn sink() -> (ReadmeCueSink, Receiver<ReadmeRequest>) {
    let (tx, rx) = channel::<ReadmeRequest>();
    (ReadmeCueSink::new(tx), rx)
}

/// クロージャ実行中に**現在のスレッド**で発火した記録を 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち指定した水準（`"WARN"`）の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|line| line.contains(&needle)).count()
}

// ---------------------------------------------------------------- 受理

/// 引数なしの `\![open,readme]` は説明書の要求をちょうど 1 件送る（要件 4.5）。
#[test]
fn bare_open_readme_sends_exactly_one_request() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("open", &["readme"]));
    let sent: Vec<_> = rx.try_iter().collect();
    assert_eq!(
        sent.len(),
        1,
        "引数なしの \\![open,readme] は要求を 1 件だけ送る（要件 4.5）"
    );
    assert!(
        matches!(sent[0], ReadmeRequest::Readme),
        "説明書の要求のまま送る"
    );
}

// ---------------------------------------------------------------- 縮退

/// 引数付きの `\![open,readme,種類,名前]` は何も送らず、警告を 1 行だけ残す（要件 4.6）。
#[test]
fn open_readme_with_arguments_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| sink.emit(carrier_cue("open", &["readme", "ghost", "x"])));
    assert_eq!(
        rx.try_iter().count(),
        0,
        "引数付きは何もしない（列挙が要るため α では語彙だけ持つ・要件 4.6）"
    );
    assert_eq!(
        count_level(&logs, "WARN"),
        1,
        "引数付きは警告を 1 行だけ残す（要件 4.6）: {logs:?}"
    );
}

// ---------------------------------------------------------------- 担当外

/// 他のコマンド名（`\![move,…]`）は何も送らない（自己選別・担当外）。
#[test]
fn other_command_name_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("move", &["-353", "", "", "0", "base", "base"]));
    assert_eq!(rx.try_iter().count(), 0, "担当外の名前は何も送らない");
}

/// `\![open,help]`（SSP 自身の窓）は担当外——何も送らず、警告も出さない
/// （報せる責任はその担当者にあるため・要件 9.3）。
#[test]
fn open_help_is_benign_skip_without_warning() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| sink.emit(carrier_cue("open", &["help"])));
    assert_eq!(
        rx.try_iter().count(),
        0,
        "\\![open,help] は担当外なので何も送らない"
    );
    assert_eq!(
        count_level(&logs, "WARN"),
        0,
        "担当外は警告にしない（良性の読み飛ばし）: {logs:?}"
    );
}

/// 第 1 引数の無い裸の `\![open]` も担当外（選別子が読めないので受理しない）。
#[test]
fn bare_open_without_selector_is_benign_skip() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| sink.emit(carrier_cue("open", &[])));
    assert_eq!(rx.try_iter().count(), 0, "裸の \\![open] は担当外");
    assert_eq!(
        count_level(&logs, "WARN"),
        0,
        "担当外は警告にしない: {logs:?}"
    );
}

/// キャリアでない cue（文字など）は何も送らない。
#[test]
fn non_carrier_cue_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("アヒル".into()),
        duration: 0.0,
    });
    assert_eq!(
        rx.try_iter().count(),
        0,
        "キャリアでない cue は何も送らない"
    );
}

/// 開封できない `Custom`（`params` が String 配列でない）でも落ちない。宛名が自分（`open`）
/// のときは壊れ物として警告を残す（宛名の規律・`zorder_cue.rs` と同じ流儀）。
#[test]
fn unopenable_custom_addressed_to_open_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: "open".into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0, "開封できない荷物は何も送らない");
    assert_eq!(
        count_level(&logs, "WARN"),
        1,
        "自分宛の開けない荷物は警告を残す（宛名の規律）: {logs:?}"
    );
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する（黙って諦めない・要件 8.4）。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let logs = capture_logs(|| sink.emit(carrier_cue("open", &["readme"])));
    assert_eq!(
        count_level(&logs, "WARN"),
        1,
        "送出の失敗は警告として残る（要件 8.4）: {logs:?}"
    );
}

/// 複製した受け口からの送出も同じ受信端へ届く（配送が台本ごとに複製する前提）。
#[test]
fn clone_reaches_the_same_receiver() {
    let (sink, rx) = sink();
    let mut clone = sink.clone();
    clone.emit(carrier_cue("open", &["readme"]));
    assert_eq!(rx.try_iter().count(), 1, "複製からの送出も同じ受信端へ届く");
}

// ---------------------------------------------------------------- 開く系（open-external-tags）

/// 受信端に届いた開く系の行き先を順に取り出す（`Readme` が混じれば落とす）。
fn sent_destinations(rx: &Receiver<ReadmeRequest>) -> Vec<Destination> {
    rx.try_iter()
        .map(|req| match req {
            ReadmeRequest::Open(dest) => dest,
            ReadmeRequest::Readme => panic!("開く系の cue が説明書の要求として送られた"),
        })
        .collect()
}

/// 5 つの `open` の形と `\j` は、規則で分類した行き先をちょうど 1 件送り、警告を出さない
/// （要件 1.5・8.6）。
#[test]
fn open_forms_and_jump_send_classified_destination() {
    let cases: &[(&str, &[&str], OpenKind)] = &[
        ("open", &["file", "notepad.exe"], OpenKind::File),
        ("open", &["browser", "https://example.com/"], OpenKind::Url),
        ("open", &["explorer", r"C:\x"], OpenKind::Folder),
        ("open", &["explorer", "ghost", "emo2"], OpenKind::Folder),
        ("open", &["editor", "a.txt", "3"], OpenKind::Editor),
        ("open", &["mailer", "a@b.c"], OpenKind::Mail),
        (JUMP_TAG_CARRIER, &["http://example.com/"], OpenKind::Url),
        (JUMP_TAG_CARRIER, &["mailto:a@b.c"], OpenKind::Mail),
        (JUMP_TAG_CARRIER, &["file:///descript.txt"], OpenKind::File),
    ];
    for (name, tokens, kind) in cases {
        let (mut sink, rx) = sink();
        let logs = capture_logs(|| sink.emit(carrier_cue(name, tokens)));
        let sent = sent_destinations(&rx);
        let expected = classify(name, tokens).unwrap().unwrap();
        assert_eq!(sent, vec![expected], "{name} {tokens:?} は 1 件だけ送る");
        assert_eq!(sent[0].target.kind(), *kind, "{name} {tokens:?} の種類");
        assert_eq!(count_level(&logs, "WARN"), 0, "受理は警告しない: {logs:?}");
    }
}

/// 断る入力は何も送らず、`open_external_rejected` の警告を綴りと理由つきで 1 行だけ残す
/// （要件 1.7・2.6・3.2・4.7・6.3）。
#[test]
fn rejected_inputs_warn_once_and_send_nothing() {
    let cases: &[(&str, &[&str], &str)] = &[
        ("open", &["file"], "MissingArgument"),
        ("open", &["browser", ""], "MissingArgument"),
        ("open", &["explorer"], "MissingArgument"),
        ("open", &["editor"], "MissingArgument"),
        ("open", &["mailer"], "MissingArgument"),
        ("open", &["explorer", "plugin", "p"], "UnsupportedStore"),
        ("open", &["explorer", "headline", "h"], "UnsupportedStore"),
        (JUMP_TAG_CARRIER, &["nope"], "UnknownJumpId"),
        (JUMP_TAG_CARRIER, &[], "UnknownJumpId"),
    ];
    for (name, tokens, reason) in cases {
        let (mut sink, rx) = sink();
        let logs = capture_logs(|| sink.emit(carrier_cue(name, tokens)));
        assert_eq!(rx.try_iter().count(), 0, "{name} {tokens:?} は何も送らない");
        let warns: Vec<&String> = logs.iter().filter(|l| l.contains("level=WARN")).collect();
        assert_eq!(warns.len(), 1, "{name} {tokens:?} は警告 1 行: {logs:?}");
        let line = warns[0];
        assert!(
            line.contains("open_external_rejected")
                && line.contains(reason)
                && line.contains("tag="),
            "{name} {tokens:?} の警告に事象名・理由・綴りが要る: {line}"
        );
    }
}

/// 開封できない荷物の宛名が運搬名 `\j` でも、自分宛の壊れ物として警告を残す。
#[test]
fn unopenable_custom_addressed_to_jump_warns_and_sends_nothing() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: JUMP_TAG_CARRIER.into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0, "開封できない荷物は何も送らない");
    assert_eq!(
        count_level(&logs, "WARN"),
        1,
        "自分宛（\\j）の開けない荷物は警告を残す（宛名の規律）: {logs:?}"
    );
}

/// 他人宛の開封できない荷物は警告しない（良性の読み飛ばし）。
#[test]
fn unopenable_custom_addressed_to_others_does_not_warn() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| {
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Custom {
                command: "move".into(),
                params: DynamicValue::Null,
            },
            duration: 0.0,
        })
    });
    assert_eq!(rx.try_iter().count(), 0);
    assert_eq!(
        count_level(&logs, "WARN"),
        0,
        "他人宛は警告しない: {logs:?}"
    );
}

/// 同じ台本を読み込んで得た汎用コマンドを運び手の cue にして受け口へ通すと、送られた
/// 行き先の列は台本からの取り出しと一致する（同じ規則・要件 8.6・10.3）。
#[test]
fn sink_agrees_with_link_destinations() {
    let script = concat!(
        r"\0本文\![open,mailer,a@b.c]",
        r"\![open,readme]\![open,help]\![open]",
        r"\q[選ぶ,OnX]\_a[OnY]リンク\_a",
        r"\j[https://a/]\![open,editor,x.txt,3]",
        r"\![open,file,notepad.exe]\![open,explorer,ghost,emo2]",
        r"\j[nope]\![open,browser]\![open,explorer,plugin,p]",
        r"\![open,browser,http://b/]\j[file:///descript.txt]\![open,explorer,C:\x]\e",
    );
    let (mut sink, rx) = sink();
    for ins in areka_parsers::sakura::parse(script) {
        if let areka_parsers::sakura::Instruction::GenericCommand { name, raw_args } = ins {
            sink.emit(TalkCue {
                at: 0.0,
                actor: ActorKey::from("0"),
                command: CueCommand::command_carrier(&name, raw_args),
                duration: 0.0,
            });
        }
    }
    let sent: Vec<Destination> = rx
        .try_iter()
        .filter_map(|req| match req {
            ReadmeRequest::Open(dest) => Some(dest),
            ReadmeRequest::Readme => None,
        })
        .collect();
    let extracted = link_destinations(script);
    assert_eq!(extracted.len(), 8, "見本の台本は 8 件の行き先を持つ");
    assert_eq!(sent, extracted, "受け口と取り出しは同じ列を返す");
}
