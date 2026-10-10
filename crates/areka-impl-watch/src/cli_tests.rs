//! 引数の表・使い方・誤りの扱いの決定論テスト。
//!
//! 環境変数の値は引数で渡し（プロセスの環境変数は触らない）、標準出力と標準エラーは
//! 差し替えた書き手で受ける。置き場所の向け先はワークツリーの `target\` の下の一時フォルダだけ。
//!
//! [`DESIGN`] は設計の「コマンドの表」の写しで、本番の表（`COMMANDS`）とは別に持つ
//! （本番の表を書き換えると、このファイルのテストが赤になる）。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{COMMANDS, Command, Invocation, Opt, Outcome, exit_code, parse, run_with, usage};
use crate::error::WatchError;
use crate::plan;

use Opt::{Bug, Id, Name, Pr, Purpose, Repo, Sha, Spec, Wait};

/// 設計のコマンドの表: 名前・要る引数・任意の引数・`--wait` を取るか（14 行・同じ並び）。
const DESIGN: [(&str, &[Opt], &[Opt], bool); 14] = [
    ("watch", &[Id, Repo], &[Name], false),
    ("merge", &[Id, Repo, Spec], &[Bug, Name], true),
    ("merged", &[Id, Pr, Sha], &[], false),
    ("loadtest", &[Id, Repo, Purpose], &[Name], true),
    ("loadrunning", &[Id, Repo, Purpose], &[Name], false),
    ("loaddone", &[Id], &[], false),
    ("stopped", &[Id], &[], true),
    ("resume", &[Id], &[], false),
    ("unstop", &[], &[Id], false),
    ("cancel", &[Id], &[], false),
    ("leave", &[Id], &[], false),
    ("tick", &[], &[], false),
    ("status", &[], &[], false),
    ("clear", &[], &[], false),
];

const ALL_OPTIONS: [Opt; 9] = [Id, Repo, Spec, Name, Purpose, Pr, Sha, Bug, Wait];

/// 引数の綴りと、形に合う値（値を取らないものは `None`）。
fn sample(opt: Opt) -> (&'static str, Option<&'static str>) {
    match opt {
        Id => ("--id", Some("a1")),
        Repo => ("--repo", Some("areka")),
        Spec => ("--spec", Some("areka-P0-impl-watch")),
        Name => ("--name", Some("見張り役")),
        Purpose => ("--purpose", Some("負荷の計測")),
        Pr => ("--pr", Some("281")),
        Sha => ("--sha", Some("414d43eb")),
        Bug => ("--bug", None),
        Wait => ("--wait", None),
    }
}

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// コマンドの名前の後ろに、引数を形に合う値つきで並べる。
fn line(name: &str, opts: &[Opt]) -> Vec<String> {
    let mut words = vec![name];
    for opt in opts {
        let (flag, value) = sample(*opt);
        words.push(flag);
        words.extend(value);
    }
    args(&words)
}

/// 誤りの文（使い方の誤りでなければ落ちる）。
fn refusal(words: &[String]) -> String {
    match parse(words) {
        Err(err) => err.0,
        Ok(invocation) => panic!("通ってしまった: {words:?} -> {invocation:?}"),
    }
}

/// 差し替えた書き手で走らせ、結果・標準出力・標準エラーを返す。
fn run_captured(
    words: &[String],
    home: Option<OsString>,
) -> (Result<Outcome, WatchError>, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let result = run_with(words, home, &mut out, &mut err);
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).expect("UTF-8 で出る");
    (result, text(out), text(err))
}

/// フォルダの直下に在るものの一覧（並べ替え済み）。
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}

fn s(text: &str) -> String {
    text.to_owned()
}

// ---- 表 ----

#[test]
fn the_table_is_the_design_table() {
    let table: Vec<(&str, &[Opt], &[Opt], bool)> = COMMANDS
        .iter()
        .map(|spec| (spec.name, spec.required, spec.optional, spec.wait))
        .collect();
    assert_eq!(table, DESIGN);
}

