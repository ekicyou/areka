//! `--install` の値の並びと語 `callghost`・`deletesource`（startup-install-option 要件 1.1・1.3〜1.6・9.1・10.5）。

use super::*;
use crate::cli::InstallArgs;
use std::path::PathBuf;

/// `args[0]` に exe を足して読む。
fn run(args: &[&str]) -> Parsed {
    let mut all = vec!["areka".to_owned()];
    all.extend(args.iter().map(|a| (*a).to_owned()));
    parse(&all)
}

fn s(v: &str) -> String {
    v.to_owned()
}

fn install(archives: &[&str], call_ghost: bool, delete_source: bool) -> InstallArgs {
    InstallArgs {
        archives: archives.iter().map(PathBuf::from).collect(),
        call_ghost,
        delete_source,
    }
}

/// 最後のまとめの (効く・何もしない・無視)。
type Summary<'a> = (&'a [String], &'a [String], &'a [String]);

/// まとめの前の材料と、最後のまとめ。
fn split(parsed: &Parsed) -> (&[ParseNote], Summary<'_>) {
    match parsed.notes.split_last() {
        Some((
            ParseNote::Summary {
                effective,
                noop,
                ignored,
            },
            rest,
        )) => (rest, (effective, noop, ignored)),
        other => panic!("最後はまとめ: {other:?}"),
    }
}

fn without(word: &str) -> ParseNote {
    ParseNote::InstallWordWithoutInstall { word: s(word) }
}

/// 1.1: 値は書いた順・書いた綴りのまま。短い形・旧い形・`=` の形・重ねは続けて足す（`Repeated` は出ない）。
#[test]
fn install_values_in_written_order() {
    let rows: Vec<(Vec<&str>, Vec<&str>)> = vec![
        (vec!["--install", "a.nar", "b.nar"], vec!["a.nar", "b.nar"]),
        (vec!["-i", "a.nar"], vec!["a.nar"]),
        (vec!["/i", "a.nar"], vec!["a.nar"]),
        (vec!["/I", "a.nar"], vec!["a.nar"]),
        (vec!["--INSTALL", "a.nar"], vec!["a.nar"]),
        (vec!["--install=a.nar", "b.nar"], vec!["a.nar", "b.nar"]),
        (
            vec!["--install", "b.nar", "--random", "-i", "a.nar"],
            vec!["b.nar", "a.nar"],
        ),
        // 綴りのまま（相対・大小・空白入り）。空の値は入らない（1.2 の「空の値は値なし」）
        (
            vec!["--install", r"x\B.NAR", "", r"C:\d e\f.nar"],
            vec![r"x\B.NAR", r"C:\d e\f.nar"],
        ),
    ];
    for (args, want) in rows {
        let parsed = run(&args);
        assert_eq!(
            parsed.options.install,
            install(&want, false, false),
            "{args:?}"
        );
        let (notes, (effective, _, ignored)) = split(&parsed);
        assert_eq!(notes, &[], "{args:?}");
        assert_eq!(
            effective.iter().filter(|n| *n == "install").count(),
            1,
            "{args:?}: {effective:?}"
        );
        assert!(!ignored.contains(&s("install")), "{args:?}: {ignored:?}");
        // 書庫は位置の引数にならない
        assert_eq!(parsed.options.boot.positional_ghost, None, "{args:?}");
    }
}

/// 1.3: 値が 0 本の `--install` は注 1 件で、まとめの無視の側へ。重ねても 1 件。
#[test]
fn install_without_values_is_one_note() {
    let rows: Vec<Vec<&str>> = vec![
        vec!["--install"],
        vec!["--install="],
        vec!["-i", "--random"],
        vec!["--install", ""],
        vec!["--install", "/i"],
    ];
    for args in rows {
        let parsed = run(&args);
        assert_eq!(parsed.options.install, InstallArgs::default(), "{args:?}");
        let (notes, (effective, _, ignored)) = split(&parsed);
        assert_eq!(notes, &[ParseNote::InstallEmpty], "{args:?}");
        assert!(
            !effective.contains(&s("install")),
            "{args:?}: {effective:?}"
        );
        assert_eq!(
            ignored.iter().filter(|n| *n == "install").count(),
            1,
            "{args:?}: {ignored:?}"
        );
    }
}

