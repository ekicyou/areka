//! 引数の表・使い方の決定論テスト（`cli_tests.rs` から移した、引数の読みと `--help` の本文）。
//!
//! [`DESIGN`] は設計の「コマンドの表」の写しで、本番の表（`COMMANDS`）とは別に持つ
//! （本番の表を書き換えると、このファイルのテストが赤になる）。

use super::super::subject;
use super::{COMMANDS, Command, Invocation, Opt, parse, usage};
use crate::plan;
use crate::state::DeskSet;

use Opt::{Bug, Desk, Id, Name, Pr, Purpose, Repo, Sha, Spec, Wait};

/// 設計のコマンドの表: 名前・要る引数・任意の引数・`--wait` を取るか（14 行・同じ並び）。
const DESIGN: [(&str, &[Opt], &[Opt], bool); 14] = [
    ("watch", &[Id, Repo], &[Name], false),
    ("merge", &[Id, Repo, Spec], &[Bug, Name], true),
    ("merged", &[Id, Pr, Sha], &[], false),
    ("request", &[Id, Repo, Desk, Purpose], &[Name], true),
    ("return", &[Id], &[Desk], false),
    ("loadrunning", &[Id, Repo, Purpose], &[Name], false),
    ("stopped", &[Id], &[], true),
    ("resume", &[Id], &[], false),
    ("unstop", &[], &[Id], false),
    ("cancel", &[Id], &[], false),
    ("leave", &[Id], &[], false),
    ("tick", &[], &[], false),
    ("status", &[], &[], false),
    ("clear", &[], &[], false),
];