#[test]
fn every_command_parses_in_its_full_form() {
    let (id, repo, name) = (s("a1"), s("areka"), Some(s("見張り役")));
    let purpose = s("負荷の計測");
    let change = Command::Change;
    let expected = [
        (
            Command::Watch {
                id: id.clone(),
                repo: repo.clone(),
                name: name.clone(),
            },
            false,
        ),
        (
            change(plan::Command::Merge {
                id: id.clone(),
                name: name.clone(),
                repo: repo.clone(),
                spec: s("areka-P0-impl-watch"),
                bug: true,
            }),
            true,
        ),
        (
            change(plan::Command::Merged {
                id: id.clone(),
                pr: s("281"),
                sha: s("414d43eb"),
            }),
            false,
        ),
        (
            change(plan::Command::LoadTest {
                id: id.clone(),
                name: name.clone(),
                repo: repo.clone(),
                purpose: purpose.clone(),
            }),
            true,
        ),
        (
            change(plan::Command::LoadRunning {
                id: id.clone(),
                name: name.clone(),
                repo: repo.clone(),
                purpose: purpose.clone(),
            }),
            false,
        ),
        (change(plan::Command::LoadDone { id: id.clone() }), false),
        (change(plan::Command::Stopped { id: id.clone() }), true),
        (Command::Resume { id: id.clone() }, false),
        (
            change(plan::Command::Unstop {
                id: Some(id.clone()),
            }),
            false,
        ),
        (change(plan::Command::Cancel { id: id.clone() }), false),
        (change(plan::Command::Leave { id: id.clone() }), false),
        (change(plan::Command::Tick), false),
        (Command::Status, false),
        (Command::Clear, false),
    ];
    for ((name, required, optional, wait), (command, waits)) in DESIGN.into_iter().zip(expected) {
        assert_eq!(wait, waits, "{name}: 期待値の並びが表とずれている");
        let mut opts = [required, optional].concat();
        if wait {
            opts.push(Wait);
        }
        let words = line(name, &opts);
        let got = parse(&words).unwrap_or_else(|err| panic!("{words:?}: {}", err.0));
        assert_eq!(
            got,
            Invocation {
                name,
                command,
                wait
            },
            "{words:?}"
        );
    }
}

#[test]
fn arguments_may_come_in_any_order() {
    let got = parse(&args(&[
        "merge", "--wait", "--spec", "s", "--bug", "--repo", "r", "--id", "a",
    ]))
    .expect("通る");
    assert_eq!(
        got.command,
        Command::Change(plan::Command::Merge {
            id: s("a"),
            name: None,
            repo: s("r"),
            spec: s("s"),
            bug: true,
        })
    );
    assert!(got.wait);
}

#[test]
fn omitted_optional_arguments_are_absent_not_filled_in() {
    // `--name` を省いたら `None`（識別を詰めて渡さない）。
    assert_eq!(
        parse(&line("watch", &[Id, Repo])).expect("通る").command,
        Command::Watch {
            id: s("a1"),
            repo: s("areka"),
            name: None,
        }
    );
    let merge = parse(&line("merge", &[Id, Repo, Spec])).expect("通る");
    assert_eq!(
        merge.command,
        Command::Change(plan::Command::Merge {
            id: s("a1"),
            name: None,
            repo: s("areka"),
            spec: s("areka-P0-impl-watch"),
            bug: false,
        })
    );
    assert!(!merge.wait);
    assert_eq!(
        parse(&args(&["unstop"])).expect("通る").command,
        Command::Change(plan::Command::Unstop { id: None })
    );
}

#[test]
fn dropping_any_required_argument_is_a_usage_error() {
    for (name, required, _, _) in DESIGN {
        parse(&line(name, required)).unwrap_or_else(|err| panic!("{name}: {}", err.0));
        for dropped in required {
            let kept: Vec<Opt> = required.iter().copied().filter(|o| o != dropped).collect();
            let text = refusal(&line(name, &kept));
            let flag = sample(*dropped).0;
            assert_eq!(text, format!("{name}: missing {flag}"));
        }
    }
}

#[test]
fn each_option_is_accepted_only_by_the_commands_that_list_it() {
    // `--wait`・`--bug` もこの総当たりに入る。
    for (name, required, optional, wait) in DESIGN {
        for opt in ALL_OPTIONS {
            if required.contains(&opt) {
                continue;
            }
            let mut opts = required.to_vec();
            opts.push(opt);
            let words = line(name, &opts);
            let listed = optional.contains(&opt) || (opt == Wait && wait);
            match parse(&words) {
                Ok(invocation) => {
                    assert!(listed, "表に無い引数が通った: {words:?}");
                    assert_eq!(invocation.wait, opt == Wait, "{words:?}");
                }
                Err(err) => {
                    assert!(!listed, "表に在る引数が断られた: {words:?}: {}", err.0);
                    let flag = sample(opt).0;
                    assert_eq!(err.0, format!("{name}: {flag} is not accepted"));
                }
            }
        }
    }
}

