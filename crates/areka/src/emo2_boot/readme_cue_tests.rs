// =============================================================================
// ReadmeCueSink の自己選別と送り出しの決定論テスト（要件 9.5・4.5・4.6・8.4）
// =============================================================================
//
// 確かめること: 引数なしの `\![open,readme]` が要求を 1 件だけ送ること・引数付きが
// 何も送らず警告を 1 行残すこと・他のコマンド名と `\![open,他]` とキャリアでない cue が
// 何も送らないこと・`\![open,他]` では警告を出さないこと（担当外なので報せる責任は
// その担当者にある）・受信端が落ちていても落ちずに記録すること。

use super::*;
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
    assert_eq!(
        rx.try_iter().count(),
        1,
        "引数なしの \\![open,readme] は要求を 1 件だけ送る（要件 4.5）"
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

/// 同じ `open` でも第 1 引数が違えば担当外——何も送らず、警告も出さない
/// （報せる責任はその第 1 引数の担当者にあるため）。
#[test]
fn other_open_target_is_benign_skip_without_warning() {
    let (mut sink, rx) = sink();
    let logs = capture_logs(|| sink.emit(carrier_cue("open", &["browser", "https://example.com"])));
    assert_eq!(
        rx.try_iter().count(),
        0,
        "\\![open,他] は担当外なので何も送らない"
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
    sink.emit(carrier_cue("open", &[]));
    assert_eq!(rx.try_iter().count(), 0, "裸の \\![open] は担当外");
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
