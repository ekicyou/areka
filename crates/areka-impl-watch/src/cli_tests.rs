//! 入口の決定論テスト: 使い方の誤りの出し方・置き場所の失敗・`--help` の出し先・終了コード。
//! 引数の読みと `--help` の本文のテストは `cli_args_tests.rs` に在る。
//!
//! 環境変数の値は引数で渡し（プロセスの環境変数は触らない）、標準出力と標準エラーは
//! 差し替えた書き手で受ける。置き場所の向け先はワークツリーの `target\` の下の一時フォルダだけ。

use std::ffi::OsString;
#[cfg(windows)]
use std::path::Component;
use std::path::PathBuf;

use temp_path_kit::TempPath;

use super::args::Opt;
use super::test_support::{args, entries, line, run_captured};
use super::{Outcome, exit_code, usage};
use crate::error::WatchError;

use Opt::{Id, Repo};

// ---- 使い方の誤り ----

#[test]
fn a_usage_error_is_one_ascii_line_on_stderr_even_for_japanese_arguments() {
    for words in [
        args(&[]),
        args(&["見張り"]),
        args(&["leave", "離脱"]),
        args(&["leave", "--識別", "a"]),
        args(&["leave", "--id", "見張り"]),
        args(&["leave", "--id", "a\nb"]),
        args(&["tick", "一行目\n二行目"]),
    ] {
        let (result, out, err) = run_captured(&words, None);
        assert!(
            matches!(result, Ok(Outcome::Usage)),
            "{words:?}: {result:?}"
        );
        assert_eq!(out, "", "{words:?}");
        assert!(err.is_ascii(), "ASCII の外の字: {err:?}");
        assert!(err.ends_with('\n'), "{err:?}");
        assert_eq!(err.matches('\n').count(), 1, "1 行でない: {err:?}");
        assert!(err.starts_with("usage error: "), "{err:?}");
        assert!(err.contains("--help"), "{err:?}");
    }
    // 打たれた綴りは逃がして映す。
    let (_, _, err) = run_captured(&args(&["見"]), None);
    assert!(err.contains("unknown command '\\u{898b}'"), "{err:?}");
}

#[test]
fn a_usage_error_reads_and_writes_nothing() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = root.child("home");
    for words in [
        args(&["join"]),
        args(&["leave"]),
        args(&["leave", "--id", "a b"]),
        args(&["status", "--wait"]),
    ] {
        let (result, _, _) = run_captured(&words, Some(home.clone().into_os_string()));
        assert!(
            matches!(result, Ok(Outcome::Usage)),
            "{words:?}: {result:?}"
        );
        // 置き場所の解決（フォルダを作る）にも進まない。
        assert!(!home.exists(), "{words:?}: 置き場所を作った");
    }
    assert_eq!(entries(root.path()), Vec::<PathBuf>::new());
}

// ---- 環境変数 ----

#[test]
fn a_missing_or_empty_home_fails_in_at_most_two_ascii_lines_and_creates_nothing() {
    let cwd = std::env::current_dir().expect("カレントが分かる");
    let before = entries(&cwd);
    for home in [None, Some(""), Some("  ")] {
        for words in [args(&["status"]), line("watch", &[Id, Repo])] {
            let (result, out, err) = run_captured(&words, home.map(OsString::from));
            assert!(
                matches!(result, Err(WatchError::HomeUnset)),
                "{home:?} {words:?}: {result:?}"
            );
            assert_eq!(exit_code(&result), 1);
            assert_eq!(out, "");
            assert!(err.is_ascii(), "ASCII の外の字: {err:?}");
            assert!(err.contains("AREKA_IMPL_WATCH_HOME"), "{err:?}");
            assert!(err.ends_with('\n'), "{err:?}");
            assert!((1..=2).contains(&err.lines().count()), "{err:?}");
        }
    }
    assert_eq!(entries(&cwd), before, "既定の場所へ倒れて何かを作った");
}

#[test]
fn a_home_that_cannot_be_created_fails_in_at_most_two_ascii_lines_and_creates_nothing() {
    let root = TempPath::under_target("impl-watch-cli");
    let file = root.child("file.txt");
    std::fs::write(&file, b"keep").expect("書ける");
    // 親がファイル。道筋には ASCII の外の字も入れる。
    let home = file.join("置き場");

    let (result, out, err) = run_captured(&args(&["status"]), Some(home.into_os_string()));

    assert!(
        matches!(result, Err(WatchError::HomeNotCreatable { .. })),
        "{result:?}"
    );
    assert_eq!(exit_code(&result), 1);
    assert_eq!(out, "");
    assert!(err.is_ascii(), "ASCII の外の字: {err:?}");
    assert!(
        err.contains("AREKA_IMPL_WATCH_HOME cannot be created"),
        "{err:?}"
    );
    assert!((1..=2).contains(&err.lines().count()), "{err:?}");
    assert_eq!(std::fs::read(&file).expect("読める"), b"keep");
    assert_eq!(entries(root.path()), vec![file]);
}

