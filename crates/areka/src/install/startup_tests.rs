//! 起動の依頼の持ち物と消す書庫の判断のテスト（design「Testing Strategy / 単体」・要件 3.1・3.2・4.3・4.4・9.6）。

use std::path::PathBuf;

use super::*;
use crate::cli::BootRequest;
use crate::install::judge::{FailureWord, InstalledItem};

fn item(kind: &'static str, place: &str) -> InstalledItem {
    InstalledItem {
        kind,
        name: format!("{kind} name"),
        place: place.to_owned(),
    }
}

fn installed(kind: &'static str, place: &str) -> ArchiveEnd {
    ArchiveEnd::Installed(vec![item(kind, place)])
}

fn ghost(folder: &str) -> ArchiveEnd {
    installed("ghost", &format!(r"C:\areka\ghost\{folder}"))
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

// ---- sources_to_delete ----

#[test]
fn only_installed_of_the_five_ends_is_deleted() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar", r"C:\c.nar", r"C:\d.nar"]);
    let ends = [
        ghost("a"),
        ArchiveEnd::Refused,
        ArchiveEnd::Declined,
        ArchiveEnd::Failed(FailureWord::Extraction),
    ];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

#[test]
fn abandoned_and_unhandled_rest_are_not_deleted() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar", r"C:\c.nar"]);
    let ends = [
        installed("shell", r"C:\areka\ghost\x\shell\s"),
        ArchiveEnd::Abandoned,
    ];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

#[test]
fn nothing_installed_deletes_nothing() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar"]);
    let ends = [ArchiveEnd::Refused, ArchiveEnd::Failed(FailureWord::FileIo)];
    assert!(sources_to_delete(&archives, &ends).is_empty());
}

#[test]
fn same_path_in_other_case_is_deleted_once_with_first_spelling() {
    // 片方だけが入った（1 回目は失敗・2 回目が入った）でも、最初の綴りで 1 回だけ。
    let archives = paths(&[r"C:\Dir\A.nar", r"C:\b.nar", r"c:\dir\a.NAR"]);
    let ends = [
        ArchiveEnd::Failed(FailureWord::Unsupported),
        ghost("b"),
        ghost("a"),
    ];
    assert_eq!(
        sources_to_delete(&archives, &ends),
        paths(&[r"C:\Dir\A.nar", r"C:\b.nar"])
    );
}

#[test]
fn same_path_installed_twice_is_deleted_once() {
    let archives = paths(&[r"C:\a.nar", r"C:\A.NAR"]);
    let ends = [ghost("a"), ghost("a")];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

// ---- 持ち物を作る・出さずに終わる（タスク 3.2） ----

fn args(archives: &[&str]) -> InstallArgs {
    InstallArgs {
        archives: paths(archives),
        call_ghost: true,
        delete_source: true,
    }
}

fn not_submitted(events: &[log_capture_kit::CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("startup_install_not_submitted"))
        .map(|e| {
            assert_eq!(e.level, tracing::Level::WARN);
            e.field_str("reason").unwrap_or_default().to_owned()
        })
        .collect()
}

#[test]
fn new_makes_relative_paths_absolute_from_the_current_folder() {
    let absolute = std::env::current_dir()
        .expect("今のフォルダ")
        .join("abs.nar");
    let (startup, events) = log_capture_kit::capture(|| {
        StartupInstall::new_with(
            InstallArgs {
                archives: vec![PathBuf::from("rel.nar"), absolute.clone(), PathBuf::new()],
                ..Default::default()
            },
            None,
        )
    });
    let pending = startup.pending.as_ref().expect("据えた");
    let cwd = std::env::current_dir().expect("今のフォルダ");
    assert_eq!(
        pending.archives,
        vec![cwd.join("rel.nar"), absolute, PathBuf::new()],
        "相対は今のフォルダから・絶対はそのまま・直せない空は綴りのまま"
    );
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_path_not_absolute"))
        .collect();
    assert_eq!(warns.len(), 1, "直せない 1 本だけ warn");
    assert_eq!(warns[0].level, tracing::Level::WARN);
    assert_eq!(warns[0].field("archive"), Some(""), "直せない綴りを載せる");
    assert!(warns[0].field("error").is_some(), "理由を載せる");
    std::mem::forget(startup); // 落としたときの記録はこのテストの外
}

#[test]
fn dropping_unsubmitted_startup_warns_boot_stopped_once() {
    let ((), events) =
        log_capture_kit::capture(|| drop(StartupInstall::new_with(args(&[r"C:\a.nar"]), None)));
    assert_eq!(not_submitted(&events), ["boot_stopped"]);
    let warn = startup_events(&events, "startup_install_not_submitted");
    assert_eq!(
        warn[0].field("archives"),
        Some(format!("{:?}", paths(&[r"C:\a.nar"])).as_str())
    );
    assert!(warn[0].field("initial_install").is_some());
}

#[test]
fn dropping_with_only_initial_install_warns_and_nothing_to_install_is_silent() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-drop-initial");
    let dir = tmp.child("exe");
    std::fs::create_dir_all(&dir).expect("exe の代わり");
    let ((), silent) = log_capture_kit::capture(|| {
        drop(StartupInstall::new_with(
            InstallArgs::default(),
            Some(dir.clone()),
        ))
    });
    assert!(
        not_submitted(&silent).is_empty(),
        "入れる物が無ければ出さない"
    );
    std::fs::write(dir.join("Initial_Install.NAR"), b"nar").expect("書庫の代わり");
    let ((), events) = log_capture_kit::capture(|| {
        drop(StartupInstall::new_with(
            InstallArgs::default(),
            Some(dir.clone()),
        ))
    });
    assert_eq!(
        not_submitted(&events),
        ["boot_stopped"],
        "大小を問わず exe の隣を見る"
    );
}

