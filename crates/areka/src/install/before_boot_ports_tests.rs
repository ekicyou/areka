//! 起動の前の手続きの口の決定論テスト（design「Testing Strategy / Unit」・要件 1.4・2.4・2.5・2.7・
//! 3.1・3.4・9.3・9.4）。
//!
//! 確かめること: イベントはどれも「送らなかった」でどこへも送られない・素性は「在る・中身は空」・
//! `accept` 付きの書庫は断られて `install_refused_no_ghost` が出る・バルーンの控えが目録の綴りに
//! なり `install_balloon_memory_skipped` が出る・抑止なら利用条件が拒否になる・空の根に入れた
//! ゴーストが置き場に在り、控えが入れた順に取り出せる。画面は出さない（どれも抑止で回す）。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoots;
use areka_nar::InstallKind;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::install::procedure::{ArchiveEnd, run_order};
use crate::install::startup::CarriedInstall;
use crate::install::{InstallOrder, InstallOrigin};

/// 空の根の上の口（告知は抑止）。
fn ports(root: &TempPath) -> BootPorts {
    BootPorts::new(BasewareRoots::single(root.path().to_path_buf()), true)
}

fn count(events: &[CapturedEvent], name: &str, level: Level) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name) && e.level == level)
        .count()
}

/// `install.txt` の行で書庫を `dir` へ置く（ゴーストなら `ghost/master/descript.txt` も入れる）。
fn archive(dir: &TempPath, file: &str, lines: &[&str]) -> PathBuf {
    let path = dir.child(file);
    let mut builder = NarBuilder::new()
        .file("install.txt", &install_txt(lines))
        .done();
    if lines.contains(&"type,ghost") {
        builder = builder
            .file(
                "ghost/master/descript.txt",
                b"charset,UTF-8\r\nname,Newbie\r\n",
            )
            .done();
    } else {
        builder = builder
            .file("descript.txt", b"charset,UTF-8\r\nname,Fluffy\r\n")
            .done();
    }
    builder.write_to(&path).expect("書庫を置ける");
    path
}

fn order(archives: Vec<PathBuf>) -> InstallOrder {
    InstallOrder {
        archives,
        origin: InstallOrigin::StartupArgs,
    }
}

/// どのイベントにも「送らなかった」を返し、「閉じた」は返さない（要件 3.1・10.2）。
#[test]
fn every_event_is_unsent_never_closed() {
    let root = TempPath::under_target("before-boot-ports");
    let mut ports = ports(&root);
    for id in [
        "OnInstallBegin",
        "OnInstallComplete",
        "OnInstallCompleteEx",
        "OnInstallCompleteAll",
        "OnInstallFailure",
        "OnInstallRefuse",
    ] {
        assert_eq!(ports.raise(id, vec!["x".to_owned()]), Raised::Unsent);
    }
}

/// 素性は必ず在り、根の並びのほかは空（要件 2.5・2.6）。
#[test]
fn ghost_facts_are_present_and_empty() {
    let root = TempPath::under_target("before-boot-ports");
    let facts = ports(&root).ghost_facts().expect("素性は必ず在る");
    assert_eq!(
        facts.roots,
        BasewareRoots::single(root.path().to_path_buf())
    );
    assert_eq!(facts.folder, None);
    assert_eq!(facts.running_dir, None);
    assert_eq!(facts.name, "");
    assert_eq!(facts.sakura_name, None);
    assert!(facts.install_accept.is_empty());
}

/// 空の根に入れたゴーストは置き場に在り、イベントはどこへも送られず（info `install_event` 0 件・
/// debug `install_event_unsent` だけ）、控えが目録の綴りで残る（要件 1.4・3.1・3.4・9.4）。
#[test]
fn a_ghost_lands_in_the_empty_root_and_no_event_is_sent() {
    let root = TempPath::under_target("before-boot-ports");
    let dir = TempPath::under_target("before-boot-ports-nar");
    let path = archive(
        &dir,
        "newbie.nar",
        &[
            "charset,UTF-8",
            "type,ghost",
            "name,あたらしい",
            "directory,newbie",
        ],
    );
    let mut ports = ports(&root);
    let (ends, events) = capture(|| run_order(&order(vec![path]), &mut ports));
    assert!(
        matches!(ends.as_slice(), [ArchiveEnd::Installed(_)]),
        "{ends:?}"
    );
    assert!(
        root.path()
            .join("ghost/newbie/ghost/master/descript.txt")
            .is_file(),
        "窓へ落としたときと同じ置き場に在る"
    );
    assert_eq!(
        count(&events, "install_event", Level::INFO),
        0,
        "{events:?}"
    );
    assert!(count(&events, "install_event_unsent", Level::DEBUG) >= 2);
    assert_eq!(
        ports.take_carried(),
        vec![CarriedInstall {
            kind: InstallKind::Ghost,
            folder: "newbie".to_owned(),
            ghost_folder: Some("newbie".to_owned()),
            object_name: "あたらしい".to_owned(),
            ghost_name: Some("Newbie".to_owned()),
        }]
    );
    assert!(ports.take_carried().is_empty(), "取り出したら空");
}

