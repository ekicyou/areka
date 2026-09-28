//! 入れた後の記録の決定論テスト（design「areka / install / desk」の入れた後の記録・設計で決めたこと
//! 14・要件 6.1・6.4・6.6・6.7・6.8・12.14）。
//!
//! 確かめること: ゴーストなら受け皿へ目録の綴りのフォルダ名が書かれる・バルーンだけなら今のゴーストの
//! 「最後に使ったバルーン」の記憶が目録の綴りへ替わる・シェルは記録だけで表示を替えない・置換語の
//! 名前は目録の `name`・どの場合も反映が済んでから答える。
//!
//! 偽の SHIORI の土台で本物の実行系を起こし（今のゴーストの記憶の書き手が本物）、頼みは窓口の
//! 頼みの送出端へ直接入れて取り出しの系 [`drain`] を直接呼ぶ。記憶は答えを受けた後に実 fs から読む
//! （答えの前に反映の柵を掛けているので、時間では待たない）。

use areka_actor::reply_channel;
use areka_nar::InstallKind;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::*;
use crate::boot_resolve::read_last_balloon;
use crate::emo2_boot::ghost_switch::LastInstalledGhost;
use crate::emo2_boot::ghost_switch_test_support::{
    BALLOON, FakeShiori, SwitchRig, standard_script,
};
use crate::install::names::LastInstallNames;
use crate::install::procedure::InstalledRecord;

/// 今のゴースト A を起こした土台（受け皿・置換語の値はまだ無い）。
fn rig() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.boot("A");
    rig
}

/// 入れた後の記録を 1 件頼んで取り出しの系を 1 回回し、答えが直ちに返ったかと記録を返す。
fn record(rig: &mut SwitchRig, record: InstalledRecord) -> (bool, Vec<CapturedEvent>) {
    let (reply, answer) = reply_channel();
    rig.world
        .non_send::<InstallDesk>()
        .asks_tx
        .send(DeskAsk::Record { record, reply })
        .expect("窓口の頼みの受信端は生きている");
    let ((), events) = capture(|| drain(&mut rig.world));
    (matches!(answer.try_recv(), Ok(Some(()))), events)
}

/// 目録に置くゴースト（`ghost/<folder>/ghost/master/descript.txt` の `name`）。
fn plant_ghost(rig: &SwitchRig, folder: &str, name: &str) {
    let master = rig.root.ghost_dir(folder).join("ghost").join("master");
    std::fs::create_dir_all(&master).unwrap();
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\r\nname,{name}\r\n"),
    )
    .unwrap();
}

/// 目録に置くバルーン（`balloon/<folder>/descript.txt` の `name`）。
fn plant_balloon(rig: &SwitchRig, folder: &str, name: &str) {
    let dir = rig.root.balloon_dir(folder);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\nname,{name}\r\n"),
    )
    .unwrap();
}

fn last_installed(rig: &SwitchRig) -> Option<String> {
    rig.world
        .get_resource::<LastInstalledGhost>()
        .map(|g| g.0.clone())
}

fn names(rig: &SwitchRig) -> Option<LastInstallNames> {
    rig.world.get_resource::<LastInstallNames>().cloned()
}

fn count(events: &[CapturedEvent], name: &str, level: Level) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name) && e.level == level)
        .count()
}

/// ゴーストを入れた: 受け皿へ目録の綴りのフォルダ名（書庫の `directory` と大文字小文字だけが違う）、
/// 置換語は目録の `name` と `install.txt` の `name`。バルーンの記憶は替わらない（要件 6.1・6.8）。
#[test]
fn a_ghost_goes_to_last_installed_in_the_catalog_spelling() {
    let mut rig = rig();
    plant_ghost(&rig, "NewBie", "ニュービー");
    let (answered, events) = record(
        &mut rig,
        InstalledRecord {
            kind: InstallKind::Ghost,
            object_name: "あたらしいゴースト".to_owned(),
            folder: "newbie".to_owned(),
            ghost_folder: Some("newbie".to_owned()),
        },
    );
    assert!(answered, "反映が済んでから答える");
    assert_eq!(last_installed(&rig).as_deref(), Some("NewBie"));
    assert_eq!(count(&events, "last_installed_recorded", Level::INFO), 1);
    assert_eq!(
        names(&rig),
        Some(LastInstallNames {
            ghost: Some("ニュービー".to_owned()),
            object: Some("あたらしいゴースト".to_owned()),
        })
    );
    assert_eq!(
        read_last_balloon(&rig.root.ghost_dir("A")).as_deref(),
        Some(BALLOON),
        "バルーンの記憶は替わらない"
    );
    assert_eq!(count(&events, "install_balloon_remembered", Level::INFO), 0);
    assert!(rig.shutdown());
}

