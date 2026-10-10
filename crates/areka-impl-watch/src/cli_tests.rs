//! 引数の表・使い方・誤りの扱いの決定論テスト。
//!
//! 環境変数の値は引数で渡し（プロセスの環境変数は触らない）、標準出力と標準エラーは
//! 差し替えた書き手で受ける。置き場所の向け先はワークツリーの `target\` の下の一時フォルダだけ。
//!
//! [`DESIGN`] は設計の「コマンドの表」の写しで、本番の表（`COMMANDS`）とは別に持つ
//! （本番の表を書き換えると、このファイルのテストが赤になる）。

use std::ffi::OsString;
#[cfg(windows)]
use std::path::Component;
use std::path::PathBuf;

use temp_path_kit::TempPath;

use super::test_support::{args, entries, run_captured};
use super::{COMMANDS, Command, Invocation, Opt, Outcome, exit_code, parse, subject, usage};
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

// ---- 識別の小文字への寄せ ----

/// 読んだコマンドが名指す識別。
fn id_of(command: &Command) -> Option<&str> {
    match command {
        Command::Watch { id, .. } | Command::Resume { id } => Some(id),
        Command::Change(change) => subject(change),
        Command::Help | Command::Status | Command::Clear => None,
    }
}

#[test]
fn the_id_is_folded_to_ascii_lower_case_in_every_command_that_takes_it() {
    let mut taking = 0;
    for (name, required, optional, _) in DESIGN {
        if !required.contains(&Id) && !optional.contains(&Id) {
            continue;
        }
        taking += 1;
        let mut opts = required.to_vec();
        if !opts.contains(&Id) {
            opts.push(Id);
        }
        let mut words = line(name, &opts);
        let at = words.iter().position(|word| word == "--id");
        words[at.expect("--id が在る") + 1] = s("AbC-P0.x_Z9");

        let got = parse(&words).unwrap_or_else(|err| panic!("{words:?}: {}", err.0));

        assert_eq!(id_of(&got.command), Some("abc-p0.x_z9"), "{words:?}");
    }
    assert_eq!(taking, 11, "識別を取るコマンドの数");
}

#[test]
fn only_the_id_is_folded_and_the_other_values_stay_as_typed() {
    let merge = [
        "merge", "--id", "AbC", "--repo", "Areka", "--spec", "Spec-X", "--name", "NaMe",
    ];
    assert_eq!(
        parse(&args(&merge)).expect("通る").command,
        Command::Change(plan::Command::Merge {
            id: s("abc"),
            name: Some(s("NaMe")),
            repo: s("Areka"),
            spec: s("Spec-X"),
            bug: false,
        })
    );
    let loadtest = [
        "loadtest",
        "--id",
        "A",
        "--repo",
        "Areka",
        "--purpose",
        "PurPose",
    ];
    assert_eq!(
        parse(&args(&loadtest)).expect("通る").command,
        Command::Change(plan::Command::LoadTest {
            id: s("a"),
            name: None,
            repo: s("Areka"),
            purpose: s("PurPose"),
        })
    );
    let merged = ["merged", "--id", "A", "--pr", "281", "--sha", "AbCdEf0"];
    assert_eq!(
        parse(&args(&merged)).expect("通る").command,
        Command::Change(plan::Command::Merged {
            id: s("a"),
            pr: s("281"),
            sha: s("AbCdEf0"),
        })
    );
    // 名指しの取り消しも、小文字の識別を指す。
    assert_eq!(
        parse(&args(&["unstop", "--id", "A"]))
            .expect("通る")
            .command,
        Command::Change(plan::Command::Unstop { id: Some(s("a")) })
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
        (
            args(&["leave", "--id"]),
            "leave: --id needs a value (a value cannot start with --)",
        ),
        // 値の場所に次の引数が来ている（空の変数を引用符なしで渡したときの形）。
        (
            args(&["merge", "--id", "a", "--repo", "r", "--spec", "--bug"]),
            "merge: --spec needs a value (a value cannot start with --)",
        ),
        (
            args(&[
                "merge", "--id", "a", "--repo", "r", "--spec", "s", "--name", "--wait",
            ]),
            "merge: --name needs a value (a value cannot start with --)",
        ),
        // 名前・内容も `--` で始まる値は取らない（使い方の `any text not starting with --`）。
        (
            args(&[
                "loadtest",
                "--id",
                "a",
                "--repo",
                "r",
                "--purpose",
                "--release x5",
            ]),
            "loadtest: --purpose needs a value (a value cannot start with --)",
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
        (Name, "any text not starting with --"),
        (Purpose, "any text not starting with --"),
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

#[test]
fn usage_notes_say_what_3_means_for_exactly_the_commands_that_can_end_with_3() {
    // 設計のコマンドの表で、終わり方に 3 を持つ 9 個。
    let with_3 = [
        "watch",
        "merge",
        "merged",
        "loadtest",
        "loadrunning",
        "loaddone",
        "stopped",
        "resume",
        "unstop",
    ];
    let lines: Vec<&str> = usage().lines().collect();
    for (name, ..) in DESIGN {
        let at = lines
            .iter()
            .position(|l| l.strip_prefix("  ").and_then(|l| l.split(' ').next()) == Some(name))
            .unwrap_or_else(|| panic!("{name} の行が無い"));
        let note = lines[at + 1];
        assert!(note.contains("0="), "{name}: {note}");
        assert_eq!(
            note.contains("3="),
            with_3.contains(&name),
            "{name}: {note}"
        );
    }
}

/// 再開の待ちの 0 は 2 通り（再開した・停止要請が出し直された）。
#[test]
fn usage_notes_of_the_resume_waits_say_that_0_is_resumed_or_a_stop_requested_again() {
    let lines: Vec<&str> = usage().lines().collect();
    for (command, note) in [
        (
            "  stopped --id <id> [--wait]",
            "report stopped. 0=recorded 3=no stop request; with --wait 0=resumed, or stop requested again 3=record gone",
        ),
        (
            "  resume --id <id>",
            "wait for the resume only. 0=resumed, or stop requested again 3=record gone",
        ),
    ] {
        let at = lines.iter().position(|l| *l == command);
        let at = at.unwrap_or_else(|| panic!("{command} の行が無い"));
        assert_eq!(lines[at + 1].trim_start(), note);
    }
}

/// 使い方の末尾: 0 に停止要請の出し直しが入り、1 に置き場所の失敗 3 つ（無い・絶対パスで
/// ない・作れない）が並び、識別が小文字に寄ることを言う。
#[test]
fn usage_foot_names_the_reissued_stop_request_the_home_failures_and_the_id_folding() {
    let lines: Vec<&str> = usage().lines().collect();
    for line in [
        "  0  done: granted, stop requested (or requested again), resumed, or the state was changed",
        "  1  failure: AREKA_IMPL_WATCH_HOME unset, not absolute or not creatable, state file, lock, io, same wait already running",
        "--id is folded to lower case: A and a are the same participant.",
    ] {
        assert!(lines.contains(&line), "{line}:\n{}", usage());
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