fn startup_events<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

// ---- decide_call_ghost（要件 4.3〜4.6・4.9・7.4・9.5） ----

fn facts(ends: &[ArchiveEnd]) -> CallGhostFacts<'_> {
    CallGhostFacts {
        requested: true,
        readonly: false,
        boot_target: None,
        ends,
    }
}

/// 判断を 1 回して、答えと `startup_callghost_*` の記録を返す。
fn decide(facts: &CallGhostFacts<'_>) -> (CallGhost, Vec<log_capture_kit::CapturedEvent>) {
    let (answer, events) = log_capture_kit::capture(|| decide_call_ghost(facts));
    let events = events
        .into_iter()
        .filter(|e| {
            e.field_str("event")
                .is_some_and(|name| name.starts_with("startup_callghost"))
        })
        .collect();
    (answer, events)
}

fn skipped_reason(events: &[log_capture_kit::CapturedEvent]) -> (String, Option<String>) {
    assert_eq!(events.len(), 1, "記録は 1 件");
    let e = &events[0];
    assert_eq!(e.field_str("event"), Some("startup_callghost_skipped"));
    assert_eq!(e.level, tracing::Level::INFO);
    (
        e.field_str("reason").unwrap_or_default().to_owned(),
        e.field_str("target").map(str::to_owned),
    )
}

#[test]
fn not_requested_answers_not_requested_and_records_nothing() {
    let ends = [ghost("tama")];
    let (answer, events) = decide(&CallGhostFacts {
        requested: false,
        readonly: true,
        boot_target: Some(BootTarget::Random),
        ..facts(&ends)
    });
    assert_eq!(answer, CallGhost::NotRequested, "渡されていないが最初");
    assert!(events.is_empty(), "渡されていなければ記録しない");
}

#[test]
fn readonly_skips_before_boot_target() {
    let ends = [ghost("tama")];
    let (answer, events) = decide(&CallGhostFacts {
        readonly: true,
        boot_target: Some(BootTarget::Positional),
        ..facts(&ends)
    });
    assert_eq!(answer, CallGhost::Skip(CallGhostSkip::Readonly));
    assert_eq!(skipped_reason(&events), ("readonly".to_owned(), None));
}