// ---- 形の決まり ----

#[test]
fn id_repo_and_spec_are_1_to_100_chars_of_the_allowed_set() {
    let longest = "a".repeat(100);
    let too_long = "a".repeat(101);
    for good in ["a", "A-z_0.9", "areka-P0.impl_watch", longest.as_str()] {
        parse(&args(&["leave", "--id", good])).unwrap_or_else(|err| panic!("{good}: {}", err.0));
    }
    for bad in ["", too_long.as_str(), "a b", "a/b", "a\\b", "見張り", "a\n"] {
        assert_eq!(
            refusal(&args(&["leave", "--id", bad])),
            "leave: --id must be 1-100 chars of A-Z a-z 0-9 . _ -",
            "{bad:?}"
        );
    }
    // リポジトリと spec も同じ決まり。
    assert_eq!(
        refusal(&args(&["watch", "--id", "a", "--repo", "a/b"])),
        "watch: --repo must be 1-100 chars of A-Z a-z 0-9 . _ -"
    );
    assert_eq!(
        refusal(&args(&[
            "merge", "--id", "a", "--repo", "r", "--spec", "仕様"
        ])),
        "merge: --spec must be 1-100 chars of A-Z a-z 0-9 . _ -"
    );
}

#[test]
fn pr_is_digits_only_and_sha_is_7_to_40_hex_digits() {
    let merged = |pr: &str, sha: &str| args(&["merged", "--id", "a", "--pr", pr, "--sha", sha]);
    let sha40 = "0123456789abcdefABCDEF0123456789abcdef01";
    assert_eq!(sha40.len(), 40);
    for sha in ["414d43e", "AbCdEf0", sha40] {
        parse(&merged("281", sha)).unwrap_or_else(|err| panic!("{sha}: {}", err.0));
    }
    for pr in ["", "28a", "#281", "-1", "２８１"] {
        assert_eq!(
            refusal(&merged(pr, "414d43eb")),
            "merged: --pr must be digits only",
            "{pr:?}"
        );
    }
    let sha41 = format!("{sha40}0");
    for sha in ["414d43", sha41.as_str(), "414d43g", "414d43e "] {
        assert_eq!(
            refusal(&merged("281", sha)),
            "merged: --sha must be 7-40 hex digits",
            "{sha:?}"
        );
    }
}

#[test]
fn name_and_purpose_take_any_text() {
    let got = parse(&args(&[
        "loadtest",
        "--id",
        "a",
        "--repo",
        "r",
        "--purpose",
        "a b/c 負荷",
        "--name",
        "",
    ]))
    .expect("通る");
    assert_eq!(
        got.command,
        Command::Change(plan::Command::LoadTest {
            id: s("a"),
            name: Some(s("")),
            repo: s("r"),
            purpose: s("a b/c 負荷"),
        })
    );
}

// ---- 使い方の誤り ----

#[test]
fn malformed_command_lines_are_usage_errors() {
    for (words, text) in [
        (args(&[]), "no command given"),
        (args(&["-h"]), "unknown command '-h'"),
        (args(&["join"]), "unknown command 'join'"),
        (args(&["Watch"]), "unknown command 'Watch'"),
        (
            args(&["--help", "merge"]),
            "--help: unexpected argument 'merge'",
        ),
        (
            args(&["tick", "--help"]),
            "tick: unexpected argument '--help'",
        ),
        (
            args(&["tick", "--verbose"]),
            "tick: unexpected argument '--verbose'",
        ),
        (args(&["leave", "a"]), "leave: unexpected argument 'a'"),
        (
            args(&["leave", "--id", "a", "b"]),
            "leave: unexpected argument 'b'",
        ),
        (args(&["leave", "--id"]), "leave: --id needs a value"),
        // 値の場所に次の引数が来ている（空の変数を引用符なしで渡したときの形）。
        (
            args(&["merge", "--id", "a", "--repo", "r", "--spec", "--bug"]),
            "merge: --spec needs a value",
        ),
        (
            args(&[
                "merge", "--id", "a", "--repo", "r", "--spec", "s", "--name", "--wait",
            ]),
            "merge: --name needs a value",
        ),
        (
            args(&["leave", "--id", "a", "--id", "a"]),
            "leave: --id given twice",
        ),
        (
            args(&["stopped", "--id", "a", "--wait", "--wait"]),
            "stopped: --wait given twice",
        ),
    ] {
        assert_eq!(refusal(&words), text, "{words:?}");
    }
}

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