/// 絶対パスでない値（相対の形）は、何も作らずに 1。使い方の誤りはそれより先で、`--help` は
/// 置き場所を決めない。値は、断られなければ一時フォルダの中に落ちるもの（判定が壊れていても
/// `target\` の外には何も作られない）。ほかの形は `home_tests.rs` が確かめる。
#[cfg(windows)]
#[test]
fn a_home_that_is_not_absolute_fails_in_at_most_two_ascii_lines_and_creates_nothing() {
    let root = TempPath::under_target("impl-watch-cli");
    // 道筋には ASCII の外の字も入れる。
    let dir = root.child("置き場");
    let cwd = std::env::current_dir().expect("カレントが分かる");
    // カレントから見た相対の道筋（`..\..\target\…`）。
    let shared = cwd.components().zip(dir.components());
    let shared = shared.take_while(|(here, there)| here == there).count();
    let up = cwd.components().skip(shared).map(|_| Component::ParentDir);
    let home: PathBuf = up.chain(dir.components().skip(shared)).collect();
    assert!(!home.is_absolute(), "前提: {home:?}");
    // 較正: 断られなければ、この値のフォルダは一時フォルダの中にできる。
    assert_eq!(std::path::absolute(&home).expect("綴れる"), dir);
    let home = home.into_os_string();

    for words in [args(&["status"]), line("watch", &[Id, Repo])] {
        let (result, out, err) = run_captured(&words, Some(home.clone()));
        assert!(
            matches!(result, Err(WatchError::HomeNotAbsolute { .. })),
            "{words:?}: {result:?}"
        );
        assert_eq!(exit_code(&result), 1);
        assert_eq!(out, "");
        assert!(err.is_ascii(), "ASCII の外の字: {err:?}");
        assert!(
            err.starts_with("AREKA_IMPL_WATCH_HOME must be an absolute path"),
            "{err:?}"
        );
        assert!(err.ends_with('\n'), "{err:?}");
        assert!((1..=2).contains(&err.lines().count()), "{err:?}");
    }

    let (result, _, err) = run_captured(&args(&["leave"]), Some(home.clone()));
    assert!(matches!(result, Ok(Outcome::Usage)), "{result:?}");
    assert!(!err.contains("AREKA_IMPL_WATCH_HOME"), "{err:?}");
    let (result, out, err) = run_captured(&args(&["--help"]), Some(home));
    assert!(matches!(result, Ok(Outcome::Done)), "{result:?}");
    assert_eq!((out, err), (format!("{}\n", usage()), String::new()));

    assert_eq!(
        entries(root.path()),
        Vec::<PathBuf>::new(),
        "断った値のフォルダを作った"
    );
}

#[test]
fn a_usage_error_wins_over_a_missing_home() {
    let (result, _, err) = run_captured(&args(&["leave"]), None);
    assert!(matches!(result, Ok(Outcome::Usage)), "{result:?}");
    assert!(!err.contains("AREKA_IMPL_WATCH_HOME"), "{err:?}");
}

// ---- 使い方 ----

#[test]
fn help_prints_the_usage_to_stdout_and_needs_no_home() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = root.child("home");
    for value in [None, Some(home.clone().into_os_string())] {
        let (result, out, err) = run_captured(&args(&["--help"]), value);
        assert!(matches!(result, Ok(Outcome::Done)), "{result:?}");
        assert_eq!(exit_code(&result), 0);
        assert_eq!(out, format!("{}\n", usage()));
        assert_eq!(err, "");
    }
    assert!(!home.exists(), "--help が置き場所を作った");
}

// ---- 終了コード ----

#[test]
fn outcomes_map_to_the_four_exit_codes() {
    assert_eq!(exit_code(&Ok(Outcome::Done)), 0);
    assert_eq!(exit_code(&Err(WatchError::LockBusy)), 1);
    assert_eq!(exit_code(&Ok(Outcome::Usage)), 2);
    assert_eq!(exit_code(&Ok(Outcome::NotApplied)), 3);
}
