//! 根を解いてから入れ、入れた後の置き場で起動の解決を続ける口（install-before-boot タスク 4.5・
//! 要件 1.1・1.7・4.2・4.3・4.9・4.11・6.3・9.1・9.5・9.7）。
//!
//! 中身の口 `resolve_boot_with_env` に根の結果・旗・選び方を差し込む（env は触らない）。
//! 根と書庫は `TempPath::under_target`（ワークツリーの `target\` の下）。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::*;
use crate::alert::AlertScene;
use crate::boot_config::BootResolved;
use crate::boot_resolve::{DEFAULT_BALLOON_FOLDER, GhostRoute};

/// 告知を出さない・読み取りだけでない走り。
const QUIET: BeforeBootEnv = BeforeBootEnv {
    readonly: false,
    alert_suppressed: true,
};

fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

/// `install.txt` の行と足すファイルで書庫を `dir` へ置く。
fn nar(dir: &TempPath, file: &str, lines: &[&str], files: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.child(file);
    let mut builder = sample_ghost_kit::NarBuilder::new()
        .file("install.txt", &sample_ghost_kit::install_txt(lines))
        .done();
    for (name, body) in files {
        builder = builder.file(*name, body).done();
    }
    builder.write_to(&path).expect("書庫を置ける");
    path
}

fn ghost_nar(dir: &TempPath, folder: &str) -> PathBuf {
    let directory = format!("directory,{folder}");
    nar(
        dir,
        &format!("{folder}.nar"),
        &["charset,UTF-8", "type,ghost", "name,入る", &directory],
        &[(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\nname,Newbie\r\n",
        )],
    )
}

fn balloon_nar(dir: &TempPath) -> PathBuf {
    nar(
        dir,
        "round.nar",
        &[
            "charset,UTF-8",
            "type,balloon",
            "name,まるい",
            "directory,round",
        ],
        &[("descript.txt", b"charset,UTF-8\r\nname,Round\r\n")],
    )
}

/// 既定のバルーンだけを置いた根（ゴーストは `ghosts`・記憶は `remember`）。
fn root_with(label: &str, ghosts: &[&str], remember: Option<&str>) -> TempPath {
    use areka_sylphya::persist::FsPersistIo;
    use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, save_scope};
    let tmp = TempPath::under_target(label);
    let balloon = tmp.child("balloon").join(DEFAULT_BALLOON_FOLDER);
    std::fs::create_dir_all(&balloon).expect("フォルダを組む");
    std::fs::write(balloon.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    for g in ghosts {
        let master = tmp.child("ghost").join(g).join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    }
    if let Some(last) = remember {
        save_scope(
            PersistScope::App,
            &ScopeRoots {
                app: Some(tmp.child("app-profile")),
                ..ScopeRoots::default()
            },
            &FsPersistIo,
            vec![(PersistKey::LastGhost, last.to_owned())],
        );
    }
    tmp
}

/// 根 `root` の上で起動の前の道 → 起動の解決を 1 回走らせ、答えと記録を返す。
fn boot(
    resolved: Result<crate::roots::ResolvedRoots, crate::roots::RootError>,
    profile: &Path,
    startup: &mut StartupInstall,
) -> (
    Result<BootResolved, AlertScene>,
    Vec<log_capture_kit::CapturedEvent>,
) {
    log_capture_kit::capture(|| {
        resolve_boot_with_env(
            resolved,
            &BootRequest::default(),
            startup,
            QUIET,
            profile,
            no_pick,
            |_, dir| dir.to_path_buf(),
        )
    })
}

fn args(archives: Vec<PathBuf>, call_ghost: bool) -> InstallArgs {
    InstallArgs {
        archives,
        call_ghost,
        delete_source: false,
    }
}

fn named<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn route(got: Result<BootResolved, AlertScene>) -> (GhostRoute, PathBuf) {
    let got = got.unwrap_or_else(|_| panic!("決まる"));
    (got.1.route, got.1.dir)
}

/// 空の根（バルーンだけ）に起動の前に入れたゴーストが、唯一のゴーストとして選ばれる（要件 1.1・4.2・9.1）。
#[test]
fn ghost_installed_into_an_empty_root_is_chosen_as_the_only_one() {
    let root = root_with("startup-flow-empty", &[], None);
    let dir = TempPath::under_target("startup-flow-nar");
    let mut startup = StartupInstall::new_with(args(vec![ghost_nar(&dir, "newbie")], false), None);
    let (got, events) = boot(
        crate::roots::single_env(root.path().to_path_buf()),
        &root.child("app-profile"),
        &mut startup,
    );
    assert_eq!(
        route(got),
        (GhostRoute::Only, root.child("ghost").join("newbie"))
    );
    let started = events
        .iter()
        .position(|e| e.field_str("event") == Some("startup_install_started"));
    let resolved = events
        .iter()
        .position(|e| e.field_str("event") == Some("ghost_resolved"));
    assert!(started < resolved && started.is_some(), "入れてから選ぶ");
}