#[test]
fn each_boot_target_skips_with_its_word() {
    let ends = [ghost("tama")];
    for (target, word) in [
        (BootTarget::Positional, "positional"),
        (BootTarget::GhostOption, "ghost_option"),
        (BootTarget::Random, "random"),
    ] {
        let (answer, events) = decide(&CallGhostFacts {
            boot_target: Some(target),
            ..facts(&ends)
        });
        assert_eq!(
            answer,
            CallGhost::Skip(CallGhostSkip::BootTargetGiven(target))
        );
        assert_eq!(
            skipped_reason(&events),
            ("boot_target_given".to_owned(), Some(word.to_owned()))
        );
    }
}

#[test]
fn boot_target_is_read_from_the_request_even_when_ghost_name_misses() {
    let of = |request: BootRequest| BootTarget::of(&request);
    assert_eq!(of(BootRequest::default()), None);
    assert_eq!(
        of(BootRequest {
            positional_ghost: Some(PathBuf::from(r"C:\ghost\tama")),
            ..Default::default()
        }),
        Some(BootTarget::Positional)
    );
    assert_eq!(
        of(BootRequest {
            ghost: Some("置き場に無い名前".to_owned()),
            ..Default::default()
        }),
        Some(BootTarget::GhostOption),
        "--ghost が当たらなくても指定あり"
    );
    assert_eq!(
        of(BootRequest {
            random: true,
            ..Default::default()
        }),
        Some(BootTarget::Random)
    );
}

#[test]
fn no_ghost_installed_skips() {
    for ends in [
        vec![installed("shell", r"C:\areka\ghost\tama\shell\alt")],
        vec![installed("balloon", r"C:\areka\balloon\b")],
        vec![
            ArchiveEnd::Failed(FailureWord::Extraction),
            ArchiveEnd::Refused,
            ArchiveEnd::Declined,
        ],
        vec![],
    ] {
        let (answer, events) = decide(&facts(&ends));
        assert_eq!(answer, CallGhost::Skip(CallGhostSkip::NoGhostInstalled));
        assert_eq!(
            skipped_reason(&events),
            ("no_ghost_installed".to_owned(), None)
        );
    }
}

#[test]
fn last_installed_ghost_is_chosen_and_recorded() {
    let ends = [
        ghost("first"),
        ghost("last"),
        installed("balloon", r"C:\areka\balloon\b"),
        ArchiveEnd::Failed(FailureWord::Extraction),
    ];
    let (answer, events) = decide(&facts(&ends));
    assert_eq!(
        answer,
        CallGhost::Choose {
            folder: "last".to_owned()
        }
    );
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].field_str("event"),
        Some("startup_callghost_chosen")
    );
    assert_eq!(events[0].level, tracing::Level::INFO);
    assert_eq!(events[0].field("folder"), Some("last"));
    assert_eq!(events[0].field("origin"), Some("StartupArgs"));
}

#[test]
fn ghost_with_bundled_balloon_counts_by_its_body() {
    let bundled = ArchiveEnd::Installed(vec![
        item("ghost", r"C:\areka\ghost\tama"),
        item("balloon", r"C:\areka\balloon\tama_b"),
    ]);
    let body_balloon = ArchiveEnd::Installed(vec![
        item("balloon", r"C:\areka\balloon\b"),
        item("ghost", r"C:\areka\ghost\not_body"),
    ]);
    let (answer, _) = decide(&facts(&[bundled, body_balloon]));
    assert_eq!(
        answer,
        CallGhost::Choose {
            folder: "tama".to_owned()
        },
        "本体（先頭）がゴーストの書庫だけを数える"
    );
}

#[test]
fn readonly_and_boot_target_come_before_no_ghost_installed() {
    for ends in [
        vec![],
        vec![installed("shell", r"C:\areka\ghost\tama\shell\alt")],
    ] {
        let (answer, _) = decide(&CallGhostFacts {
            readonly: true,
            ..facts(&ends)
        });
        assert_eq!(
            answer,
            CallGhost::Skip(CallGhostSkip::Readonly),
            "ゴーストが無くても読み取りだけが先"
        );
        let (answer, _) = decide(&CallGhostFacts {
            boot_target: Some(BootTarget::GhostOption),
            ..facts(&ends)
        });
        assert_eq!(
            answer,
            CallGhost::Skip(CallGhostSkip::BootTargetGiven(BootTarget::GhostOption)),
            "ゴーストが無くても指定ありが先"
        );
    }
}

