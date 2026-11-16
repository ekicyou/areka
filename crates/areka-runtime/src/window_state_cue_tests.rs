// =============================================================================
// WindowStateCueSink の自己選別・第 2 引数の読み分け・送り出しの決定論テスト
// （task 1.2・要件 1.1〜1.3／1.5〜1.7／7.1）
//
// 読み分けは純関数 `parse_window_state` で全分岐を押さえ、受け口では「送るか送らないか」
// と記録の水準と欄（値・余り）を押さえる。「送らない」の主張が空虚にならないよう、送らない
// 側の檻でも同じ受け口へ先に受理される指令を 1 本流し、受け口が生きていることを確かめる。
//
// ログ捕捉は `log-capture-kit` の捕捉窓へ委譲する（`zorder_cue_tests.rs` と同じ流儀）。
// =============================================================================

use super::*;
use areka_parsers::sakura::RAISE_TAG_CARRIER;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Receiver, channel};

// ---------------------------------------------------------------- 道具立て

/// 汎用キャリアの cue を組む（正準形＝`Custom` の String 配列）。
fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// 開けない `Custom`（`params` が配列でない）を宛名だけ差し替えて組む。
fn broken_custom_cue(command: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Custom {
            command: command.to_string(),
            params: DynamicValue::Null,
        },
        duration: 0.0,
    }
}

fn drain(rx: &Receiver<WindowStateDirective>) -> Vec<WindowStateDirective> {
    rx.try_iter().collect()
}

/// 受け口へ cue を 1 本流し、送られた指令と記録の行を返す。
fn run(cue: TalkCue) -> (Vec<WindowStateDirective>, Vec<String>) {
    let (tx, rx) = channel::<WindowStateDirective>();
    let mut sink = WindowStateCueSink::new(tx);
    let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || sink.emit(cue));
    (drain(&rx), logs)
}

fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|line| line.contains(&needle)).count()
}

fn lines_at<'a>(logs: &'a [String], level: &str) -> Vec<&'a String> {
    let needle = format!("level={level}");
    logs.iter().filter(|line| line.contains(&needle)).collect()
}

/// 註釈の行を落とす（説明文の綴りを「在る」と数えない）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

const ON: WindowStateDirective = WindowStateDirective::StayOnTop { on: true };
const OFF: WindowStateDirective = WindowStateDirective::StayOnTop { on: false };

// ---------------------------------------------------------------- 読み分け（純関数）

/// 要件 1.2・1.3: 小文字ちょうどの `stayontop`／`!stayontop` がオン／オフになる。
#[test]
fn t_wsc1_lowercase_spellings_decide_on_and_off() {
    assert_eq!(
        parse_window_state("set", &["windowstate", "stayontop"]),
        ParsedWindowState::Directive {
            directive: ON,
            extra: &[]
        }
    );
    assert_eq!(
        parse_window_state("set", &["windowstate", "!stayontop"]),
        ParsedWindowState::Directive {
            directive: OFF,
            extra: &[]
        }
    );
}

/// 要件 1.5・1.7: 大文字の混じり・`minimize`・空・無いは読めない値（値をそのまま返す）。
#[test]
fn t_wsc2_other_values_are_unrecognized_with_the_value_kept() {
    for value in [
        "StayOnTop",
        "STAYONTOP",
        "!StayOnTop",
        "minimize",
        "",
        " stayontop",
    ] {
        assert_eq!(
            parse_window_state("set", &["windowstate", value]),
            ParsedWindowState::Unrecognized { value: Some(value) },
            "第 2 引数 {value:?} は読めない値"
        );
    }
    assert_eq!(
        parse_window_state("set", &["windowstate"]),
        ParsedWindowState::Unrecognized { value: None },
        "第 2 引数が無いのも読めない値"
    );
}

/// 要件 1.6: 第 3 引数以降は第 2 引数で決めた上で余りとして返す。
#[test]
fn t_wsc3_trailing_arguments_become_extra_after_the_decision() {
    assert_eq!(
        parse_window_state("set", &["windowstate", "stayontop", "x", "1"]),
        ParsedWindowState::Directive {
            directive: ON,
            extra: &["x", "1"]
        }
    );
    assert_eq!(
        parse_window_state("set", &["windowstate", "!stayontop", ""]),
        ParsedWindowState::Directive {
            directive: OFF,
            extra: &[""]
        }
    );
    // 余りがあっても読めない値は読めないまま（余りで救わない）。
    assert_eq!(
        parse_window_state("set", &["windowstate", "minimize", "stayontop"]),
        ParsedWindowState::Unrecognized {
            value: Some("minimize")
        }
    );
}

