//! 起動の依頼の純粋な判断のテスト（design「Testing Strategy / 単体」・要件 3.1・3.2・4.3・4.4・5.1〜5.4・9.3・9.5・10.1）。

use std::path::PathBuf;

use areka_kanade::ChangeOrigin;

use super::*;
use crate::emo2_boot::ghost_switch::{GhostSpec, SwitchRequest};
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

/// 枝を 1 つずつ外していく既定の事実（そのままなら `Request`）。
fn facts<'a>(archives: usize, ends: &'a [ArchiveEnd]) -> CallGhostFacts<'a> {
    CallGhostFacts {
        archives,
        ends,
        closing: false,
        switching: false,
        switches_at_submit: 3,
        switches_now: 3,
        ghost_now: Some("emo2"),
    }
}

fn skip(reason: CallGhostSkip) -> CallGhost {
    CallGhost::Skip(reason)
}

fn request(folder: &str) -> CallGhost {
    CallGhost::Request {
        folder: folder.to_owned(),
    }
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

// ---- decide_call_ghost ----

#[test]
fn all_clear_requests_the_installed_ghost_folder() {
    let ends = [ghost("yame")];
    assert_eq!(decide_call_ghost(&facts(1, &ends)), request("yame"));
}

#[test]
fn abandoned_end_skips() {
    let ends = [ghost("a"), ArchiveEnd::Abandoned];
    assert_eq!(
        decide_call_ghost(&facts(3, &ends)),
        skip(CallGhostSkip::Abandoned)
    );
}

#[test]
fn full_ends_with_last_abandoned_skips() {
    // 書庫の数はそろっていても、Abandoned が在れば 1 の枝で止まる。
    let ends = [ghost("a"), ArchiveEnd::Abandoned];
    assert_eq!(
        decide_call_ghost(&facts(2, &ends)),
        skip(CallGhostSkip::Abandoned)
    );
}

#[test]
fn short_ends_without_abandoned_skips_as_abandoned() {
    let ends = [ghost("a")];
    assert_eq!(
        decide_call_ghost(&facts(2, &ends)),
        skip(CallGhostSkip::Abandoned)
    );
}

#[test]
fn closing_skips() {
    let ends = [ghost("a")];
    let f = CallGhostFacts {
        closing: true,
        ..facts(1, &ends)
    };
    assert_eq!(decide_call_ghost(&f), skip(CallGhostSkip::Closing));
}

#[test]
fn no_ghost_installed_skips_for_shell_balloon_supplement_and_all_failed() {
    let cases: Vec<Vec<ArchiveEnd>> = vec![
        vec![installed("shell", r"C:\areka\ghost\emo2\shell\s")],
        vec![installed("balloon", r"C:\areka\balloon\b")],
        vec![installed("supplement", r"C:\areka\ghost\emo2")],
        vec![
            ArchiveEnd::Refused,
            ArchiveEnd::Declined,
            ArchiveEnd::Failed(FailureWord::InvalidType),
        ],
    ];
    for ends in &cases {
        assert_eq!(
            decide_call_ghost(&facts(ends.len(), ends)),
            skip(CallGhostSkip::NoGhostInstalled),
            "{ends:?}"
        );
    }
}

#[test]
fn ghost_with_bundled_balloon_counts_by_its_first_item() {
    // 同梱バルーンは本体の後ろに並ぶ（本体が先）。
    let ends = [ArchiveEnd::Installed(vec![
        item("ghost", r"C:\areka\ghost\yame"),
        item("balloon", r"C:\areka\balloon\yb"),
    ])];
    assert_eq!(decide_call_ghost(&facts(1, &ends)), request("yame"));
}

#[test]
fn switching_skips() {
    let ends = [ghost("a")];
    let f = CallGhostFacts {
        switching: true,
        ..facts(1, &ends)
    };
    assert_eq!(decide_call_ghost(&f), skip(CallGhostSkip::Switching));
}

#[test]
fn accept_count_grew_skips_as_switched_meanwhile() {
    let ends = [ghost("a")];
    // A -> B は 1 増え、A -> B -> A と戻っても 2 増えるので見分けられる。
    for grown in [4, 5] {
        let f = CallGhostFacts {
            switches_now: grown,
            ..facts(1, &ends)
        };
        assert_eq!(
            decide_call_ghost(&f),
            skip(CallGhostSkip::SwitchedMeanwhile)
        );
    }
}

#[test]
fn unchanged_accept_count_goes_on_to_next_branch() {
    let ends = [ghost("emo2")];
    // 数が同じなら 5 を抜け、6（起動中）へ進む。
    assert_eq!(
        decide_call_ghost(&facts(1, &ends)),
        skip(CallGhostSkip::AlreadyRunning)
    );
}

#[test]
fn running_ghost_skips_regardless_of_case() {
    let ends = [ghost("Emo2")];
    assert_eq!(
        decide_call_ghost(&facts(1, &ends)),
        skip(CallGhostSkip::AlreadyRunning)
    );
}

#[test]
fn no_current_ghost_name_requests() {
    let ends = [ghost("emo2")];
    let f = CallGhostFacts {
        ghost_now: None,
        ..facts(1, &ends)
    };
    assert_eq!(decide_call_ghost(&f), request("emo2"));
}

#[test]
fn target_is_last_installed_ghost() {
    let ends = [
        ghost("a"),
        ArchiveEnd::Failed(FailureWord::Extraction),
        ghost("b"),
        installed("shell", r"C:\areka\ghost\b\shell\s2"),
    ];
    assert_eq!(decide_call_ghost(&facts(4, &ends)), request("b"));
}

// ---- callghost_request ----

#[test]
fn request_matches_the_menu_ghost_frame() {
    assert_eq!(
        callghost_request("yame".to_owned()),
        SwitchRequest {
            ghost: GhostSpec::Folder("yame".to_owned()),
            raise_event: true,
            origin: ChangeOrigin::Manual,
            boot_event: None,
        }
    );
}

// ---- 持ち物を作る・据える・出す・出さずに終わる（タスク 3.2） ----

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

fn desk_world() -> bevy_ecs::world::World {
    let mut world = bevy_ecs::world::World::new();
    world.init_resource::<bevy_ecs::schedule::Schedules>();
    crate::install::register(&mut world);
    world
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
    std::mem::forget(startup); // 落としたときの記録はこのテストの外
}

#[test]
fn dropping_unsubmitted_startup_warns_boot_stopped_once() {
    let ((), events) =
        log_capture_kit::capture(|| drop(StartupInstall::new_with(args(&[r"C:\a.nar"]), None)));
    assert_eq!(not_submitted(&events), ["boot_stopped"]);
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

#[test]
fn discard_before_steady_warns_exit_before_steady_and_not_again_on_drop() {
    let mut world = desk_world();
    let ((), events) = log_capture_kit::capture(|| {
        install(
            &mut world,
            StartupInstall::new_with(args(&[r"C:\a.nar"]), None),
        );
        discard_for_exit(&mut world);
        on_steady(&mut world);
        drop(world);
    });
    assert_eq!(
        not_submitted(&events),
        ["exit_before_steady"],
        "二重に出さない"
    );
}

#[test]
fn first_steady_submits_initial_install_then_args_once() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-steady");
    let dir = tmp.child("exe");
    std::fs::create_dir_all(&dir).expect("exe の代わり");
    let initial = dir.join(INITIAL_INSTALL_FILE);
    std::fs::write(&initial, b"nar").expect("書庫の代わり");
    let mut world = desk_world();
    world.insert_resource(crate::emo2_boot::ghost_switch::SwitchAcceptCount(7));
    let ((orders, followup), events) = log_capture_kit::capture(|| {
        install(
            &mut world,
            StartupInstall::new_with(args(&[r"C:\a.nar", r"C:\b.nar"]), Some(dir.clone())),
        );
        on_steady(&mut world);
        on_steady(&mut world);
        let orders = crate::install::queued_orders(&world);
        let followup = world.resource_mut::<StartupInstall>().args_followup.take();
        drop(world); // 取り出した後に落としても warn は出ない
        (orders, followup)
    });
    assert_eq!(
        orders,
        vec![
            InstallOrder {
                archives: vec![initial],
                origin: InstallOrigin::InitialInstall
            },
            InstallOrder {
                archives: paths(&[r"C:\a.nar", r"C:\b.nar"]),
                origin: InstallOrigin::StartupArgs
            },
        ],
        "初期インストールが先・2 回目の定常到達では出さない"
    );
    let followup = followup.expect("受け付けられたので控える");
    assert!(followup.call_ghost && followup.delete_source);
    assert_eq!(followup.switches_at_submit, 7);
    assert!(not_submitted(&events).is_empty());
}

#[test]
fn absent_initial_install_is_debug_only_and_refused_args_keep_no_followup() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-readonly");
    let dir = tmp.child("exe");
    std::fs::create_dir_all(&dir).expect("exe の代わり");
    let mut world = desk_world();
    world.insert_resource(crate::readonly::ReadOnlyRun);
    let ((), events) = log_capture_kit::capture(|| {
        install(
            &mut world,
            StartupInstall::new_with(args(&[r"C:\a.nar"]), Some(dir.clone())),
        );
        on_steady(&mut world);
    });
    assert!(crate::install::queued_orders(&world).is_empty());
    assert!(
        world.resource::<StartupInstall>().args_followup.is_none(),
        "断られたら控えない"
    );
    let absent: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("initial_install_absent"))
        .collect();
    assert_eq!(absent.len(), 1);
    assert_eq!(absent[0].level, tracing::Level::DEBUG);
    assert!(
        !events.iter().any(|e| e.level <= tracing::Level::WARN),
        "warn 以上は無い"
    );
}