/// callghost で、記憶の前回のゴーストより先に入れたゴーストが選ばれる（要件 4.3・9.5）。
#[test]
fn callghost_wins_over_the_remembered_ghost() {
    let root = root_with("startup-flow-callghost", &["g1"], Some("g1"));
    let dir = TempPath::under_target("startup-flow-nar");
    let mut startup = StartupInstall::new_with(args(vec![ghost_nar(&dir, "g2")], true), None);
    let (got, events) = boot(
        crate::roots::single_env(root.path().to_path_buf()),
        &root.child("app-profile"),
        &mut startup,
    );
    assert_eq!(
        route(got),
        (GhostRoute::CallGhost, root.child("ghost").join("g2"))
    );
    assert_eq!(named(&events, "startup_callghost_chosen").len(), 1);
}

/// ゴーストが `initial_install.nar` だけから入ったら `no_ghost_installed` で今の選び方（記憶）になる
/// （要件 4.9・9.5）。
#[test]
fn ghost_only_from_initial_install_falls_back_to_the_usual_choice() {
    let root = root_with("startup-flow-initial", &["g1"], Some("g1"));
    let exe = TempPath::under_target("startup-flow-exe");
    std::fs::rename(ghost_nar(&exe, "first"), exe.child(INITIAL_INSTALL_FILE)).expect("exe の隣へ");
    let mut startup = StartupInstall::new_with(
        args(vec![balloon_nar(&exe)], true),
        Some(exe.path().to_path_buf()),
    );
    let (got, events) = boot(
        crate::roots::single_env(root.path().to_path_buf()),
        &root.child("app-profile"),
        &mut startup,
    );
    assert!(root.child("ghost").join("first").is_dir(), "initial は入る");
    assert_eq!(
        route(got),
        (GhostRoute::Memory, root.child("ghost").join("g1"))
    );
    let skipped = named(&events, "startup_callghost_skipped");
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].field_str("reason"), Some("no_ghost_installed"));
}

/// 根が決まらなければ warn `root_missing` を 1 件残して何も入れず、根なしの場面で止まる。
/// 取り出した後なので落としても `boot_stopped` は出ない（要件 1.7・6.3・9.7）。
#[test]
fn root_missing_warns_once_and_installs_nothing() {
    let profile = TempPath::under_target("startup-flow-noroot");
    let dir = TempPath::under_target("startup-flow-nar");
    let archive = ghost_nar(&dir, "lost");
    let gone = profile.child("gone");
    let mut startup = StartupInstall::new_with(args(vec![archive.clone()], true), None);
    let (got, events) = boot(
        Err(crate::roots::RootError::NoneFound {
            tried: vec![gone.clone()],
            origin: crate::roots::RootsOrigin::EnvVar,
            skipped: Vec::new(),
            memory_fell_back: false,
        }),
        &profile.child("app-profile"),
        &mut startup,
    );
    assert!(matches!(got, Err(AlertScene::RootMissing(_))));
    let warned = named(&events, "startup_install_not_submitted");
    assert_eq!(warned.len(), 1, "{events:?}");
    assert_eq!(warned[0].level, tracing::Level::WARN);
    assert_eq!(warned[0].field_str("reason"), Some("root_missing"));
    assert!(named(&events, "startup_install_started").is_empty());
    assert!(
        archive.is_file() && !gone.exists(),
        "何も入らず何も消さない"
    );
    let ((), dropped) = log_capture_kit::capture(|| drop(startup));
    assert!(named(&dropped, "startup_install_not_submitted").is_empty());
}

/// 入れた後も 0 体（バルーンだけ入れた）なら「ゴーストなし」の場面で止まり、`boot_stopped` は出ない
/// （要件 4.11・6.3・9.7）。
#[test]
fn no_ghost_after_install_stops_with_ghost_missing_and_no_boot_stopped() {
    let root = root_with("startup-flow-balloon-only", &[], None);
    let dir = TempPath::under_target("startup-flow-nar");
    let mut startup = StartupInstall::new_with(args(vec![balloon_nar(&dir)], false), None);
    let (got, events) = boot(
        crate::roots::single_env(root.path().to_path_buf()),
        &root.child("app-profile"),
        &mut startup,
    );
    assert!(
        root.child("balloon").join("round").is_dir(),
        "バルーンは入る"
    );
    assert!(matches!(
        got,
        Err(AlertScene::GhostMissing { argv: None, .. })
    ));
    let ((), dropped) = log_capture_kit::capture(|| drop(startup));
    assert!(named(&events, "startup_install_not_submitted").is_empty());
    assert!(named(&dropped, "startup_install_not_submitted").is_empty());
}