// ---- 起動の前に 1 回だけ入れる（タスク 4.3・要件 1.1〜1.3・1.5・2.6・6.1・6.2・6.4・9.2・9.3） ----

/// `install.txt` の行と足すファイルで書庫を `dir` へ置く。
fn nar(
    dir: &temp_path_kit::TempPath,
    file: &str,
    lines: &[&str],
    extra: &[(&str, &[u8])],
) -> PathBuf {
    let path = dir.child(file);
    let mut builder = sample_ghost_kit::NarBuilder::new()
        .file("install.txt", &sample_ghost_kit::install_txt(lines))
        .done();
    for (name, body) in extra {
        builder = builder.file(*name, body).done();
    }
    builder.write_to(&path).expect("書庫を置ける");
    path
}

/// ゴーストの書庫（`directory` が `folder`・`accept` などは `more` で足す）。
fn ghost_nar(
    dir: &temp_path_kit::TempPath,
    folder: &str,
    more: &[&str],
    extra: &[(&str, &[u8])],
) -> PathBuf {
    let directory = format!("directory,{folder}");
    let mut lines = vec![
        "charset,UTF-8",
        "type,ghost",
        "name,入る",
        directory.as_str(),
    ];
    lines.extend_from_slice(more);
    let mut files: Vec<(&str, &[u8])> = vec![(
        "ghost/master/descript.txt",
        b"charset,UTF-8\r\nname,Newbie\r\n",
    )];
    files.extend_from_slice(extra);
    nar(dir, &format!("{folder}.nar"), &lines, &files)
}

fn roots_of(root: &temp_path_kit::TempPath) -> BasewareRoots {
    BasewareRoots::single(root.path().to_path_buf())
}

