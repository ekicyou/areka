//! 副手続きの振り分けと使い方の表示を釘付けする（設計「入口 / cli」・要件 6.12）。
//!
//! ここで確かめるのは 4 つ。⑴ 8 つの副手続きの名前が 1 つずつ綴りどおりに通ること、
//! ⑵ 引数が無いときと知らない名前のときに使い方を求めること、⑶ 使い方の本文が
//! 8 つ全部を並べること、⑷ 名前の後ろに余計な引数を付けたら断ること。
//!
//! 期待値の本文は実装の定数を引かず、独立した文字列として書く（実装と同じ値を
//! 参照すると、綴りが一斉に変わっても緑のままになるため）。
//!
//! このファイルはファイルを 1 つも作らず、一時ディレクトリも使わず、スナップショットも
//! 読まない（要件 6.2・設計 File Structure Plan）。

use super::{
    Outcome, SUBCOMMANDS, extra_arguments_notice, lookup, run_reporting_to,
    unknown_subcommand_notice, usage,
};

/// 文字列の引数列を組み立てる小道具（実行ファイル名は既に落とされている前提）。
fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

/// 振り分けを 1 度走らせ、結果と断りの本文を受け取る。
fn run_and_read(line: &[&str]) -> (Result<Outcome, crate::SurveyError>, String) {
    let mut notices: Vec<u8> = Vec::new();
    let outcome = run_reporting_to(&args(line), &mut notices);
    let body = String::from_utf8(notices).expect("断りの本文が UTF-8 でない");
    (outcome, body)
}

#[test]
fn each_of_the_eight_subcommand_names_is_accepted_verbatim() {
    for name in [
        "catalog",
        "ledger-init",
        "report",
        "report-summary",
        "check",
        "evidence",
        "candidates",
        "diff",
    ] {
        let found = lookup(name).unwrap_or_else(|| panic!("副手続き {name} が振り分け表に無い"));
        assert_eq!(found.name, name, "振り分け表の名前が {name} と違う");
    }
}

#[test]
fn the_accepted_names_are_exactly_these_eight() {
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
fn the_notice_for_an_unknown_subcommand_repeats_the_name_that_was_typed() {
    let one = unknown_subcommand_notice("catalogue");
    assert!(
        one.contains("catalogue"),
        "打ち間違えた綴りが本文に無い: {one}"
    );
    let other = unknown_subcommand_notice("chekc");
    assert!(
        other.contains("chekc"),
        "打ち間違えた綴りが本文に無い: {other}"
    );
    // 打った綴りをそのまま返すこと（どの綴りでも同じ本文になるなら手掛かりが無い）。
    assert_ne!(one, other, "打ち間違いの綴りが本文に映っていない");
}

#[test]
fn extra_arguments_after_a_known_subcommand_ask_for_the_usage_text() {
    // 8 つの副手続きはどれも引数を取らない（設計「入口 / cli」の表）。
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
fn a_subcommand_on_its_own_draws_no_complaint() {
    // 断りが出る側だけを確かめると、何にでも断りを出す形に壊れても緑になる。
    let (_outcome, notices) = run_and_read(&["evidence"]);
    assert_eq!(notices, "", "正しく打っているのに断られている: {notices}");
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

副手続きは 8 つ。いずれも引数を取らない。

  catalog         正典のカタログを作り直す（スナップショットが要る）
  ledger-init     初期の台帳を作って既存の台帳へ差し込む
  report          ドメイン別の報告 4 本を作り直す
  report-summary  全体の報告を作り直す
  check           台帳と正典とソースの食い違いを調べる
  evidence        項目ごとの証拠を並べる
  candidates      手掛かりの候補を並べる
  diff            今のカタログと新しいスナップショットの差を並べる（スナップショットが要る）

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

// 足場のテスト `every_unwired_name_reaches_its_own_not_wired_body` はタスク 6.3 で
// 役目を終えた（8 つとも中身が繋がったので、`SurveyError::NotWired` ごと退役した）。
// 名前と中身の結び付きは、いまは 8 つの副手続き自身の在中テストと
// `tests/cli_streams.rs` が受け持つ——ここで走らせるとファイルを読み書きしてしまう
// （要件 6.2 が禁じる）。