const ALL_OPTIONS: [Opt; 10] = [Id, Repo, Spec, Name, Purpose, Desk, Pr, Sha, Bug, Wait];

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// 引数の綴りと、形に合う値（値を取らないものは `None`）。入口のテストの支え
/// （`cli_test_support.rs`）とは別に持つ（`--desk` を含む）。
fn sample(opt: Opt) -> (&'static str, Option<&'static str>) {
    match opt {
        Id => ("--id", Some("a1")),
        Repo => ("--repo", Some("areka")),
        Spec => ("--spec", Some("areka-P0-impl-watch")),
        Name => ("--name", Some("見張り役")),
        Purpose => ("--purpose", Some("負荷の計測")),
        Desk => ("--desk", Some("test,sole")),
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
            change(plan::Command::Request {
                id: id.clone(),
                name: name.clone(),
                repo: repo.clone(),
                desks: DeskSet::TestSole,
                purpose: purpose.clone(),
            }),
            true,
        ),
        (
            change(plan::Command::Return {
                id: id.clone(),
                desks: Some(DeskSet::TestSole),
            }),
            false,
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
            // `clear` は名前の行に出さない別の形 `--repo` を受ける（要件 11.4）。
            let variant = name == "clear" && opt == Repo;
            let listed = optional.contains(&opt) || (opt == Wait && wait) || variant;
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
        "request",
        "--id",
        "a",
        "--repo",
        "r",
        "--desk",
        "test",
        "--purpose",
        "a b/c 負荷",
        "--name",
        "",
    ]))
    .expect("通る");
    assert_eq!(
        got.command,
        Command::Change(plan::Command::Request {
            id: s("a"),
            name: Some(s("")),
            repo: s("r"),
            desks: DeskSet::Test,
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
    let request = [
        "request",
        "--id",
        "A",
        "--repo",
        "Areka",
        "--desk",
        "sole",
        "--purpose",
        "PurPose",
    ];
    assert_eq!(
        parse(&args(&request)).expect("通る").command,
        Command::Change(plan::Command::Request {
            id: s("a"),
            name: None,
            repo: s("Areka"),
            desks: DeskSet::Sole,
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
        // 古い負荷テストの呼び方は表から消えた（読み替えは手順書）。
        (args(&["loadtest"]), "unknown command 'loadtest'"),
        (args(&["loaddone"]), "unknown command 'loaddone'"),
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
                "loadrunning",
                "--id",
                "a",
                "--repo",
                "r",
                "--purpose",
                "--release x5",
            ]),
            "loadrunning: --purpose needs a value (a value cannot start with --)",
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

// ---- 使い方 ----

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
        (
            Desk,
            "test, sole or test,sole (the merge desk is requested with merge)",
        ),
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
        "request",
        "return",
        "loadrunning",
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
            "report stopped; the test desk held is freed. 0=recorded 3=no stop request; with --wait 0=resumed, or stop requested again 3=record gone",
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

// ---- 机の申し込みと返し ----

fn request_with(desk: &str) -> Vec<String> {
    args(&[
        "request",
        "--id",
        "a",
        "--repo",
        "r",
        "--desk",
        desk,
        "--purpose",
        "p",
    ])
}

fn return_with(desk: &str) -> Vec<String> {
    args(&["return", "--id", "a", "--desk", desk])
}

/// 読んだ申し込み・返しが並べた机。
fn desks_of(words: &[String]) -> Option<DeskSet> {
    let got = parse(words).unwrap_or_else(|err| panic!("{words:?}: {}", err.0));
    match got.command {
        Command::Change(plan::Command::Request { desks, .. }) => Some(desks),
        Command::Change(plan::Command::Return { desks, .. }) => desks,
        other => panic!("{words:?}: {other:?}"),
    }
}

#[test]
fn desk_takes_test_and_sole_in_either_order() {
    for (value, desks) in [
        ("test", DeskSet::Test),
        ("sole", DeskSet::Sole),
        ("test,sole", DeskSet::TestSole),
        ("sole,test", DeskSet::TestSole),
    ] {
        assert_eq!(desks_of(&request_with(value)), Some(desks), "{value}");
        assert_eq!(desks_of(&return_with(value)), Some(desks), "{value}");
    }
    // 名指しの無い返しはマージの机のほかの全部。
    assert_eq!(desks_of(&args(&["return", "--id", "a"])), None);
}

/// 空・同じ名前を 2 度・知らない名前・マージの机は使い方の誤り（要件 2.5）。断りは読みの中で
/// 決まり（読みは何も読み書きしない）、入口は使い方の誤りを終了コード 2 で返す。
#[test]
fn desk_refuses_empty_twice_unknown_and_merge() {
    const WHY: &str =
        "--desk must be test, sole or test,sole (the merge desk is requested with merge)";
    for value in [
        "",
        "test,test",
        "sole,sole",
        "test,sole,test",
        "merge",
        "test,merge",
        "Test",
        "both",
        "test,",
        ",sole",
        ",",
        "test sole",
    ] {
        assert_eq!(
            refusal(&request_with(value)),
            format!("request: {WHY}"),
            "{value:?}"
        );
        assert_eq!(
            refusal(&return_with(value)),
            format!("return: {WHY}"),
            "{value:?}"
        );
    }
    assert_eq!(
        refusal(&args(&[
            "request",
            "--id",
            "a",
            "--repo",
            "r",
            "--desk",
            "test",
            "--desk",
            "sole",
            "--purpose",
            "p",
        ])),
        "request: --desk given twice"
    );
    assert_eq!(
        refusal(&args(&[
            "request",
            "--id",
            "a",
            "--repo",
            "r",
            "--purpose",
            "p"
        ])),
        "request: missing --desk"
    );
}

#[test]
fn usage_shows_the_request_return_and_loadrunning_lines_of_the_design() {
    let text = usage();
    let lines: Vec<&str> = text.lines().collect();
    for (command, note) in [
        (
            "  request --id <id> --repo <repo> --desk <desk> --purpose <purpose> [--name <name>] [--wait]",
            "queue one request for the listed desks; all are granted at once. 0=queued or granted 3=not applied; with --wait 0=granted 3=request gone or not applied",
        ),
        (
            "  return --id <id> [--desk <desk>]",
            "return the named desks, or every desk but the merge desk. 0=returned 3=not the holder",
        ),
        (
            "  loadrunning --id <id> --repo <repo> --purpose <purpose> [--name <name>]",
            "record an already running load test as the holder of the test and sole desks. 0=recorded 3=either desk has a holder or a merge request is waiting",
        ),
        (
            "  cancel --id <id>",
            "drop the requests and desks of the id; the participant stays. 0=done",
        ),
    ] {
        let at = lines.iter().position(|l| *l == command);
        let at = at.unwrap_or_else(|| panic!("{command} の行が無い:\n{text}"));
        assert_eq!(lines[at + 1].trim_start(), note);
    }
    assert!(!text.contains("loadtest"), "{text}");
    assert!(!text.contains("loaddone"), "{text}");
}

// ---- clear --repo ----

/// `--help` に足すのは `clear` の説明の行の直後の 1 行だけ。名前の行は `  clear` のまま。
#[test]
fn usage_adds_exactly_one_clear_repo_line_after_the_clear_note() {
    const ADDED: &str = "      clear --repo <repo>: leave everyone of the repo and empty its merge desk (the last merge stays). prints \"cleared repo=<repo>; left: <ids or none>\". 0=done";
    let lines: Vec<&str> = usage().lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "  clear")
        .expect("clear の行");
    assert_eq!(
        lines[at..at + 4],
        [
            "  clear",
            "      set the state file aside and start empty, without asking. 0=done",
            ADDED,
            "  --help",
        ]
    );
    // 別の形の行はこの 1 行だけ（説明の行の字下げ 6 つの後にコマンドの名前と引数が来るのは
    // ここだけ。`return` の説明は動詞の return で始まるので、名前の次の `--` まで見る）。
    let variants = lines
        .iter()
        .filter(|l| {
            l.strip_prefix("      ")
                .and_then(|l| l.split_once(' '))
                .is_some_and(|(word, rest)| {
                    rest.starts_with("--") && COMMANDS.iter().any(|spec| spec.name == word)
                })
        })
        .count();
    assert_eq!(variants, 1, "{}", usage());
}

#[test]
fn clear_takes_repo_as_its_other_form_and_stays_plain_without_it() {
    assert_eq!(
        parse(&args(&["clear", "--repo", "areka"])).expect("通る"),
        Invocation {
            name: "clear",
            command: Command::Change(plan::Command::ClearRepo { repo: s("areka") }),
            wait: false,
        }
    );
    assert_eq!(
        parse(&args(&["clear"])).expect("通る").command,
        Command::Clear
    );
    let rule = "1-100 chars of A-Z a-z 0-9 . _ -";
    for (words, why) in [
        (
            &["clear", "--repo"][..],
            s("--repo needs a value (a value cannot start with --)"),
        ),
        (
            &["clear", "--repo", "a/b"],
            format!("--repo must be {rule}"),
        ),
        (
            &["clear", "--repo", r"C:\x"],
            format!("--repo must be {rule}"),
        ),
        (
            &["clear", "--repo", "a", "--repo", "a"],
            s("--repo given twice"),
        ),
        (
            &["clear", "--repo", "a", "--id", "a"],
            s("--id is not accepted"),
        ),
        (&["clear", "--repo", "a", "x"], s("unexpected argument 'x'")),
        (
            &["clear", "--repo", "a", "--wait"],
            s("--wait is not accepted"),
        ),
    ] {
        assert_eq!(refusal(&args(words)), format!("clear: {why}"), "{words:?}");
    }
}