/// `startup_install_started`／`done` の (origin, 欄) の並び。どれも info で `stage="before_boot"`。
fn before_boot(
    events: &[log_capture_kit::CapturedEvent],
    name: &str,
    field: &str,
) -> Vec<(String, String)> {
    startup_events(events, name)
        .into_iter()
        .map(|e| {
            assert_eq!(e.level, tracing::Level::INFO, "{name}");
            assert_eq!(e.field_str("stage"), Some("before_boot"), "{name}");
            (
                e.field("origin").unwrap_or_default().to_owned(),
                e.field(field).unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn warn_or_worse(
    events: &[log_capture_kit::CapturedEvent],
) -> Vec<&log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect()
}

/// `initial_install.nar` の依頼が先、`--install` の並びが後で別々の依頼になり、どちらも入る。
/// 2 回目は何もしない。控えは入れた順（要件 1.1・1.3・6.1・6.2・9.2）。
#[test]
fn initial_install_goes_first_then_args_and_the_second_call_does_nothing() {
    let root = temp_path_kit::TempPath::under_target("startup-before-boot-root");
    let exe = temp_path_kit::TempPath::under_target("startup-before-boot-exe");
    let initial = ghost_nar(&exe, "first", &[], &[]);
    std::fs::rename(&initial, exe.child(INITIAL_INSTALL_FILE)).expect("exe の隣へ");
    let args_nar = ghost_nar(&exe, "second", &[], &[]);
    let mut startup = StartupInstall::new_with(
        InstallArgs {
            archives: vec![args_nar],
            ..Default::default()
        },
        Some(exe.path().to_path_buf()),
    );
    let ((), events) = log_capture_kit::capture(|| startup.run_before_boot(&roots_of(&root), true));
    assert_eq!(
        before_boot(&events, "startup_install_started", "count"),
        [
            ("InitialInstall".to_owned(), "1".to_owned()),
            ("StartupArgs".to_owned(), "1".to_owned())
        ]
    );
    let done = before_boot(&events, "startup_install_done", "results");
    assert_eq!(
        done.iter()
            .map(|(origin, _)| origin.as_str())
            .collect::<Vec<_>>(),
        ["InitialInstall", "StartupArgs"]
    );
    assert!(
        done.iter()
            .all(|(_, results)| results.contains("installed")),
        "{done:?}"
    );
    for folder in ["first", "second"] {
        assert!(
            root.path()
                .join("ghost")
                .join(folder)
                .join("ghost/master/descript.txt")
                .is_file()
        );
    }
    let carried: Vec<_> = startup.carried.iter().map(|c| c.folder.as_str()).collect();
    assert_eq!(carried, ["first", "second"]);
    assert!(warn_or_worse(&events).is_empty(), "{events:?}");

    let ((), again) = log_capture_kit::capture(|| startup.run_before_boot(&roots_of(&root), true));
    assert!(again.is_empty(), "2 回目は何もしない: {again:?}");
    assert_eq!(startup.carried.len(), 2);
    let ((), dropped) = log_capture_kit::capture(|| drop(startup));
    assert!(
        not_submitted(&dropped).is_empty(),
        "取り出した後は boot_stopped を出さない"
    );
}

/// 在らない・読めない・`accept` 付き・利用条件付き（抑止で拒否）を混ぜても残りが入り、
/// 書庫ごとの終わり方が記録に並ぶ（要件 2.6・6.4・9.3）。
#[test]
fn missing_unreadable_accept_and_terms_do_not_stop_the_rest() {
    let root = temp_path_kit::TempPath::under_target("startup-before-boot-root");
    let dir = temp_path_kit::TempPath::under_target("startup-before-boot-nar");
    let missing = dir.child("missing.nar");
    let unreadable = dir.child("broken.nar");
    std::fs::write(&unreadable, b"not a zip").expect("壊れた書庫");
    let picky = ghost_nar(&dir, "picky", &["accept,さくら"], &[]);
    let terms = ghost_nar(
        &dir,
        "termsy",
        &[],
        &[("terms.txt", "読んでください".as_bytes())],
    );
    let good = ghost_nar(&dir, "good", &[], &[]);
    let mut startup = StartupInstall::new_with(
        InstallArgs {
            archives: vec![missing, unreadable, picky, terms, good],
            ..Default::default()
        },
        None,
    );
    let ((), events) = log_capture_kit::capture(|| startup.run_before_boot(&roots_of(&root), true));
    assert_eq!(
        before_boot(&events, "startup_install_started", "count"),
        [("StartupArgs".to_owned(), "5".to_owned())]
    );
    let done = before_boot(&events, "startup_install_done", "results");
    assert_eq!(done.len(), 1);
    let words: Vec<_> = ["failed", "failed", "refused", "declined", "installed"]
        .iter()
        .map(|w| format!("\"{w}\""))
        .collect();
    let mut at = 0;
    for word in &words {
        at += done[0].1[at..]
            .find(word.as_str())
            .unwrap_or_else(|| panic!("{word} が順に在る: {}", done[0].1))
            + 1;
    }
    assert!(
        root.path()
            .join("ghost/good/ghost/master/descript.txt")
            .is_file()
    );
    for folder in ["picky", "termsy"] {
        assert!(!root.path().join("ghost").join(folder).exists(), "{folder}");
    }
    let carried: Vec<_> = startup.carried.iter().map(|c| c.folder.as_str()).collect();
    assert_eq!(carried, ["good"]);
}

/// 入れる物が何も無ければ手続きを呼ばず、warn 以上を残さない（要件 1.5）。
#[test]
fn nothing_to_install_calls_nothing_and_warns_nothing() {
    let root = temp_path_kit::TempPath::under_target("startup-before-boot-root");
    let exe = temp_path_kit::TempPath::under_target("startup-before-boot-exe");
    let mut startup =
        StartupInstall::new_with(InstallArgs::default(), Some(exe.path().to_path_buf()));
    let ((), events) = log_capture_kit::capture(|| {
        startup.run_before_boot(&roots_of(&root), true);
        drop(startup);
    });
    assert!(warn_or_worse(&events).is_empty(), "{events:?}");
    assert!(startup_events(&events, "startup_install_started").is_empty());
    assert!(!root.path().join("ghost").exists(), "何も入らない");
}