/// `accept` 付きのゴーストの書庫は、受け取るゴーストが居ないので断られ、info
/// `install_refused_no_ghost` が書庫のパスつきで 1 件出る。根には何も入らない（要件 2.5）。
#[test]
fn an_archive_with_accept_is_refused_and_logged() {
    let root = TempPath::under_target("before-boot-ports");
    let dir = TempPath::under_target("before-boot-ports-nar");
    let path = archive(
        &dir,
        "picky.nar",
        &[
            "charset,UTF-8",
            "type,ghost",
            "name,えらぶ",
            "directory,picky",
            "accept,さくら",
        ],
    );
    let mut ports = ports(&root);
    let (ends, events) = capture(|| run_order(&order(vec![path.clone()]), &mut ports));
    assert_eq!(ends, [ArchiveEnd::Refused]);
    let refused: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_refused_no_ghost"))
        .collect();
    assert_eq!(refused.len(), 1, "{events:?}");
    assert_eq!(refused[0].level, Level::INFO);
    assert_eq!(
        refused[0].field("archive"),
        Some(path.display().to_string().as_str())
    );
    assert!(!root.path().join("ghost").join("picky").exists());
    assert!(ports.take_carried().is_empty());
}

/// 根に `balloon/<folder>/descript.txt` を置く。
fn plant_balloon(root: &Path, folder: &str) {
    let dir = root.join("balloon").join(folder);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("descript.txt"),
        b"charset,UTF-8\r\nname,Fluffy\r\n",
    )
    .unwrap();
}

/// バルーンの控えは目録の綴りになり、記憶へは書かず info `install_balloon_memory_skipped` が
/// 1 件出る（要件 2.7・3.4）。
#[test]
fn a_balloon_is_carried_in_the_catalog_spelling_and_memory_is_skipped() {
    let root = TempPath::under_target("before-boot-ports");
    plant_balloon(root.path(), "Fluffy");
    let mut ports = ports(&root);
    let ((), events) = capture(|| {
        ports.record(InstalledRecord {
            kind: InstallKind::Balloon,
            object_name: "ふわふわ".to_owned(),
            folder: "fluffy".to_owned(),
            ghost_folder: None,
        })
    });
    let skipped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_balloon_memory_skipped"))
        .collect();
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, Level::INFO);
    assert_eq!(skipped[0].field_str("folder"), Some("Fluffy"));
    assert_eq!(
        ports.take_carried(),
        vec![CarriedInstall {
            kind: InstallKind::Balloon,
            folder: "Fluffy".to_owned(),
            ghost_folder: None,
            object_name: "ふわふわ".to_owned(),
            ghost_name: None,
        }]
    );
}

/// 控えは入れた順に取り出せる。ゴーストの記録ではバルーンの記録は出ない（要件 3.4）。
#[test]
fn carried_installs_come_out_in_order() {
    let root = TempPath::under_target("before-boot-ports");
    let mut ports = ports(&root);
    let record = |kind, folder: &str, ghost_folder: Option<&str>| InstalledRecord {
        kind,
        object_name: folder.to_owned(),
        folder: folder.to_owned(),
        ghost_folder: ghost_folder.map(str::to_owned),
    };
    let ((), events) = capture(|| {
        ports.record(record(InstallKind::Ghost, "kou", Some("kou")));
        ports.record(record(InstallKind::Shell, "sh", Some("kou")));
    });
    assert_eq!(
        count(&events, "install_balloon_memory_skipped", Level::INFO),
        0
    );
    let folders: Vec<_> = ports
        .take_carried()
        .into_iter()
        .map(|c| (c.kind, c.folder))
        .collect();
    assert_eq!(
        folders,
        [
            (InstallKind::Ghost, "kou".to_owned()),
            (InstallKind::Shell, "sh".to_owned())
        ]
    );
}

/// 抑止なら利用条件の画面を出さず「抑止」を返し、手続きは拒否にする（要件 2.4）。
#[test]
fn suppressed_terms_are_declined() {
    let root = TempPath::under_target("before-boot-ports");
    let dir = TempPath::under_target("before-boot-ports-nar");
    let path = dir.child("terms.nar");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,条件つき",
                "directory,termsy",
            ]),
        )
        .done()
        .file("terms.txt", "使う前に読んでください".as_bytes())
        .done()
        .file(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\nname,Termsy\r\n",
        )
        .done()
        .write_to(&path)
        .expect("書庫を置ける");
    let mut ports = ports(&root);
    let ends = run_order(&order(vec![path]), &mut ports);
    assert_eq!(ends, [ArchiveEnd::Declined]);
    assert!(!root.path().join("ghost").join("termsy").exists());
    assert!(ports.take_carried().is_empty());
}