// ---- 依頼の終わりを受ける（タスク 3.3） ----

fn startup_events<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// `--install` を据えて定常到達まで進めた World（控えが置かれている）。
fn world_with_args(archives: &[PathBuf], call_ghost: bool, delete_source: bool) -> World {
    let mut world = desk_world();
    install(
        &mut world,
        StartupInstall::new_with(
            InstallArgs {
                archives: archives.to_vec(),
                call_ghost,
                delete_source,
            },
            None,
        ),
    );
    on_steady(&mut world);
    world
}

fn shell_end() -> ArchiveEnd {
    installed("shell", r"C:\areka\ghost\emo2\shell\s")
}

#[test]
fn order_done_deletes_installed_copies_and_goes_on_past_absent_and_folder() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-delete");
    let deleted = tmp.child("a.nar");
    let absent = tmp.child("b.nar");
    let folder = tmp.child("c.nar");
    let refused = tmp.child("d.nar");
    std::fs::write(&deleted, b"nar").expect("写し");
    std::fs::create_dir_all(&folder).expect("フォルダ");
    std::fs::write(&refused, b"nar").expect("写し");
    let archives = vec![deleted.clone(), absent, folder.clone(), refused.clone()];
    let mut world = world_with_args(&archives, false, true);
    let order = InstallOrder {
        archives,
        origin: InstallOrigin::StartupArgs,
    };
    let ends = [shell_end(), shell_end(), shell_end(), ArchiveEnd::Refused];
    let ((), events) = log_capture_kit::capture(|| on_order_done(&mut world, &order, &ends, false));
    assert!(!deleted.exists(), "入った写しは消える");
    assert!(folder.is_dir(), "フォルダは消えない");
    assert!(refused.is_file(), "断られた写しは消えない");
    let done = startup_events(&events, "startup_install_done");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].level, tracing::Level::INFO);
    let ok = startup_events(&events, "startup_source_deleted");
    assert_eq!(ok.len(), 1);
    assert_eq!(ok[0].level, tracing::Level::INFO);
    let failed = startup_events(&events, "startup_source_delete_failed");
    assert_eq!(failed.len(), 2, "既に無い・フォルダの 2 本は warn で次へ");
    assert!(failed.iter().all(|e| e.level == tracing::Level::WARN));
    assert!(
        startup_events(&events, "startup_callghost_skipped").is_empty(),
        "callghost の語が無ければ判断しない"
    );
    assert!(world.resource::<StartupInstall>().args_followup.is_none());
}

