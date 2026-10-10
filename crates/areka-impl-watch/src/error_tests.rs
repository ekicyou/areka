//! 失敗の型の決定論テスト: 文が ASCII の 1 行であること、OS と JSON の失敗の写しに
//! 元の文（ローカル言語でありうる）が混ざらないこと。

use std::io;
use std::path::Path;

use temp_path_kit::TempPath;

use super::{WatchError, escape_path};

/// 全部の種類の見本。ASCII の外の字が来うる所（置き場所の道筋・OS と JSON の元の文）には
/// 日本語を入れてある。
fn samples() -> Vec<WatchError> {
    let os = io::Error::new(
        io::ErrorKind::PermissionDenied,
        "アクセスが拒否されました。",
    );
    let json = serde_json::from_str::<u64>("\"日本語\"").expect_err("数でないので読めない");
    vec![
        WatchError::HomeUnset,
        WatchError::home_not_creatable(Path::new("C:\\利用者\\置き場"), &os),
        WatchError::LockBusy,
        WatchError::VersionMismatch { found: 2, known: 1 },
        WatchError::Broken,
        WatchError::AlreadyRunning {
            id: "areka-P0-impl-watch".to_string(),
            kind: "watch",
        },
        WatchError::io("write state.json", &os),
        WatchError::from(json),
    ]
}

/// 種類の通し番号。種類を足すとここが建たなくなり、見本への足し忘れに気付く。
fn variant_index(err: &WatchError) -> usize {
    match err {
        WatchError::HomeUnset => 0,
        WatchError::HomeNotCreatable { .. } => 1,
        WatchError::LockBusy => 2,
        WatchError::VersionMismatch { .. } => 3,
        WatchError::Broken => 4,
        WatchError::AlreadyRunning { .. } => 5,
        WatchError::Io { .. } => 6,
        WatchError::Json(_) => 7,
    }
}

#[test]
fn every_message_is_one_ascii_line() {
    let samples = samples();
    let covered: Vec<usize> = samples.iter().map(variant_index).collect();
    assert_eq!(
        covered,
        (0..8).collect::<Vec<_>>(),
        "見本が全部の種類を踏む"
    );
    for err in &samples {
        let text = err.to_string();
        assert!(!text.is_empty(), "{err:?}");
        assert!(text.is_ascii(), "ASCII の外の字: {text}");
        assert!(!text.contains(['\r', '\n']), "1 行でない: {text:?}");
    }
}

#[test]
fn messages_spell_the_design_text() {
    let texts: Vec<String> = samples().iter().map(ToString::to_string).collect();
    assert_eq!(
        texts[1],
        "AREKA_IMPL_WATCH_HOME cannot be created: C:\\\\u{5229}\\u{7528}\\u{8005}\\\\u{7f6e}\\u{304d}\\u{5834}: PermissionDenied (os error 0)"
    );
    assert_eq!(
        texts[3],
        "state file version mismatch: file has 2, this exe knows 1. Do not mix old and new exes; see doc/impl-watch.md"
    );
    assert_eq!(
        texts[5],
        "a watch wait for areka-P0-impl-watch is already running"
    );
    assert_eq!(
        texts[6],
        "io write state.json: PermissionDenied (os error 0)"
    );
}

#[test]
fn io_failure_spells_kind_and_os_code_only() {
    // 本物の OS の失敗: 日本語の Windows では Display が「指定されたファイルが見つかりません。」になる。
    let root = TempPath::under_target("impl-watch-error");
    let os = std::fs::File::open(root.child("missing.json")).expect_err("無いファイルは開けない");
    assert_eq!(
        WatchError::io("open state.json", &os).to_string(),
        "io open state.json: NotFound (os error 2)"
    );
}

#[test]
fn json_failure_spells_line_and_column_only() {
    let json = serde_json::from_str::<u64>("\n  \"日本語\"").expect_err("数でないので読めない");
    // 較正: 元の文には入力の日本語が写っている（写っていなければこのテストは何も確かめていない）。
    assert!(!json.to_string().is_ascii(), "{json}");
    let expected = format!("json: line {} column {}", json.line(), json.column());
    assert_eq!(json.line(), 2);
    assert_eq!(WatchError::from(json).to_string(), expected);
}

#[test]
fn escape_path_keeps_printable_ascii_and_escapes_the_rest() {
    assert_eq!(
        escape_path("C:\\Users\\a b\\.areka-impl-watch"),
        "C:\\Users\\a b\\.areka-impl-watch"
    );
    assert_eq!(escape_path("あ\n"), "\\u{3042}\\u{a}");
}
