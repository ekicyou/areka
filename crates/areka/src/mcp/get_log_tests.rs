//! `get_log` の決定論テスト。
//!
//! 履歴は手元で作り（種別の混ざった 10 件・時刻は与えた値）、その種別の列を写す口を
//! `answer` へ渡す（置き場を共有しない）。`log_type` の検査（要件 3.1〜3.3・3.6）・
//! 書式（要件 4.1〜4.7）・絞り込みと `ghost_name` の解決（要件 5.1〜5.10）・入口（要件 3.4・3.5）を固定する。

use std::cell::Cell;
use std::path::PathBuf;

use areka_mcp::tools::{ToolCall, ToolRequest};
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
    ask_with(h, None, &args(log_type))
}

/// 起動中のゴーストと引数を与えて `h` の写しで答える。
fn ask_with(
    h: &History,
    active: Option<&ActiveGhost>,
    args: &Args,
) -> (String, bool, Option<Kind>) {
    let asked = Cell::new(None);
    let outcome = answer(
        |kind| {
            asked.set(Some(kind));
            h.rows(kind).cloned().collect()
        },
        active,
        args,
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

// ---- 絞り込み（要件 5.5〜5.10） ----

/// 引数を全部与える形。
fn full(
    log_type: Option<&str>,
    ghost_name: Option<&str>,
    since_id: Option<i64>,
    max_count: Option<i64>,
) -> Args {
    Args {
        log_type: log_type.map(str::to_string),
        ghost_name: ghost_name.map(str::to_string),
        since_id,
        max_count,
    }
}

/// 本文の行のうち記録の先頭の行の番号（古い順）。0 件の本文は空の列。
fn ids(body: &str) -> Vec<u64> {
    body.split("\r\n")
        .filter_map(|l| l.strip_prefix('#'))
        .map(|l| l.split(' ').next().unwrap().parse().unwrap())
        .collect()
}

/// error 種別（番号 1・5・8・10）を `since_id`・`max_count` で絞った番号。
fn error_ids(since_id: Option<i64>, max_count: Option<i64>) -> Vec<u64> {
    let (body, is_error, _) = ask_with(
        &history(),
        None,
        &full(Some("error"), None, since_id, max_count),
    );
    assert!(!is_error, "{since_id:?} {max_count:?}");
    if ids(&body).is_empty() {
        assert_eq!(body, NO_ENTRIES, "{since_id:?} {max_count:?}");
    }
    ids(&body)
}

#[test]
fn since_id_boundaries() {
    // ちょうど同じ番号は含めない。
    assert_eq!(error_ids(Some(8), None), [10]);
    // 1 つ小さい番号なら含める。
    assert_eq!(error_ids(Some(7), None), [8, 10]);
    // 負・無いは絞らない。0 は全件（番号は 1 から）。
    assert_eq!(error_ids(Some(-1), None), [1, 5, 8, 10]);
    assert_eq!(error_ids(None, None), [1, 5, 8, 10]);
    assert_eq!(error_ids(Some(0), None), [1, 5, 8, 10]);
    // 最後に振った番号以上・最大より大きいは 0 件（要件 5.8）。
    assert_eq!(error_ids(Some(10), None), Vec::<u64>::new());
    assert_eq!(error_ids(Some(11), None), Vec::<u64>::new());
}

#[test]
fn max_count_zero_negative_one_and_larger() {
    assert_eq!(error_ids(None, Some(0)), Vec::<u64>::new());
    assert_eq!(error_ids(None, Some(-1)), [1, 5, 8, 10]);
    // 新しい方から 1 件。
    assert_eq!(error_ids(None, Some(1)), [10]);
    // 新しい方から N 件を古い順で。
    assert_eq!(error_ids(None, Some(3)), [5, 8, 10]);
    // 件数より大きければ全件。
    assert_eq!(error_ids(None, Some(99)), [1, 5, 8, 10]);
}

#[test]
fn since_id_and_max_count_together() {
    // 番号 1 より後（5・8・10）のうち新しい方から 2 件を古い順で。
    assert_eq!(error_ids(Some(1), Some(2)), [8, 10]);
    assert_eq!(error_ids(Some(5), Some(5)), [8, 10]);
}

// ---- ghost_name（要件 5.1〜5.4・3.4） ----

fn emily() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    }
}

fn ghost_ids(active: Option<&ActiveGhost>, log_type: &str, ghost_name: &str) -> Vec<u64> {
    let (body, is_error, _) = ask_with(
        &history(),
        active,
        &full(Some(log_type), Some(ghost_name), None, None),
    );
    assert!(!is_error, "{log_type} {ghost_name:?}: {body}");
    ids(&body)
}

#[test]
fn ghost_name_keeps_only_records_of_that_name() {
    let g = emily();
    assert_eq!(ghost_ids(Some(&g), "error", "Emily/Phase4.5"), [10]);
    assert_eq!(ghost_ids(Some(&g), "script", "Emily/Phase4.5"), [2]);
}