#[test]
fn order_done_summary_names_each_end_and_the_unhandled_rest() {
    let mut world = world_with_args(&[], false, false);
    let order = InstallOrder {
        archives: paths(&[
            r"C:\a.nar",
            r"C:\b.nar",
            r"C:\c.nar",
            r"C:\d.nar",
            r"C:\e.nar",
            r"C:\f.nar",
        ]),
        origin: InstallOrigin::InitialInstall,
    };
    let ends = [
        ArchiveEnd::Refused,
        ArchiveEnd::Declined,
        ArchiveEnd::Failed(FailureWord::Extraction),
        shell_end(),
        ArchiveEnd::Abandoned,
    ];
    let ((), events) = log_capture_kit::capture(|| on_order_done(&mut world, &order, &ends, true));
    let done = startup_events(&events, "startup_install_done");
    assert_eq!(done.len(), 1);
    let results = done[0].field("results").expect("results");
    for word in [
        "refused",
        "declined",
        "failed",
        "installed",
        "abandoned",
        "not_started",
    ] {
        assert!(results.contains(word), "{word} in {results}");
    }
    assert_eq!(done[0].field("origin"), Some("InitialInstall"));
}

#[test]
fn order_done_without_deletesource_keeps_copies_and_skips_callghost_with_reason() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-keep");
    let copy = tmp.child("a.nar");
    std::fs::write(&copy, b"nar").expect("写し");
    let mut world = world_with_args(std::slice::from_ref(&copy), true, false);
    let order = InstallOrder {
        archives: vec![copy.clone()],
        origin: InstallOrigin::StartupArgs,
    };
    let ((), events) =
        log_capture_kit::capture(|| on_order_done(&mut world, &order, &[shell_end()], false));
    assert!(copy.is_file(), "deletesource が無ければ消さない");
    let skipped = startup_events(&events, "startup_callghost_skipped");
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].field_str("reason"), Some("no_ghost_installed"));
    assert_eq!(skipped[0].level, tracing::Level::INFO);
}