#[test]
fn usage_is_ascii_and_shows_every_command_argument_and_exit_code() {
    let text = usage();
    assert!(text.is_ascii());
    let lines: Vec<&str> = text.lines().collect();

    // コマンドの行は「字下げ 2 つ＋名前＋引数」。要る引数はそのまま、任意は角かっこ。
    for (name, required, optional, wait) in DESIGN {
        let shown: Vec<&&str> = lines
            .iter()
            .filter(|l| l.strip_prefix("  ").and_then(|l| l.split(' ').next()) == Some(name))
            .collect();
        let [shown] = shown.as_slice() else {
            panic!("{name} の行が 1 つでない: {shown:?}");
        };
        let words: Vec<&str> = shown.split_whitespace().collect();
        for opt in ALL_OPTIONS {
            let flag = sample(opt).0;
            let bare = words.contains(&flag);
            let bracketed = words.contains(&format!("[{flag}").as_str())
                || words.contains(&format!("[{flag}]").as_str());
            assert_eq!(bare, required.contains(&opt), "{shown}: {flag}");
            assert_eq!(
                bracketed,
                optional.contains(&opt) || (opt == Wait && wait),
                "{shown}: [{flag}]"
            );
        }
    }
    assert!(
        lines.contains(
            &"  merge --id <id> --repo <repo> --spec <spec> [--bug] [--name <name>] [--wait]"
        ),
        "{text}"
    );
    assert!(lines.contains(&"  unstop [--id <id>]"), "{text}");
    assert!(lines.contains(&"  --help"), "{text}");

    // 引数の形の決まり。
    for (opt, rule) in [
        (Id, "1-100 chars of A-Z a-z 0-9 . _ -"),
        (Repo, "1-100 chars of A-Z a-z 0-9 . _ -"),
        (Spec, "1-100 chars of A-Z a-z 0-9 . _ -"),
        (Name, "any text"),
        (Purpose, "any text"),
        (Pr, "digits only"),
        (Sha, "7-40 hex digits"),
        (Bug, "no value"),
        (Wait, "no value"),
    ] {
        let flag = sample(opt).0;
        let found = lines.iter().any(|l| {
            l.strip_prefix("  ")
                .and_then(|l| l.strip_prefix(flag))
                .is_some_and(|rest| rest.trim_start() == rule)
        });
        assert!(found, "{flag} の形の行が無い:\n{text}");
    }

    // 終了コード 4 つ。
    let at = lines
        .iter()
        .position(|l| *l == "exit codes:")
        .expect("終了コードの見出し");
    for code in ["0", "1", "2", "3"] {
        let found = lines[at + 1..]
            .iter()
            .any(|l| l.strip_prefix("  ").and_then(|l| l.split(' ').next()) == Some(code));
        assert!(found, "終了コード {code} の行が無い:\n{text}");
    }
}

// ---- 終了コード ----

#[test]
fn outcomes_map_to_the_four_exit_codes() {
    assert_eq!(exit_code(&Ok(Outcome::Done)), 0);
    assert_eq!(exit_code(&Err(WatchError::LockBusy)), 1);
    assert_eq!(exit_code(&Ok(Outcome::Usage)), 2);
    assert_eq!(exit_code(&Ok(Outcome::NotApplied)), 3);
}

// ---- つなぐ前の仮の枝 ----

#[test]
fn a_parsed_command_is_refused_until_its_procedure_is_wired() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = root.child("home");
    for words in [args(&["status"]), line("merge", &[Id, Repo, Spec, Wait])] {
        let (result, out, err) = run_captured(&words, Some(home.clone().into_os_string()));
        // 成功を装わない。
        assert_ne!(exit_code(&result), 0, "{words:?}: {result:?}");
        assert_eq!(out, "", "{words:?}");
        assert!(err.is_ascii(), "{err:?}");
        assert_eq!(err.matches('\n').count(), 1, "{err:?}");
        assert!(err.contains(&words[0]), "{err:?}");
        // 置き場所のフォルダは作るが、状態ファイルもログも作らない。
        assert_eq!(entries(&home), Vec::<PathBuf>::new(), "{words:?}");
    }
}
