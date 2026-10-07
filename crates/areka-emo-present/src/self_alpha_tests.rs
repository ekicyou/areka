//! 透過の宣言の読みのテスト（spec: areka-P0-self-alpha-declaration 要件 1.2〜1.6・2.2〜2.6・7.1・7.2・8.8）。
//!
//! 値の読みの表と、記録の 3 つの場合（宣言あり・行なし・読めない値）の行数を固定する。
//! ログの捕捉は硬化機構の唯一の定義元 `log_capture_kit::capture` に任せる（呼んだスレッドの分だけ拾う）。

use std::path::Path;

use areka_emo_atlas::UseSelfAlpha;
use log_capture_kit::CapturedEvent;

use super::{parse, read_use_self_alpha};

const KIND: &str = "shell";
const KEY: &str = "seriko.use_self_alpha";

/// 値の読みの表（要件 1.2〜1.6・2.2〜2.6）。前後の空白を除き、英字の大小を区別しない。
#[test]
fn parse_reads_only_the_four_canonical_spellings() {
    let table: &[(&str, Option<UseSelfAlpha>)] = &[
        ("1", Some(UseSelfAlpha::On)),
        ("true", Some(UseSelfAlpha::On)),
        ("TRUE", Some(UseSelfAlpha::On)),
        ("full", Some(UseSelfAlpha::Full)),
        ("Full", Some(UseSelfAlpha::Full)),
        (" 0 ", Some(UseSelfAlpha::Off)),
        ("false", None),
        ("", None),
        ("2", None),
    ];
    for &(value, expected) in table {
        assert_eq!(parse(value), expected, "値 {value:?} の読み");
    }
}

/// `self_alpha:` の記録だけを level ごとに数える。
fn count(events: &[CapturedEvent], level: tracing::Level) -> usize {
    events
        .iter()
        .filter(|e| e.level == level && e.message().starts_with("self_alpha:"))
        .count()
}

/// `info!` がちょうど 1 行で、欄が期待どおりであることを確かめる。
fn assert_one_info(events: &[CapturedEvent], treatment: &str, declared: bool) {
    let infos: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::INFO && e.message().starts_with("self_alpha:"))
        .collect();
    assert_eq!(infos.len(), 1, "info! は 1 行: {events:?}");
    let ev = infos[0];
    assert_eq!(ev.field_str("kind"), Some(KIND), "{ev:?}");
    assert_eq!(ev.field_str("key"), Some(KEY), "{ev:?}");
    assert_eq!(ev.field("dir"), Some("shells/master"), "{ev:?}");
    assert_eq!(ev.field_str("treatment"), Some(treatment), "{ev:?}");
    assert_eq!(
        ev.field("declared"),
        Some(if declared { "true" } else { "false" }),
        "{ev:?}"
    );
}

fn read(descript: Option<&str>) -> (UseSelfAlpha, Vec<CapturedEvent>) {
    log_capture_kit::capture(|| {
        read_use_self_alpha(KIND, KEY, Path::new("shells/master"), descript)
    })
}

/// 宣言あり: 値どおりの扱いを返し、info! が 1 行・warn! は無い（要件 1.3・7.1）。
#[test]
fn declared_value_logs_one_info_and_no_warn() {
    let (got, events) = read(Some("charset,UTF-8\r\nseriko.use_self_alpha,full\r\n"));
    assert_eq!(got, UseSelfAlpha::Full);
    assert_one_info(&events, "full", true);
    assert_eq!(count(&events, tracing::Level::WARN), 0, "{events:?}");
}

/// 同じキーが 2 行なら後の行が勝つ。
#[test]
fn later_line_wins() {
    let (got, _) = read(Some("seriko.use_self_alpha,0\nseriko.use_self_alpha,1\n"));
    assert_eq!(got, UseSelfAlpha::On);
    let (got, events) = read(Some("seriko.use_self_alpha,1\nseriko.use_self_alpha,0\n"));
    assert_eq!(got, UseSelfAlpha::Off);
    assert_one_info(&events, "0", true);
}

/// 行なし（本文が読めない場合を含む）: 宣言なしとし、info! が 1 行・warn! は無い（要件 1.5・7.1）。
#[test]
fn missing_line_logs_one_info_and_no_warn() {
    for descript in [None, Some("charset,UTF-8\n")] {
        let (got, events) = read(descript);
        assert_eq!(got, UseSelfAlpha::Undeclared, "{descript:?}");
        assert_one_info(&events, "none", false);
        assert_eq!(count(&events, tracing::Level::WARN), 0, "{events:?}");
    }
}

/// 読めない値: 宣言なしとし、値つきの warn! が 1 行と info! が 1 行（要件 1.6・7.2）。
#[test]
fn unreadable_value_logs_one_warn_with_value_and_one_info() {
    let (got, events) = read(Some("seriko.use_self_alpha,false\n"));
    assert_eq!(got, UseSelfAlpha::Undeclared);
    assert_one_info(&events, "none", false);
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message().starts_with("self_alpha:"))
        .collect();
    assert_eq!(warns.len(), 1, "warn! は 1 行: {events:?}");
    assert_eq!(warns[0].field_str("value"), Some("false"), "{:?}", warns[0]);
    assert_eq!(warns[0].field_str("key"), Some(KEY), "{:?}", warns[0]);
}

/// `use_input_alpha` だけの本文は行なしと同じ（要件 8.8）。
#[test]
fn use_input_alpha_alone_is_same_as_missing_line() {
    let (got, events) = read(Some("use_input_alpha,1\n"));
    assert_eq!(got, UseSelfAlpha::Undeclared);
    assert_one_info(&events, "none", false);
    assert_eq!(count(&events, tracing::Level::WARN), 0, "{events:?}");
}