#[test]
fn order_done_requests_switch_and_logs_the_verdict() {
    let mut world = world_with_args(&paths(&[r"C:\a.nar"]), true, false);
    let order = InstallOrder {
        archives: paths(&[r"C:\a.nar"]),
        origin: InstallOrigin::StartupArgs,
    };
    let ((), events) =
        log_capture_kit::capture(|| on_order_done(&mut world, &order, &[ghost("yame")], false));
    let requested = startup_events(&events, "startup_callghost_requested");
    assert_eq!(requested.len(), 1);
    assert_eq!(requested[0].level, tracing::Level::INFO);
    assert_eq!(requested[0].field("folder"), Some("yame"));
    // 起動の文脈の無い World なので入口は NoContext で退く（記録は入口のまま）。
    assert_eq!(requested[0].field("verdict"), Some("NoContext"));
}

#[test]
fn order_done_of_other_origins_does_nothing_and_keeps_followup() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-other");
    let copy = tmp.child("a.nar");
    std::fs::write(&copy, b"nar").expect("写し");
    let mut world = world_with_args(std::slice::from_ref(&copy), true, true);
    for origin in [
        InstallOrigin::Menu,
        InstallOrigin::Script,
        InstallOrigin::WindowDrop,
    ] {
        let order = InstallOrder {
            archives: vec![copy.clone()],
            origin,
        };
        let ((), events) =
            log_capture_kit::capture(|| on_order_done(&mut world, &order, &[ghost("a")], false));
        assert!(events.is_empty(), "{origin:?}: 記録も出さない");
    }
    assert!(copy.is_file(), "よその出どころの書庫は消えない");
    assert!(
        world.resource::<StartupInstall>().args_followup.is_some(),
        "控えも残る"
    );
}