/// 1 回の依頼で 2 体入れたら、受け皿は最後のゴースト（要件 6.4）。
#[test]
fn several_ghosts_leave_the_last_in_last_installed() {
    let mut rig = rig();
    plant_ghost(&rig, "Kou", "甲");
    plant_ghost(&rig, "Otsu", "乙");
    for (folder, object) in [("kou", "ゴースト甲"), ("otsu", "ゴースト乙")] {
        let (answered, _) = record(
            &mut rig,
            InstalledRecord {
                kind: InstallKind::Ghost,
                object_name: object.to_owned(),
                folder: folder.to_owned(),
                ghost_folder: Some(folder.to_owned()),
            },
        );
        assert!(answered);
    }
    assert_eq!(last_installed(&rig).as_deref(), Some("Otsu"));
    assert_eq!(names(&rig).and_then(|n| n.ghost).as_deref(), Some("乙"));
    assert!(rig.shutdown());
}

/// バルーンだけを入れた: 今のゴーストの「最後に使ったバルーン」の記憶が目録の綴りへ替わり、
/// `info!(install_balloon_remembered)` が 1 件。受け皿は書かず、切り替えない（要件 6.6・12.14）。
#[test]
fn a_balloon_only_install_rewrites_the_running_ghosts_balloon_memory() {
    let mut rig = rig();
    plant_balloon(&rig, "Fluffy", "ふわふわ");
    let (answered, events) = record(
        &mut rig,
        InstalledRecord {
            kind: InstallKind::Balloon,
            object_name: "ふわふわ".to_owned(),
            folder: "fluffy".to_owned(),
            ghost_folder: None,
        },
    );
    assert!(answered, "反映が済んでから答える");
    assert_eq!(
        read_last_balloon(&rig.root.ghost_dir("A")).as_deref(),
        Some("Fluffy"),
        "目録の綴りで書く"
    );
    let remembered: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_balloon_remembered"))
        .collect();
    assert_eq!(remembered.len(), 1);
    assert_eq!(remembered[0].level, Level::INFO);
    assert_eq!(remembered[0].field("folder"), Some("Fluffy"));
    assert_eq!(last_installed(&rig), None, "受け皿は書かない");
    assert!(rig.world.get_non_send::<SwitchInFlight>().is_none());
    assert_eq!(
        names(&rig),
        Some(LastInstallNames {
            ghost: None,
            object: Some("ふわふわ".to_owned()),
        })
    );
    assert!(rig.shutdown());
}

/// シェルを入れた: 記録（置換語）だけで、受け皿もバルーンの記憶も替えず、切り替えもしない。
/// `%lastghostname` は宛先のゴーストの目録の `name`（要件 6.7・6.8）。
#[test]
fn a_shell_install_only_records_names() {
    let mut rig = rig();
    let (answered, events) = record(
        &mut rig,
        InstalledRecord {
            kind: InstallKind::Shell,
            object_name: "夏服".to_owned(),
            folder: "summer".to_owned(),
            ghost_folder: Some("a".to_owned()),
        },
    );
    assert!(answered);
    assert_eq!(last_installed(&rig), None);
    assert_eq!(
        read_last_balloon(&rig.root.ghost_dir("A")).as_deref(),
        Some(BALLOON)
    );
    assert_eq!(count(&events, "install_balloon_remembered", Level::INFO), 0);
    assert!(rig.world.get_non_send::<SwitchInFlight>().is_none());
    assert_eq!(
        names(&rig),
        Some(LastInstallNames {
            ghost: Some("A".to_owned()),
            object: Some("夏服".to_owned()),
        })
    );
    assert!(rig.shutdown());
}
