//! 副手続きの振り分けと使い方の表示を釘付けする（設計「入口 / cli」・要件 6.12）。
//!
//! ここで確かめるのは 5 つ。⑴ 振り分け表の名前が 9 つの綴りどおりであること、
//! ⑵ 引数が無いときと知らない名前のときに使い方を求めること、⑶ 使い方の本文が
//! 9 つ全部を並べること、⑷ 名前の後ろに余計な引数を付けたら断ること、⑸ 標準出力へ
//! 書けなかったことを置き場つきの失敗として返すこと。
//!
//! 期待値の本文は実装の定数を引かず、独立した文字列として書く（実装と同じ値を
//! 参照すると、綴りが一斉に変わっても緑のままになるため）。
//!
//! このファイルはファイルを 1 つも作らず、一時ディレクトリも使わず、スナップショットも
//! 読まない（要件 6.2・設計 File Structure Plan）。
//!
//! 副手続きの中身はここで走らせない。名前と中身の結び付きを走らせて見るのは
//! `tests/cli_streams.rs` の `catalog`・`candidates` だけで、残る 7 つは表を読んで確かめる
//! （`ledger-init`・`report`・`report-summary`・`priority-apply` は台帳や報告を書き、
//! `check`・`evidence`・`diff` は repo の中身かスナップショットに寄りかかる。
//! `tests/cli_streams.rs` 冒頭の線）。各副手続きの在中テストは中身の関数を直に呼ぶので、
//! 表を通らない。

use std::io::{self, Write};

use super::{
    Outcome, SUBCOMMANDS, extra_arguments_notice, lookup, run_reporting_to, usage, write_stdout,
};
use crate::SurveyError;

/// 文字列の引数列を組み立てる小道具（実行ファイル名は既に落とされている前提）。
fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

/// 振り分けを 1 度走らせ、結果と断りの本文を受け取る。
fn run_and_read(line: &[&str]) -> (Result<Outcome, SurveyError>, String) {
    let mut notices: Vec<u8> = Vec::new();
    let outcome = run_reporting_to(&args(line), &mut notices);
    let body = String::from_utf8(notices).expect("断りの本文が UTF-8 でない");
    (outcome, body)
}

#[test]
fn the_accepted_names_are_exactly_these_nine() {
    let actual: Vec<&str> = SUBCOMMANDS.iter().map(|sub| sub.name).collect();
    assert_eq!(
        actual,
        vec![
            "catalog",
            "ledger-init",
            "report",
            "report-summary",
            "check",
            "evidence",
            "candidates",
            "diff",
            "priority-apply",
        ],
        "振り分け表に載っている名前が設計の表と違う"
    );
}

#[test]
fn spellings_that_are_close_but_wrong_are_not_accepted() {
    // 通る綴りだけを確かめると、表が何でも受け付ける形に壊れても緑になる。
    // 通らない綴りと対で置く。
    for wrong in [
        "Catalog",
        "catalogue",
        "ledger_init",
        "ledgerinit",
        "report_summary",
        "reports",
        "checked",
        "priority_apply",
        "priorityapply",
        "--help",
        "",
    ] {
        assert!(
            lookup(wrong).is_none(),
            "副手続きでない綴り {wrong} が受け付けられた"
        );
    }
}

#[test]
fn no_argument_asks_for_the_usage_text_and_says_nothing_else() {
    let (outcome, notices) = run_and_read(&[]);
    let outcome = outcome.expect("引数が無いだけで失敗にされた");
    assert_eq!(
        outcome,
        Outcome::Usage,
        "引数が無いのに使い方を求めなかった"
    );
    // 打ち間違いが無いのだから断る筋合いも無い（使い方は呼び手が出す）。
    assert_eq!(notices, "", "断る理由が無いのに何か言っている: {notices}");
}

