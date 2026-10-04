//! `get_log` の決定論テスト。
//!
//! 履歴は手元で作り（種別の混ざった 10 件・時刻は与えた値）、その種別の列を写す口を
//! `answer` へ渡す（置き場を共有しない）。`log_type` の検査（要件 3.1〜3.3・3.6）と
//! 書式（要件 4.1〜4.7）を固定する。

use std::cell::Cell;

use areka_mcp::{ToolContent, ToolOutcome};

use super::*;
use crate::log_history::{Draft, History, Kind, Stamp};

fn at(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> Stamp {
    Stamp {
        year,
        month,
        day,
        hour,
        minute,
    }
}

fn push(h: &mut History, at: Stamp, kind: Kind, label: &str, name: &str, body: &str) {
    h.push(
        at,
        Draft {
            kind,
            label: label.to_string(),
            name: name.to_string(),
            body: body.to_string(),
        },
    );
}

/// 種別の混ざった 10 件（番号は 1〜10）。
fn history() -> History {
    let mut h = History::new();
    let t = at(2026, 10, 4, 12, 34);
    push(
        &mut h,
        at(2026, 1, 5, 3, 7),
        Kind::Error,
        "Error",
        "[SYSTEM]",
        "first error",
    );
    push(
        &mut h,
        t,
        Kind::Script,
        "Ghost:OnBoot",
        "Emily/Phase4.5",
        r"\0\s[0]こんにちは\n\![raise,OnTest]",
    );
    push(&mut h, t, Kind::Status, "STAT", "STAT", "boot");
    push(&mut h, t, Kind::Network, "Info", "[SYSTEM]", "fetch");
    push(
        &mut h,
        t,
        Kind::Error,
        "Error",
        "[SYSTEM]",
        "line1\nline2\r\nline3",
    );
    push(
        &mut h,
        t,
        Kind::Update,
        "Info",
        "[SYSTEM]",
        "update checked",
    );
    push(&mut h, t, Kind::Script, "SSTP", "[SYSTEM]", r"\h\s[10]");
    push(&mut h, t, Kind::Error, "Error", "[SYSTEM]", "a\rb");
    push(&mut h, t, Kind::Status, "STAT", "STAT", "ghost switched");
    push(
        &mut h,
        t,
        Kind::Error,
        "Error",
        "Emily/Phase4.5",
        "ghost error",
    );
    h
}

fn args(log_type: Option<&str>) -> Args {
    Args {
        log_type: log_type.map(str::to_string),
        ghost_name: None,
        since_id: None,
        max_count: None,
    }
}

fn text(outcome: ToolOutcome) -> (String, bool) {
    match outcome.content.as_slice() {
        [ToolContent::Text(text)] => (text.clone(), outcome.is_error),
        other => panic!("本文 1 つのはず: {other:?}"),
    }
}

/// `h` の写しで答え、（本文, isError, 取り出した種別）を返す。
fn ask(h: &History, log_type: Option<&str>) -> (String, bool, Option<Kind>) {
    let asked = Cell::new(None);
    let outcome = answer(
        |kind| {
            asked.set(Some(kind));
            h.rows(kind).cloned().collect()
        },
        None,
        &args(log_type),
    );
    let (body, is_error) = text(outcome);
    (body, is_error, asked.get())
}

const ERROR_LINES: &str = "#1 2026/01/05 03:07 [Error] [SYSTEM] : first error\r\n\
     #5 2026/10/04 12:34 [Error] [SYSTEM] : line1\r\n\tline2\r\n\tline3\r\n\
     #8 2026/10/04 12:34 [Error] [SYSTEM] : a\r\n\tb\r\n\
     #10 2026/10/04 12:34 [Error] Emily/Phase4.5 : ghost error";

#[test]
fn omitted_log_type_is_error() {
    let (body, is_error, asked) = ask(&history(), None);
    assert_eq!(asked, Some(Kind::Error));
    assert_eq!(body, ERROR_LINES);
    assert!(!is_error);
}

#[test]
fn each_of_the_five_words_picks_its_kind() {
    let h = history();
    for (word, kind) in [
        ("error", Kind::Error),
        ("script", Kind::Script),
        ("network", Kind::Network),
        ("update", Kind::Update),
        ("status", Kind::Status),
    ] {
        let (_, is_error, asked) = ask(&h, Some(word));
        assert_eq!(asked, Some(kind), "{word}");
        assert!(!is_error, "{word}");
    }
    assert_eq!(
        ask(&h, Some("status")).0,
        "#3 2026/10/04 12:34 [STAT] STAT : boot\r\n\
         #9 2026/10/04 12:34 [STAT] STAT : ghost switched"
    );
}

#[test]
fn upper_case_error_is_error() {
    let (body, is_error, asked) = ask(&history(), Some("ERROR"));
    assert_eq!(asked, Some(Kind::Error));
    assert_eq!(body, ERROR_LINES);
    assert!(!is_error);
}

#[test]
fn empty_padded_and_unknown_words_are_ng_without_fetching() {
    for word in ["", " error ", "bogus"] {
        let (body, is_error, asked) = ask(&history(), Some(word));
        assert_eq!(
            body, "NG:Unknown log_type (error / script / network / update / status)",
            "{word:?}"
        );
        assert!(is_error, "{word:?}");
        assert_eq!(asked, None, "{word:?} で記録を取り出さない");
    }
}

#[test]
fn one_record_is_one_line_without_ok_or_separator() {
    let (body, is_error, _) = ask(&history(), Some("update"));
    assert_eq!(body, "#6 2026/10/04 12:34 [Info] [SYSTEM] : update checked");
    assert!(!is_error);
}

#[test]
fn many_records_are_joined_oldest_first_without_trailing_separator() {
    let (body, _, _) = ask(&history(), Some("error"));
    assert!(!body.starts_with("OK"));
    assert!(!body.ends_with("\r\n"));
    let ids: Vec<&str> = body
        .split("\r\n")
        .filter(|l| l.starts_with('#'))
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    assert_eq!(ids, ["#1", "#5", "#8", "#10"]);
}

#[test]
fn zero_records_is_no_log_entries() {
    let (body, is_error, asked) = ask(&History::new(), Some("network"));
    assert_eq!(asked, Some(Kind::Network));
    assert_eq!(body, "(no log entries)");
    assert!(!is_error);
}

#[test]
fn time_is_zero_padded() {
    let h = history();
    let first = h.rows(Kind::Error).next().unwrap();
    assert!(render(first).starts_with("#1 2026/01/05 03:07 "));
}

#[test]
fn backslashes_and_literal_n_pass_through_unchanged() {
    let (body, _, _) = ask(&history(), Some("script"));
    assert_eq!(
        body,
        "#2 2026/10/04 12:34 [Ghost:OnBoot] Emily/Phase4.5 : \\0\\s[0]こんにちは\\n\\![raise,OnTest]\r\n\
         #7 2026/10/04 12:34 [SSTP] [SYSTEM] : \\h\\s[10]"
    );
}

#[test]
fn real_newlines_become_continuation_lines_and_crlf_counts_once() {
    let h = history();
    let rows: Vec<_> = h.rows(Kind::Error).collect();
    // `\n` と `\r\n` が 1 つずつ＝3 行（`\r\n` を 2 つに数えない）。
    assert_eq!(
        render(rows[1]),
        "#5 2026/10/04 12:34 [Error] [SYSTEM] : line1\r\n\tline2\r\n\tline3"
    );
    // `\r` だけも 1 つの改行。
    assert_eq!(
        render(rows[2]),
        "#8 2026/10/04 12:34 [Error] [SYSTEM] : a\r\n\tb"
    );
}
