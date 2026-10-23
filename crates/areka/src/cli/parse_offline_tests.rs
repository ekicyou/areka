//! オフラインの書き出しの欄（areka-P0-offline-surface-dump 要件 1.8・2.2・10.2・12.1）。

use super::*;
use crate::cli::OfflineDumpRequest;

/// `args[0]` に exe を足して読む。
fn run(args: &[&str]) -> Parsed {
    let mut all = vec!["areka".to_owned()];
    all.extend(args.iter().map(|a| (*a).to_owned()));
    parse(&all)
}

/// まとめの前の材料。
fn notes(parsed: &Parsed) -> &[ParseNote] {
    let (last, rest) = parsed.notes.split_last().expect("まとめが 1 件はある");
    assert!(matches!(last, ParseNote::Summary { .. }), "最後はまとめ");
    rest
}

fn s(v: &str) -> String {
    v.to_owned()
}

fn missing(name: &str) -> ParseNote {
    ParseNote::MissingValue { name: s(name) }
}

fn repeated(name: &str) -> ParseNote {
    ParseNote::Repeated { name: s(name) }
}

fn ghost(dir: &str) -> OfflineDumpRequest {
    OfflineDumpRequest {
        ghost_dir: Some(s(dir)),
        ..Default::default()
    }
}

const NONE: &[ParseNote] = &[];

/// 1.8: `--offline-dump` は値の有無に関わらず欄が在り、値なしは記録しない（error は走りが出す）。
#[test]
fn offline_dump_present_with_or_without_value() {
    let cases: Vec<(Vec<&str>, OfflineDumpRequest)> = vec![
        (vec!["--offline-dump", "G"], ghost("G")),
        (vec!["--offline-dump=G"], ghost("G")),
        (vec!["--OFFLINE-DUMP", "G"], ghost("G")),
        (vec!["--offline-dump"], OfflineDumpRequest::default()),
        (vec!["--offline-dump="], OfflineDumpRequest::default()),
        (vec!["--offline-dump", ""], OfflineDumpRequest::default()),
        // 次が名前なら値にしない
        (
            vec!["--offline-dump", "--dump-scope", "1"],
            OfflineDumpRequest {
                scope: Some(s("1")),
                ..Default::default()
            },
        ),
    ];
    for (args, want) in cases {
        let parsed = run(&args);
        assert_eq!(parsed.options.offline, Some(want), "{args:?}");
        assert_eq!(notes(&parsed), NONE, "{args:?}");
        assert_eq!(parsed.options.boot.positional_ghost, None, "{args:?}");
    }
}

/// `--offline-dump` が無ければ欄は無い。
#[test]
fn no_offline_dump_no_fields() {
    for args in [vec![], vec!["--random"], vec!["--dump-scope", "1"]] {
        assert_eq!(run(&args).options.offline, None, "{args:?}");
    }
}

/// 7 つの値の欄（`--名前 値` と `--名前=値`）。
#[test]
fn seven_dump_values_both_forms() {
    let want = OfflineDumpRequest {
        ghost_dir: Some(s("G")),
        shell: Some(s("sh")),
        scope: Some(s("2")),
        surface_list: Some(s("0,10,100-110")),
        output_dir: Some(s("out")),
        output_prefix: Some(s("x_")),
        error_log: Some(s("log.csv")),
        bind: Some(s("0,cat,part")),
    };
    let names = [
        ("dump-shell", "sh"),
        ("dump-scope", "2"),
        ("dump-surface-list", "0,10,100-110"),
        ("dump-output-dir", "out"),
        ("dump-output-prefix", "x_"),
        ("dump-error-log", "log.csv"),
        ("dump-bind", "0,cat,part"),
    ];
    let mut spaced = vec![s("--offline-dump"), s("G")];
    let mut eq = vec![s("--offline-dump=G")];
    for (n, v) in names {
        spaced.extend([format!("--{n}"), s(v)]);
        eq.push(format!("--{n}={v}"));
    }
    for args in [spaced, eq] {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let parsed = run(&args);
        assert_eq!(parsed.options.offline.as_ref(), Some(&want), "{args:?}");
        assert_eq!(notes(&parsed), NONE, "{args:?}");
    }
}

/// 空の値・値なしは値なしの記録（`--offline-dump` があるときだけ）。
#[test]
fn empty_dump_value_is_missing() {
    for args in [
        vec!["--offline-dump", "G", "--dump-scope"],
        vec!["--offline-dump", "G", "--dump-scope="],
        vec!["--offline-dump", "G", "--dump-scope", ""],
        vec!["--dump-scope", "--offline-dump", "G"],
    ] {
        let parsed = run(&args);
        assert_eq!(parsed.options.offline, Some(ghost("G")), "{args:?}");
        assert_eq!(notes(&parsed), &[missing("dump-scope")], "{args:?}");
    }
    // 7 つ全部が値なしなら 7 件
    let parsed = run(&[
        "--offline-dump",
        "--dump-shell",
        "--dump-scope",
        "--dump-surface-list",
        "--dump-output-dir",
        "--dump-output-prefix",
        "--dump-error-log",
        "--dump-bind",
    ]);
    assert_eq!(parsed.options.offline, Some(OfflineDumpRequest::default()));
    assert_eq!(
        notes(&parsed),
        &[
            missing("dump-shell"),
            missing("dump-scope"),
            missing("dump-surface-list"),
            missing("dump-output-dir"),
            missing("dump-output-prefix"),
            missing("dump-error-log"),
            missing("dump-bind"),
        ]
    );
    // `--offline-dump` が無ければ値なしの記録は出さない
    assert_eq!(notes(&run(&["--dump-scope"])), NONE);
}

