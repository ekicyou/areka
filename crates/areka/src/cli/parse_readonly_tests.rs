//! `--option readonly` の語（readonly-mode 要件 1.1〜1.5・10.3）。

use super::*;
use crate::cli::ModulePolicy;

/// `args[0]` に exe を足して読む。
fn run(args: &[&str]) -> Parsed {
    let mut all = vec!["areka".to_owned()];
    all.extend(args.iter().map(|a| (*a).to_owned()));
    parse(&all)
}

fn s(v: &str) -> String {
    v.to_owned()
}

/// 最後のまとめの (効く・何もしない・無視)。
fn summary(parsed: &Parsed) -> (&[String], &[String], &[String]) {
    match parsed.notes.last() {
        Some(ParseNote::Summary {
            effective,
            noop,
            ignored,
        }) => (effective, noop, ignored),
        other => panic!("最後はまとめ: {other:?}"),
    }
}

fn pending_words(parsed: &Parsed) -> Vec<&str> {
    parsed
        .notes
        .iter()
        .filter_map(|n| match n {
            ParseNote::WordPending { word } => Some(word.as_str()),
            _ => None,
        })
        .collect()
}

/// 1.1・1.2: 大小・カンマの並び・`--option` の重ねのどれでも真になり、効く語に 1 つだけ載り、まだ扱わない記録は出ない。
#[test]
fn readonly_word_is_effective_once() {
    let rows: Vec<Vec<&str>> = vec![
        vec!["--option", "readonly"],
        vec!["--option", "READONLY"],
        vec!["--option=ReadOnly"],
        vec!["/o", "nobootcheck,readonly"],
        vec!["--option", "readonly", "--option", "readonly"],
        vec!["--option", "readonly,READONLY"],
    ];
    for args in rows {
        let parsed = run(&args);
        assert!(parsed.options.readonly, "{args:?}");
        assert!(pending_words(&parsed).is_empty(), "{args:?}");
        let (effective, _, ignored) = summary(&parsed);
        assert_eq!(
            effective.iter().filter(|w| *w == "readonly").count(),
            1,
            "{args:?}: {effective:?}"
        );
        assert!(!ignored.contains(&s("readonly")), "{args:?}");
    }
    assert!(!run(&[]).options.readonly);
    assert!(!run(&["--option", "nobootcheck"]).options.readonly);
}

/// 1.4: `kiosk,readonly` では `kiosk` だけがまだ扱わない語。
#[test]
fn kiosk_stays_pending_beside_readonly() {
    let parsed = run(&["--option", "kiosk,readonly"]);
    assert!(parsed.options.readonly);
    assert_eq!(pending_words(&parsed), vec!["kiosk"]);
    let (effective, _, ignored) = summary(&parsed);
    assert_eq!(effective, &[s("readonly")]);
    assert_eq!(ignored, &[s("kiosk")]);
}

/// 1.5: `--offline-dump` と一緒なら `option` が効かなかった記録が出て、`readonly` は無視の側へ回る。
#[test]
fn readonly_with_offline_dump_is_ignored() {
    let parsed = run(&["--offline-dump", "g", "--option", "readonly"]);
    assert!(
        parsed
            .notes
            .contains(&ParseNote::OfflineIgnored { name: s("option") })
    );
    assert!(pending_words(&parsed).is_empty());
    let (effective, _, ignored) = summary(&parsed);
    assert!(!effective.contains(&s("readonly")), "{effective:?}");
    assert!(ignored.contains(&s("readonly")), "{ignored:?}");
}

/// 1.3: `readonly` でも `bootunlock` でも起動の求めは同じ（任せ先を変えない）。
#[test]
fn readonly_boot_request_matches_bootunlock() {
    let with = run(&["--option", "readonly", "--ghost", "a"]);
    let unlock = run(&["--option", "bootunlock", "--ghost", "a"]);
    assert_eq!(with.options.boot, unlock.options.boot);
    assert_eq!(with.options.modules, unlock.options.modules);
}

/// 1.4: `nobootcheck`・`dontloadmodule` と組んでも、それぞれの効き目は変わらない。
#[test]
fn readonly_keeps_other_words_effects() {
    let alone = run(&["--option", "nobootcheck,dontloadmodule"]);
    let both = run(&["--option", "nobootcheck,readonly,dontloadmodule"]);
    assert!(both.options.readonly);
    assert!(both.options.boot.no_boot_check);
    assert_eq!(both.options.modules, ModulePolicy::Skip);
    assert_eq!(both.options.boot, alone.options.boot);
    assert_eq!(
        summary(&both).0,
        &[s("nobootcheck"), s("readonly"), s("dontloadmodule")]
    );
}