/// 要件 1.1: `\v` は引数の有無に関わらず手前へ出す（引数は全部余り）。
#[test]
fn t_wsc4_raise_carrier_always_raises() {
    assert_eq!(
        parse_window_state(RAISE_TAG_CARRIER, &[]),
        ParsedWindowState::Directive {
            directive: WindowStateDirective::Raise,
            extra: &[]
        }
    );
    assert_eq!(
        parse_window_state(RAISE_TAG_CARRIER, &["stayontop", "1"]),
        ParsedWindowState::Directive {
            directive: WindowStateDirective::Raise,
            extra: &["stayontop", "1"]
        }
    );
}

/// 他人宛は自分宛でない（惜しい組を含む）。
#[test]
fn t_wsc5_other_addresses_are_not_mine() {
    let cases: &[(&str, &[&str])] = &[
        ("set", &["zorder", "1", "0"]),
        ("change", &["ghost", "x"]),
        ("set", &[]),
        ("set", &["WindowState", "stayontop"]),
        ("reset", &["windowstate", "stayontop"]),
        ("windowstate", &["stayontop"]),
        ("v", &[]),
        ("\\V", &[]),
    ];
    for (name, params) in cases {
        assert_eq!(
            parse_window_state(name, params),
            ParsedWindowState::NotMine,
            "{name:?} {params:?} は自分宛でない"
        );
    }
}

// ---------------------------------------------------------------- 受け口（送り出しと記録）

/// オン・オフ・手前へ出すはそれぞれ指令 1 本を送り、警告を出さない。
#[test]
fn t_wsc6_directives_are_sent_one_each() {
    let cases = [
        (carrier_cue("0", "set", &["windowstate", "stayontop"]), ON),
        (carrier_cue("0", "set", &["windowstate", "!stayontop"]), OFF),
        (
            carrier_cue("0", RAISE_TAG_CARRIER, &[]),
            WindowStateDirective::Raise,
        ),
    ];
    for (cue, want) in cases {
        let (got, logs) = run(cue);
        assert_eq!(got, vec![want]);
        assert_eq!(count_level(&logs, "WARN"), 0, "{logs:?}");
    }
}

/// 書いたスコープは読まない（裁定 1）: `\1\v` も `\0\v` と同じ指令。コードに演者の欄の綴りも無い。
#[test]
fn t_wsc7_executing_scope_does_not_change_the_directive() {
    for actor in ["0", "1", "2"] {
        assert_eq!(
            run(carrier_cue(actor, RAISE_TAG_CARRIER, &[])).0,
            vec![WindowStateDirective::Raise]
        );
        assert_eq!(
            run(carrier_cue(actor, "set", &["windowstate", "stayontop"])).0,
            vec![ON]
        );
    }
    assert!(
        !code_only(include_str!("window_state_cue.rs")).contains("cue.actor"),
        "受け口は演者の欄を読まない"
    );
}

/// 要件 1.5・1.7: 読めない値は送らず、値を欄に載せた警告 1 本を残す。
#[test]
fn t_wsc8_unrecognized_value_warns_with_the_value_and_sends_nothing() {
    for value in ["minimize", "StayOnTop", ""] {
        let (got, logs) = run(carrier_cue("0", "set", &["windowstate", value]));
        assert!(got.is_empty(), "読めない値 {value:?} は送らない");
        let warns = lines_at(&logs, "WARN");
        assert_eq!(warns.len(), 1, "{logs:?}");
        assert!(
            warns[0].contains(&format!("value=Some({value:?})")),
            "値をそのまま残す: {warns:?}"
        );
    }
    let (got, logs) = run(carrier_cue("0", "set", &["windowstate"]));
    assert!(got.is_empty());
    let warns = lines_at(&logs, "WARN");
    assert_eq!(warns.len(), 1, "{logs:?}");
    assert!(
        warns[0].contains("value=None"),
        "第 2 引数の無いことを残す: {warns:?}"
    );
}