/// 重なりは後勝ちと `Repeated`。後が値なしなら値なし。
#[test]
fn repeated_dump_values_last_wins() {
    let parsed = run(&["--offline-dump", "a", "--offline-dump", "b"]);
    assert_eq!(parsed.options.offline, Some(ghost("b")));
    assert_eq!(notes(&parsed), &[repeated("offline-dump")]);

    let parsed = run(&["--offline-dump", "G", "--dump-scope", "1", "--dump-scope=2"]);
    assert_eq!(notes(&parsed), &[repeated("dump-scope")]);
    assert_eq!(parsed.options.offline.unwrap().scope, Some(s("2")));

    let parsed = run(&["--offline-dump", "G", "--dump-scope", "1", "--dump-scope"]);
    assert_eq!(
        notes(&parsed),
        &[repeated("dump-scope"), missing("dump-scope")]
    );
    assert_eq!(parsed.options.offline.unwrap().scope, None);
}

/// `--dump-scope -1`: `-1` は名前として読まれ（知らない名前）、スコープは値なし。
#[test]
fn dump_scope_negative_reads_as_name() {
    let parsed = run(&["--offline-dump", "G", "--dump-scope", "-1"]);
    assert_eq!(parsed.options.offline, Some(ghost("G")));
    assert_eq!(
        notes(&parsed),
        &[ParseNote::Unknown { arg: s("-1") }, missing("dump-scope")]
    );
    // `=` の形なら値になる（解釈は走りが行う）
    let parsed = run(&["--offline-dump", "G", "--dump-scope=-1"]);
    assert_eq!(parsed.options.offline.unwrap().scope, Some(s("-1")));
}

/// 2.2 の前提: 相対パスは生のまま運ぶ（基準のフォルダは走りが決める）。
#[test]
fn relative_paths_carried_raw() {
    let parsed = run(&[
        "--offline-dump",
        r"..\ghost\g",
        "--dump-output-dir",
        r".\out",
        "--dump-error-log",
        "log/x.csv",
    ]);
    let o = parsed.options.offline.unwrap();
    assert_eq!(o.ghost_dir.as_deref(), Some(r"..\ghost\g"));
    assert_eq!(o.output_dir.as_deref(), Some(r".\out"));
    assert_eq!(o.error_log.as_deref(), Some("log/x.csv"));
}

/// 10.2: `--help` と一緒なら両方が立つ（どちらを行うかは main が決める）。
#[test]
fn help_and_offline_both_set() {
    for args in [
        vec!["--offline-dump", "G", "--help"],
        vec!["/?", "--offline-dump", "G"],
    ] {
        let parsed = run(&args);
        assert!(parsed.options.help, "{args:?}");
        assert_eq!(parsed.options.offline, Some(ghost("G")), "{args:?}");
    }
}

/// まだ扱わない 2 つは値を飲み、扱わない名前にはしない。ほかの `dump-` は扱わない名前のまま。
#[test]
fn dump_pending_and_other_dump_names() {
    let parsed = run(&[
        "--offline-dump",
        "G",
        "--dump-surface-option",
        "x",
        "--dump-animation=y",
    ]);
    assert_eq!(parsed.options.offline, Some(ghost("G")));
    assert_eq!(parsed.options.boot.positional_ghost, None);
    assert!(
        !notes(&parsed)
            .iter()
            .any(|n| matches!(n, ParseNote::Unsupported { .. })),
        "{:?}",
        parsed.notes
    );

    let parsed = run(&["--offline-dump", "G", "--dump-surface", "d"]);
    assert_eq!(
        notes(&parsed),
        &[ParseNote::Unsupported {
            name: s("dump-surface")
        }]
    );
}

/// まとめ: 効く名前に `offline-dump` と値の欄、まだ扱わない 2 つは無視の側。
#[test]
fn summary_lists_offline_names() {
    let parsed = run(&[
        "--offline-dump",
        "G",
        "--dump-scope",
        "1",
        "--dump-animation",
    ]);
    let Some(ParseNote::Summary {
        effective, ignored, ..
    }) = parsed.notes.last()
    else {
        panic!("まとめが最後");
    };
    assert_eq!(effective, &vec![s("offline-dump"), s("dump-scope")]);
    assert_eq!(ignored, &vec![s("dump-animation")]);
}