/// 1.4: 2 つの語は大小を問わず、カンマの並び・`--option` の重ねでも効く語になり、まだ扱わない記録は出ない。
/// `kiosk` は今のまま「まだ扱わない」。
#[test]
fn install_words_are_effective_with_install() {
    let rows: Vec<(Vec<&str>, bool, bool)> = vec![
        (vec!["-i", "a", "--option", "callghost"], true, false),
        (vec!["-i", "a", "--option", "DeleteSource"], false, true),
        (vec!["-i", "a", "/o", "CALLGHOST,deletesource"], true, true),
        (
            vec!["--option=deletesource", "-o", "callghost", "-i", "a"],
            true,
            true,
        ),
        (vec!["-i", "a", "-o", "callghost,callghost"], true, false),
    ];
    for (args, call_ghost, delete_source) in rows {
        let parsed = run(&args);
        assert_eq!(
            parsed.options.install,
            install(&["a"], call_ghost, delete_source),
            "{args:?}"
        );
        let (notes, (effective, _, ignored)) = split(&parsed);
        assert_eq!(notes, &[], "{args:?}");
        for (word, on) in [("callghost", call_ghost), ("deletesource", delete_source)] {
            let n = effective.iter().filter(|n| *n == word).count();
            assert_eq!(n, usize::from(on), "{args:?}: {effective:?}");
            assert!(!ignored.contains(&s(word)), "{args:?}: {ignored:?}");
        }
    }
    let parsed = run(&["-i", "a", "-o", "kiosk,callghost"]);
    let (notes, (_, _, ignored)) = split(&parsed);
    assert_eq!(notes, &[ParseNote::WordPending { word: s("kiosk") }]);
    assert_eq!(ignored, &[s("kiosk")]);
}

/// 1.5・10.5: `--install` の値なしの語は語ごとに注 1 件、旗は偽、まとめの無視の側へ。ほかは変えない。
#[test]
fn install_words_without_install_are_noted_and_off() {
    // (引数, まとめの前の材料, 効く, 無視)
    type Row<'a> = (Vec<&'a str>, Vec<ParseNote>, Vec<&'a str>, Vec<&'a str>);
    let rows: Vec<Row> = vec![
        (
            vec!["--option", "callghost"],
            vec![without("callghost")],
            vec![],
            vec!["callghost"],
        ),
        (
            vec!["/o", "DeleteSource,nobootcheck,callghost"],
            vec![without("deletesource"), without("callghost")],
            vec!["nobootcheck"],
            vec!["deletesource", "callghost"],
        ),
        // 値が 0 本の `--install` と組むと両方の注
        (
            vec!["--install", "--option", "callghost"],
            vec![ParseNote::InstallEmpty, without("callghost")],
            vec![],
            vec!["install", "callghost"],
        ),
    ];
    for (args, want_notes, want_effective, want_ignored) in rows {
        let parsed = run(&args);
        assert_eq!(parsed.options.install, InstallArgs::default(), "{args:?}");
        let (notes, (effective, _, ignored)) = split(&parsed);
        assert_eq!(notes, want_notes.as_slice(), "{args:?}");
        assert_eq!(effective, want_effective.as_slice(), "{args:?}");
        assert_eq!(ignored, want_ignored.as_slice(), "{args:?}");
    }
    // ほかは変えない
    let parsed = run(&["--option", "nobootcheck,callghost"]);
    assert!(parsed.options.boot.no_boot_check);
}

/// 1.6: `--offline-dump` と一緒なら今の決まりどおり、`install` と `option` は効かせない記録に載り、
/// 名前と語はまとめの無視の側へ。値なしや語だけでも `--install` 向けの注は出ない。
#[test]
fn install_with_offline_dump_is_not_effective() {
    let rows: Vec<(Vec<&str>, Vec<&str>, Vec<&str>)> = vec![
        (
            vec![
                "--offline-dump",
                "g",
                "--install",
                "a.nar",
                "--option",
                "callghost,deletesource",
            ],
            vec!["install", "option"],
            vec!["install", "callghost", "deletesource"],
        ),
        (
            vec!["-i", "--offline-dump", "g"],
            vec!["install"],
            vec!["install"],
        ),
        (
            vec!["--offline-dump", "g", "-o", "callghost"],
            vec!["option"],
            vec!["callghost"],
        ),
    ];
    for (args, want_offline, want_ignored) in rows {
        let parsed = run(&args);
        let (notes, (effective, _, ignored)) = split(&parsed);
        let offline: Vec<&str> = notes
            .iter()
            .filter_map(|n| match n {
                ParseNote::OfflineIgnored { name } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(offline, want_offline, "{args:?}");
        assert!(
            !notes.iter().any(|n| matches!(
                n,
                ParseNote::InstallEmpty | ParseNote::InstallWordWithoutInstall { .. }
            )),
            "{args:?}: {notes:?}"
        );
        assert_eq!(effective, &[s("offline-dump")], "{args:?}");
        assert_eq!(ignored, want_ignored.as_slice(), "{args:?}");
    }
}

/// 記録の形: 設計の記録の表の `event`・warn・前置き `[cli]`。
#[test]
fn install_notes_log_one_warn_each() {
    use tracing::Level;
    let rows = [
        (ParseNote::InstallEmpty, "install_option_empty"),
        (without("callghost"), "install_word_without_install"),
    ];
    for (note, event) in rows {
        let ((), events) = log_capture_kit::capture(|| note.log());
        assert_eq!(events.len(), 1, "{note:?}: {events:?}");
        assert_eq!(events[0].level, Level::WARN, "{note:?}");
        assert_eq!(events[0].field_str("event"), Some(event), "{note:?}");
        assert!(events[0].message().starts_with("[cli] "), "{note:?}");
    }
    let ((), events) = log_capture_kit::capture(|| without("deletesource").log());
    // `%word` は Display で載るので Debug 表現（`field`）で見る。
    assert_eq!(events[0].field("word"), Some("deletesource"));
}
