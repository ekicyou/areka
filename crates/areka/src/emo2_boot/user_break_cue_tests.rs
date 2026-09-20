// =============================================================================
// 中断を禁じる区間の受け口の決定論テスト（areka-P0-balloon-break）
// =============================================================================
//
// 担当のコマンドだけを拾う自己選別と、トークの始まりの自己検出をここで固定する。
//
// 確かめること: `\![enter,nouserbreakmode]`／`\![leave,nouserbreakmode]` の 2 組だけを拾う
// こと・第 1 引数が違う `onlinemode` は拾わないこと・複製ごとに「トークが始まった」が
// 先頭に 1 回だけ出ること・受信端が落ちていても台本を殺さず記録すること（要件 5.1・5.2・
// 5.4・7.3）。

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
fn sink() -> (NoUserBreakCueSink, Receiver<NoUserBreakSignal>) {
    let (tx, rx) = channel::<NoUserBreakSignal>();
    (NoUserBreakCueSink::new(tx), rx)
}

/// 受信端に溜まった合図を届いた順に取り出す。
fn drain(rx: &Receiver<NoUserBreakSignal>) -> Vec<NoUserBreakSignal> {
    rx.try_iter().collect()
}

/// クロージャ実行中に**現在のスレッド**で発火した記録を 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち指定した水準の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|line| line.contains(&needle)).count()
}

// ---------------------------------------------------------------- 受理

/// `\![enter,nouserbreakmode]` は「トークが始まった」に続いて「入る」を送る（要件 5.1）。
#[test]
fn enter_no_user_break_mode_sends_enter() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("enter", &["nouserbreakmode"]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted, NoUserBreakSignal::Enter],
        "\\![enter,nouserbreakmode] は Enter を送る（先頭は複製後 1 回だけの TalkStarted）"
    );
}

/// `\![leave,nouserbreakmode]` は「出る」を送る（要件 5.2）。
#[test]
fn leave_no_user_break_mode_sends_leave() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("leave", &["nouserbreakmode"]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted, NoUserBreakSignal::Leave],
        "\\![leave,nouserbreakmode] は Leave を送る"
    );
}

/// 台本の順序がそのまま線の上の順序になる（1 本のスレッドから順に出るため）。
#[test]
fn signals_follow_the_script_order() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("enter", &["nouserbreakmode"]));
    sink.emit(carrier_cue("leave", &["nouserbreakmode"]));
    assert_eq!(
        drain(&rx),
        vec![
            NoUserBreakSignal::TalkStarted,
            NoUserBreakSignal::Enter,
            NoUserBreakSignal::Leave,
        ],
        "TalkStarted が先頭で、以降は台本の順"
    );
}

// ---------------------------------------------------------------- 担当外

/// 同じ `enter`／`leave` でも第 1 引数が `onlinemode` なら担当外——旗の合図は 1 件も出ない
/// （要件 8.2）。**両側から挟む**ため `enter` と `leave` の両方を通す。
#[test]
fn online_mode_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("enter", &["onlinemode"]));
    sink.emit(carrier_cue("leave", &["onlinemode"]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted],
        "onlinemode は担当外——TalkStarted のほかには何も出ない（要件 8.2）"
    );
}

/// 第 1 引数の無い裸の `\![enter]` も担当外（選別子が読めないので受理しない）。
#[test]
fn bare_enter_without_selector_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("enter", &[]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted],
        "裸の \\![enter] は担当外"
    );
}

/// 他のコマンド名（`\![move,…]`）は担当外。
#[test]
fn other_command_name_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(carrier_cue("move", &["-353", "", "", "0", "base", "base"]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted],
        "担当外の名前からは旗の合図を作らない"
    );
}

/// 運搬でない cue（文字など）・開封できない荷物でも落ちず、旗の合図も作らない。
#[test]
fn non_carrier_cue_is_benign_skip() {
    let (mut sink, rx) = sink();
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("アヒル".into()),
        duration: 0.0,
    });
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Custom {
            command: "enter".into(),
            params: DynamicValue::Null,
        },
        duration: 0.0,
    });
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted],
        "運搬でない cue と開封できない荷物からは旗の合図を作らない"
    );
}

// ---------------------------------------------------------------- トークの境界

/// 複製ごとに「トークが始まった」が**先頭に 1 回だけ**出る（要件 5.4・旗の順序）。
///
/// 複製＝トークの境界なので、複製した受け口は前のトークの状態を引き継がない。同じ受け口を
/// 何度 `emit` しても TalkStarted は増えず、新しい複製では必ずもう一度先頭に出る。
#[test]
fn talk_started_is_sent_once_per_clone_and_comes_first() {
    let (registered, rx) = sink();

    let mut first = registered.clone();
    first.emit(carrier_cue("enter", &["nouserbreakmode"]));
    first.emit(carrier_cue("leave", &["nouserbreakmode"]));
    assert_eq!(
        drain(&rx),
        vec![
            NoUserBreakSignal::TalkStarted,
            NoUserBreakSignal::Enter,
            NoUserBreakSignal::Leave,
        ],
        "1 つ目の複製: TalkStarted は先頭に 1 回だけ"
    );

    let mut second = registered.clone();
    second.emit(carrier_cue("enter", &["nouserbreakmode"]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted, NoUserBreakSignal::Enter],
        "2 つ目の複製でも TalkStarted がもう一度先頭に出る（複製＝トークの境界）"
    );

    // 既に指示を配った受け口から複製しても境界は戻る（登録した受け口が配られないという
    // 上流の前提に頼らない）。
    let mut third = first.clone();
    third.emit(carrier_cue("leave", &["nouserbreakmode"]));
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted, NoUserBreakSignal::Leave],
        "配った後の受け口からの複製でも TalkStarted が先頭に出る"
    );
}

/// 担当外の指示しか配られないトークでも、「トークが始まった」は出る（閉じ忘れた旗を解く
/// 契機は担当のコマンドではなく最初の指示である・要件 5.4）。
#[test]
fn talk_started_comes_from_any_first_cue() {
    let (mut sink, rx) = sink();
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::ClearAll,
        duration: 0.0,
    });
    assert_eq!(
        drain(&rx),
        vec![NoUserBreakSignal::TalkStarted],
        "最初の指示がどれであっても TalkStarted は出る"
    );
}

// ---------------------------------------------------------------- 送出の失敗

/// 受信端が落ちていても落ちず、送れなかったことを記録する（黙って諦めない・要件 6）。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (mut sink, rx) = sink();
    drop(rx);
    let logs = capture_logs(|| sink.emit(carrier_cue("enter", &["nouserbreakmode"])));
    assert_eq!(
        count_level(&logs, "WARN"),
        2,
        "TalkStarted と Enter の両方の送出の失敗が記録される: {logs:?}"
    );
}