/// 要件 1.6: 余りは指令を送った上で `debug!` に欄つきで残す（警告にしない）。
#[test]
fn t_wsc9_extra_arguments_are_recorded_and_the_directive_is_sent() {
    let (got, logs) = run(carrier_cue(
        "0",
        "set",
        &["windowstate", "!stayontop", "x", "y"],
    ));
    assert_eq!(got, vec![OFF]);
    assert_eq!(count_level(&logs, "WARN"), 0, "{logs:?}");
    assert!(
        lines_at(&logs, "DEBUG")
            .iter()
            .any(|l| l.contains(r#"extra=["x", "y"]"#)),
        "余りを欄に残す: {logs:?}"
    );

    let (got, logs) = run(carrier_cue("1", RAISE_TAG_CARRIER, &["z"]));
    assert_eq!(got, vec![WindowStateDirective::Raise]);
    assert!(
        lines_at(&logs, "DEBUG")
            .iter()
            .any(|l| l.contains(r#"extra=["z"]"#)),
        "`\\v` の余りも残す: {logs:?}"
    );

    // 余りが無ければ余りの記録は出さない。
    let (_, logs) = run(carrier_cue("0", "set", &["windowstate", "stayontop"]));
    assert!(!logs.iter().any(|l| l.contains("extra=")), "{logs:?}");
}

/// 他人宛・開けない荷物・キャリアでない cue は送らず、警告せず、`debug!` で必ず残す。
#[test]
fn t_wsc10_skips_send_nothing_and_leave_a_debug_record() {
    let skipped = vec![
        carrier_cue("0", "set", &["zorder", "1", "0"]),
        carrier_cue("0", "change", &["ghost", "x"]),
        carrier_cue("0", "set", &[]),
        broken_custom_cue("set"),
        broken_custom_cue(RAISE_TAG_CARRIER),
        broken_custom_cue("noexist"),
        TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("あひる".into()),
            duration: 0.0,
        },
    ];
    for cue in skipped {
        let label = format!("{:?}", cue.command);
        // 受け口が生きている証に、先に受理される指令を 1 本流しておく。
        let (tx, rx) = channel::<WindowStateDirective>();
        let mut sink = WindowStateCueSink::new(tx);
        sink.emit(carrier_cue("0", "set", &["windowstate", "stayontop"]));
        let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || sink.emit(cue));
        assert_eq!(
            drain(&rx),
            vec![ON],
            "読み飛ばしは指令を増やさない: {label}"
        );
        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "警告は出さない（宛名の規律）: {label} {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "読み飛ばしを残す: {label} {logs:?}"
        );
    }
}

/// 受信端が閉じていても台本を殺さない——警告を残して続ける。
#[test]
fn t_wsc11_disconnected_receiver_is_recorded_and_does_not_panic() {
    let (tx, rx) = channel::<WindowStateDirective>();
    drop(rx);
    let mut sink = WindowStateCueSink::new(tx);
    let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || {
        sink.emit(carrier_cue("0", RAISE_TAG_CARRIER, &[]));
        sink.emit(carrier_cue("0", "set", &["windowstate", "stayontop"]));
    });
    assert_eq!(
        count_level(&logs, "WARN"),
        2,
        "送れなかったことを残す: {logs:?}"
    );
}

/// 重なりの仕事の旗は送り出しの後に 1 か所で立てる（旗はプロセス共有で動的に読むと揺れる
/// ので、`zorder_cue_tests.rs` の t_zcs14 と同じくソースの前後関係で押さえる）。
#[test]
fn t_wsc12_zorder_wake_is_marked_once_after_the_send() {
    let code = code_only(include_str!("window_state_cue.rs"));
    assert_eq!(code.matches("tick_wake::mark(").count(), 1);
    let mark_at = code
        .find("tick_wake::mark(tick_wake::ZORDER)")
        .expect("重なりの旗を立てる");
    let send_at = code.find("self.tx.send(").expect("送り出しがある");
    assert!(
        send_at < mark_at,
        "旗は送り出しの後（送出={send_at}・旗={mark_at}）"
    );
}

/// 受け口は複製でき、複製から送っても同じ受信端へ届く（配送は台本ごとに複製する）。
#[test]
fn t_wsc13_clone_reaches_the_same_receiver() {
    let (tx, rx) = channel::<WindowStateDirective>();
    let sink = WindowStateCueSink::new(tx);
    let mut cloned = sink.clone();
    cloned.emit(carrier_cue("0", RAISE_TAG_CARRIER, &[]));
    assert_eq!(drain(&rx), vec![WindowStateDirective::Raise]);
}