/// 告知の抑止は `AREKA_NO_ALERT` の抑止と `AREKA_APP_SMOKE_EXIT_MS` の有無のどちらが立っても立つ
/// （要件 2.4・設計の討議 議題 1）。
#[test]
fn either_value_suppresses_the_alert_before_boot() {
    assert!(!before_boot_alert_suppressed(false, None));
    assert!(before_boot_alert_suppressed(true, None));
    assert!(before_boot_alert_suppressed(false, Some(0)));
    assert!(before_boot_alert_suppressed(true, Some(5000)));
}

fn carried(
    kind: areka_nar::InstallKind,
    folder: &str,
    ghost_folder: Option<&str>,
    ghost_name: Option<&str>,
) -> CarriedInstall {
    CarriedInstall {
        kind,
        folder: folder.to_owned(),
        ghost_folder: ghost_folder.map(str::to_owned),
        object_name: format!("{folder} の書庫"),
        ghost_name: ghost_name.map(str::to_owned),
    }
}

/// 控えは入れた順に窓口の記録と同じ受け皿と置換語へ置かれ（最後の値が残る）、info
/// `startup_install_carried` が 1 件。続けて窓口の記録の口で入れた物がそれを置き換える
/// （要件 3.2・3.4・9.12・10.6）。
#[test]
fn carried_installs_reach_the_desk_receivers_and_the_desk_replaces_them() {
    use crate::emo2_boot::ghost_switch::LastInstalledGhost;
    use crate::emo2_boot::shell_balloon_resolve::{LastInstalledBalloon, LastInstalledShell};
    use crate::install::names::LastInstallNames;
    use areka_nar::InstallKind;

    let mut world = World::new();
    let startup = StartupInstall {
        pending: None,
        carried: vec![
            carried(
                InstallKind::Ghost,
                "first",
                Some("first"),
                Some("ファースト"),
            ),
            carried(
                InstallKind::Shell,
                "dress",
                Some("first"),
                Some("ファースト"),
            ),
            carried(InstallKind::Balloon, "round", None, None),
            carried(
                InstallKind::Ghost,
                "second",
                Some("second"),
                Some("セカンド"),
            ),
            carried(
                InstallKind::Supplement,
                "extra",
                Some("second"),
                Some("セカンド"),
            ),
            carried(InstallKind::Balloon, "square", None, None),
        ],
    };
    let ((), events) = log_capture_kit::capture(|| install(&mut world, startup));
    assert_eq!(
        world
            .get_resource::<LastInstalledGhost>()
            .map(|g| g.0.as_str()),
        Some("second")
    );
    assert_eq!(
        world.get_resource::<LastInstalledShell>(),
        Some(&LastInstalledShell {
            ghost_folder: Some("first".to_owned()),
            folder: "dress".to_owned(),
        })
    );
    assert_eq!(
        world
            .get_resource::<LastInstalledBalloon>()
            .map(|b| b.0.as_str()),
        Some("square")
    );
    assert_eq!(
        world.get_resource::<LastInstallNames>(),
        Some(&LastInstallNames {
            ghost: Some("セカンド".to_owned()),
            object: Some("square の書庫".to_owned()),
        }),
        "バルーンは %lastghostname を前の値のまま"
    );
    let carried_log = named(&events, "startup_install_carried");
    assert_eq!(carried_log.len(), 1);
    assert_eq!(carried_log[0].level, tracing::Level::INFO);
    assert_eq!(carried_log[0].field("count"), Some("6"));

    super::super::desk::record_installed(
        &mut world,
        crate::install::procedure::InstalledRecord {
            kind: InstallKind::Ghost,
            object_name: "あとの書庫".to_owned(),
            folder: "later".to_owned(),
            ghost_folder: Some("later".to_owned()),
        },
    );
    assert_eq!(
        world
            .get_resource::<LastInstalledGhost>()
            .map(|g| g.0.as_str()),
        Some("later")
    );
    assert_eq!(
        world.get_resource::<LastInstallNames>(),
        Some(&LastInstallNames {
            ghost: Some("あとの書庫".to_owned()),
            object: Some("あとの書庫".to_owned()),
        })
    );
}

/// 控えが無ければ何も置かず、`startup_install_carried` も残さない。
#[test]
fn nothing_carried_places_nothing_and_logs_nothing() {
    let mut world = World::new();
    let ((), events) = log_capture_kit::capture(|| install(&mut world, StartupInstall::default()));
    assert!(
        world
            .get_resource::<crate::install::names::LastInstallNames>()
            .is_none()
    );
    assert!(named(&events, "startup_install_carried").is_empty());
}