#[test]
fn initial_install_is_deleted_without_words_and_only_when_installed() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-initial-done");
    let dir = tmp.child("exe");
    std::fs::create_dir_all(&dir).expect("exe の代わり");
    let initial = dir.join(INITIAL_INSTALL_FILE);
    std::fs::write(&initial, b"nar").expect("書庫の代わり");
    let mut world = desk_world();
    install(
        &mut world,
        StartupInstall::new_with(InstallArgs::default(), Some(dir.clone())),
    );
    on_steady(&mut world);
    let order = InstallOrder {
        archives: vec![initial.clone()],
        origin: InstallOrigin::InitialInstall,
    };
    on_order_done(&mut world, &order, &[ArchiveEnd::Declined], false);
    assert!(initial.is_file(), "入らなければ消さない");
    on_order_done(&mut world, &order, &[ghost("a")], false);
    assert!(!initial.exists(), "語なしでも入ったら消す");
}

#[test]
fn closing_keeps_copies_and_warns_once_per_origin() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-closing");
    let copy = tmp.child("a.nar");
    std::fs::write(&copy, b"nar").expect("写し");
    let mut world = world_with_args(std::slice::from_ref(&copy), true, true);
    let args_order = InstallOrder {
        archives: vec![copy.clone()],
        origin: InstallOrigin::StartupArgs,
    };
    let initial_order = InstallOrder {
        archives: vec![copy.clone()],
        origin: InstallOrigin::InitialInstall,
    };
    let ((), events) = log_capture_kit::capture(|| {
        on_order_done(&mut world, &args_order, &[ghost("a")], true);
        // 控えは取り出したので 2 度目は出さない。
        on_order_done(&mut world, &args_order, &[ghost("a")], true);
        // 初期インストールは消す候補が無ければ出さない・在れば出す。
        on_order_done(&mut world, &initial_order, &[ArchiveEnd::Refused], true);
        on_order_done(&mut world, &initial_order, &[ghost("a")], true);
    });
    assert!(copy.is_file(), "終了が始まった後は消さない");
    let dropped = startup_events(&events, "startup_followup_dropped");
    let origins: Vec<_> = dropped
        .iter()
        .map(|e| {
            assert_eq!(e.level, tracing::Level::WARN);
            e.field("origin").unwrap_or_default().to_owned()
        })
        .collect();
    assert_eq!(origins, ["StartupArgs", "InitialInstall"]);
    assert!(startup_events(&events, "startup_callghost_requested").is_empty());
    assert!(startup_events(&events, "startup_callghost_skipped").is_empty());
    assert_eq!(startup_events(&events, "startup_install_done").len(), 4);
}

#[test]
fn discard_then_late_order_done_warns_dropped_exactly_once() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-late");
    let copy = tmp.child("a.nar");
    std::fs::write(&copy, b"nar").expect("写し");
    let mut world = world_with_args(std::slice::from_ref(&copy), true, true);
    let order = InstallOrder {
        archives: vec![copy.clone()],
        origin: InstallOrigin::StartupArgs,
    };
    let ((), events) = log_capture_kit::capture(|| {
        discard_for_exit(&mut world);
        on_order_done(&mut world, &order, &[ghost("a")], true);
    });
    assert!(copy.is_file());
    assert_eq!(
        startup_events(&events, "startup_followup_dropped").len(),
        1,
        "終了の後始末と遅れた終わりで二重に出さない"
    );
    assert!(not_submitted(&events).is_empty());
}

#[test]
fn discard_without_words_drops_followup_silently() {
    let mut world = world_with_args(&paths(&[r"C:\a.nar"]), false, false);
    let ((), events) = log_capture_kit::capture(|| discard_for_exit(&mut world));
    assert!(events.is_empty(), "語が立っていなければ記録しない");
    assert!(world.resource::<StartupInstall>().args_followup.is_none());
}

#[test]
fn closing_without_words_drops_followup_silently() {
    let mut world = world_with_args(&paths(&[r"C:\a.nar"]), false, false);
    let order = InstallOrder {
        archives: paths(&[r"C:\a.nar"]),
        origin: InstallOrigin::StartupArgs,
    };
    let ((), events) =
        log_capture_kit::capture(|| on_order_done(&mut world, &order, &[ghost("a")], true));
    assert!(
        startup_events(&events, "startup_followup_dropped").is_empty(),
        "語が立っていなければ控えを落としても記録しない"
    );
    assert!(world.resource::<StartupInstall>().args_followup.is_none());
}