#[test]
fn ghost_name_full_path_is_read_as_that_ghost() {
    let g = emily();
    for path in [
        r"C:\ssp\ghost\emily4",
        r"c:\SSP\Ghost\EMILY4\",
        "C:/ssp/ghost/emily4/",
    ] {
        assert_eq!(ghost_ids(Some(&g), "error", path), [10], "{path}");
    }
}

#[test]
fn nameless_ghost_matches_records_named_by_its_full_path() {
    let g = ActiveGhost {
        name: None,
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\nameless\"),
    };
    let mut h = history();
    push(
        &mut h,
        at(2026, 10, 4, 12, 34),
        Kind::Error,
        "Error",
        r"C:\ssp\ghost\nameless",
        "nameless error",
    );
    let (body, is_error, _) = ask_with(
        &h,
        Some(&g),
        &full(None, Some(r"C:\ssp\ghost\nameless"), None, None),
    );
    assert!(!is_error);
    assert_eq!(ids(&body), [11]);
}

#[test]
fn unmatched_empty_and_no_ghost_are_cannot_find() {
    let g = emily();
    let cases: [(Option<&ActiveGhost>, &str); 6] = [
        (Some(&g), "Nobody"),
        // 大文字小文字の違い・フォルダ名だけは当たらない（他のツールと同じ解決）。
        (Some(&g), "emily/phase4.5"),
        (Some(&g), "emily4"),
        // 空は解決を呼ばずに Cannot find（解決へ渡すと 0 体で Specified ghost is not active になる）。
        (Some(&g), ""),
        (None, ""),
        // 0 体で名前あり。
        (None, "Emily/Phase4.5"),
    ];
    for (active, name) in cases {
        let (body, is_error, _) = ask_with(&history(), active, &full(None, Some(name), None, None));
        assert_eq!(
            body,
            "NG:Cannot find active ghost from specified name",
            "{name:?} {}",
            active.is_some()
        );
        assert!(is_error, "{name:?}");
    }
}

#[test]
fn matched_ghost_without_records_is_no_log_entries() {
    let g = emily();
    let (body, is_error, _) = ask_with(
        &history(),
        Some(&g),
        &full(Some("status"), Some("Emily/Phase4.5"), None, None),
    );
    assert_eq!(body, NO_ENTRIES);
    assert!(!is_error);
}

#[test]
fn without_ghost_name_records_outside_any_ghost_are_returned() {
    // 起動中のゴーストが居ても、ghost_name が無ければ [SYSTEM] の記録も返る。
    let g = emily();
    let (body, _, _) = ask_with(
        &history(),
        Some(&g),
        &full(Some("script"), None, None, None),
    );
    assert_eq!(ids(&body), [2, 7]);
}

#[test]
fn unknown_log_type_comes_before_ghost_name() {
    for active in [None, Some(emily())] {
        let (body, is_error, asked) = ask_with(
            &history(),
            active.as_ref(),
            &full(Some("bogus"), Some("Nobody"), None, None),
        );
        assert_eq!(body, format!("NG:{UNKNOWN_LOG_TYPE}"));
        assert!(is_error);
        assert_eq!(asked, None);
    }
}

#[test]
fn filters_apply_in_order_kind_ghost_since_max() {
    let g = emily();
    // script（2・7）→ Emily（2）→ 最後の 1 件＝2。件数を先に絞ると 7 だけ残り、ゴーストで 0 件になる。
    let (body, _, _) = ask_with(
        &history(),
        Some(&g),
        &full(Some("script"), Some("Emily/Phase4.5"), None, Some(1)),
    );
    assert_eq!(ids(&body), [2]);
    // error（1・5・8・10）→ Emily（10）→ 番号 9 より後（10）→ 1 件。
    let (body, _, _) = ask_with(
        &history(),
        Some(&g),
        &full(None, Some("Emily/Phase4.5"), Some(9), Some(1)),
    );
    assert_eq!(ids(&body), [10]);
}

// ---- 入口（要件 3.4・3.5） ----

/// 空の World で入口を呼び、その場で届いた答え（まだなら None）。
fn handle_now(ghost_name: Option<&str>) -> Option<ToolOutcome> {
    let mut world = World::new();
    let call = full(None, ghost_name, None, None);
    let (request, pending) = ToolRequest::new(ToolCall::GetLog(call.clone()));
    let ToolRequest { reply, .. } = request;
    handle(&mut world, call, reply);
    pending.try_answer().ok().flatten().map(|a| a.outcome)
}

#[test]
fn empty_world_without_ghost_name_answers_at_once_without_error() {
    let outcome = handle_now(None).expect("その場で答える");
    // 置き場はプロセスで共有なので本文は他のテストの積み方次第。失敗でないことだけを見る。
    assert!(!outcome.is_error);
}

#[test]
fn empty_world_with_ghost_name_answers_cannot_find_at_once() {
    let (body, is_error) = text(handle_now(Some("Emily/Phase4.5")).expect("その場で答える"));
    assert_eq!(body, "NG:Cannot find active ghost from specified name");
    assert!(is_error);
}