#[test]
fn an_unknown_subcommand_asks_for_the_usage_text_and_repeats_the_name() {
    let (outcome, notices) = run_and_read(&["nosuchthing"]);
    let outcome = outcome.expect("知らない名前が失敗として返された");
    assert_eq!(
        outcome,
        Outcome::Usage,
        "知らない副手続きなのに使い方を求めなかった"
    );
    assert!(
        notices.contains("nosuchthing"),
        "打った綴りが断りの本文に出ていない: {notices}"
    );
}

#[test]
fn extra_arguments_after_a_known_subcommand_ask_for_the_usage_text() {
    // 9 つの副手続きはどれも引数を取らない（設計「入口 / cli」の表）。
    // 黙って捨てると、指定したつもりの利用者が気づけない。
    for line in [
        vec!["check", "--verbose"],
        vec!["report", "shiori"],
        vec!["catalog", "a", "b"],
    ] {
        let (outcome, notices) = run_and_read(&line);
        let outcome = outcome.expect("余計な引数が失敗として返された");
        assert_eq!(
            outcome,
            Outcome::Usage,
            "余計な引数が黙って捨てられた: {line:?}"
        );
        for leftover in &line[1..] {
            assert!(
                notices.contains(leftover),
                "余計な引数 {leftover} が断りの本文に出ていない: {notices}"
            );
        }
    }
}

#[test]
fn the_notice_for_extra_arguments_names_the_subcommand_and_the_leftovers() {
    let notice = extra_arguments_notice("check", &["--verbose".to_string(), "x".to_string()]);
    for needle in ["check", "--verbose", "x"] {
        assert!(
            notice.contains(needle),
            "余計な引数の手掛かり {needle} が本文に無い: {notice}"
        );
    }
}

#[test]
fn the_usage_text_is_written_out_word_for_word() {
    // 表示される本文そのものが契約なので、期待値は独立した文字列で書く。
    let expected = "\
使い方: cargo run -p ukadoc-survey -- <副手続き>

副手続きは 9 つ。いずれも引数を取らない。

  catalog         正典のカタログを作り直す（スナップショットが要る）
  ledger-init     初期の台帳を作って既存の台帳へ差し込む
  report          ドメイン別の報告 4 本を作り直す
  report-summary  全体の報告を作り直す
  check           台帳と正典とソースの食い違いを調べる
  evidence        項目ごとの証拠を並べる
  candidates      手掛かりの候補を並べる
  diff            今のカタログと新しいスナップショットの差を並べる（スナップショットが要る）
  priority-apply  確定した優先度を台帳 4 本へ書き戻す

スナップショットの場所は環境変数 AREKA_UKADOC_SNAPSHOT で指定できる。";
    assert_eq!(usage(), expected, "使い方の本文が変わっている");
}

#[test]
fn the_usage_text_lists_every_name_in_the_dispatch_table() {
    // 表に副手続きを足したのに使い方へ書き忘れる、を防ぐ。
    let text = usage();
    for sub in SUBCOMMANDS.iter() {
        assert!(
            text.contains(sub.name),
            "使い方に副手続き {} が載っていない: {text}",
            sub.name
        );
    }
}

/// 書くたびにパイプが閉じたと答える書き手（`| head -n 5` の後ろの姿）。
struct ClosedPipe;

impl Write for ClosedPipe {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[test]
fn a_closed_pipe_on_standard_output_comes_back_as_a_failure_naming_standard_output() {
    // 黙って捨てると終了コード 0 で結果が欠け、`print!` のままだと panic して 101 になる。
    let err = write_stdout(&mut ClosedPipe, "所見 0 件").expect_err("書けないのに成功した");
    match err {
        SurveyError::Io { path, .. } => {
            assert_eq!(path, "<標準出力>", "失敗の置き場が標準出力でない");
        }
        other => panic!("パイプが閉じた失敗が別の失敗に化けている: {other}"),
    }

    // 対の腕: 書ける先には本文を 1 バイトも変えずに渡す。
    let mut sink: Vec<u8> = Vec::new();
    write_stdout(&mut sink, "所見 0 件").expect("書ける先で失敗した");
    assert_eq!(sink, "所見 0 件".as_bytes(), "書いた本文が変わっている");
}
