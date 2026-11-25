//! 入れた後の記録の決定論テスト。
//!
//! 確かめること: ゴーストなら受け皿へ目録の綴りのフォルダ名が書かれる・バルーンだけなら今のゴーストの
//! 「最後に使ったバルーン」の記憶が目録の綴りへ替わる・シェルは記録だけで表示を替えない・置換語の
//! 名前は目録の `name`・どの場合も反映が済んでから答える。
//!
//! 偽の SHIORI の土台で本物の実行系を起こし（今のゴーストの記憶の書き手が本物）、目録の綴りと隠れの
//! 判定は背景のスレッドの口と同じ [`carry_record`] で済ませ、窓口には綴り直した値だけを頼む（窓口は
//! 目録を読まない）。頼みは窓口の頼みの送出端へ直接入れて取り出しの系 [`drain`] を直接呼ぶ。記憶は
//! 答えを受けた後に実 fs から読む（答えの前に反映の柵を掛けているので、時間では待たない）。

use areka_actor::reply_channel;
use areka_nar::InstallKind;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::*;
use crate::install::names::LastInstallNames;
use crate::install::procedure::InstalledRecord;
use crate::install::worker::{carry_record, catalog_spelling};
use crate::sample_test_support::switch_rig;
use crate::session_tests::{count, events_named, write_descript};
use areka_runtime::shell_balloon_resolve::{LastInstalledBalloon, LastInstalledShell};
use areka_session::boot_resolve::read_last_balloon;
use areka_session::ghost_switch::LastInstalledGhost;
use areka_session::ghost_switch_test_support::{BALLOON, FakeShiori, SwitchRig, standard_script};

/// 今のゴースト A を起こした土台（受け皿・置換語の値はまだ無い）。
fn rig() -> SwitchRig {
    let mut rig = switch_rig(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.boot("A");
    rig
}

/// 入れた後の記録を今のゴースト A の素性（根と場所）で綴り直し、窓口へ 1 件頼んで取り出しの系を
/// 1 回回し、答えが直ちに返ったかと、綴り直しと窓口の記録を合わせて返す。
fn record(rig: &mut SwitchRig, record: InstalledRecord) -> (bool, Vec<CapturedEvent>) {
    let roots = rig.world.resource::<BootContext>().roots.clone();
    let dir = rig.root.ghost_dir("A");
    let (installed, mut events) = capture(|| carry_record(Some(&roots), Some(&dir), record));
    let (reply, answer) = reply_channel();
    rig.world
        .non_send::<InstallDesk>()
        .asks_tx
        .send(DeskAsk::Record { installed, reply })
        .expect("窓口の頼みの受信端は生きている");
    let ((), desk_events) = capture(|| drain(&mut rig.world));
    events.extend(desk_events);
    (matches!(answer.try_recv(), Ok(Some(()))), events)
}

/// 目録に置くゴースト（`ghost/<folder>/ghost/master/descript.txt` の `name`）。
fn plant_ghost(rig: &SwitchRig, folder: &str, name: &str) {
    write_descript(
        &rig.root.ghost_dir(folder).join("ghost").join("master"),
        &format!("name,{name}"),
    );
}

/// 目録に置くバルーン（`balloon/<folder>/descript.txt` の `name`）。
fn plant_balloon(rig: &SwitchRig, folder: &str, name: &str) {
    write_descript(&rig.root.balloon_dir(folder), &format!("name,{name}"));
}

fn last_installed(rig: &SwitchRig) -> Option<String> {
    rig.world
        .get_resource::<LastInstalledGhost>()
        .map(|g| g.0.clone())
}

fn names(rig: &SwitchRig) -> Option<LastInstallNames> {
    rig.world.get_resource::<LastInstallNames>().cloned()
}

/// ゴーストを入れた: 受け皿へ目録の綴りのフォルダ名（書庫の `directory` と大文字小文字だけが違う）、
/// 置換語は目録の `name` と `install.txt` の `name`。バルーンの記憶は替わらない。
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

/// 1 回の依頼で 2 体入れたら、受け皿は最後のゴースト。
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
/// `info!(install_balloon_remembered)` が 1 件。受け皿は書かず、切り替えない。
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
    let remembered: Vec<_> = events_named(&events, "install_balloon_remembered");
    assert_eq!(remembered.len(), 1);
    assert_eq!(remembered[0].level, Level::INFO);
    assert_eq!(remembered[0].field("folder"), Some("Fluffy"));
    assert_eq!(last_installed(&rig), None, "受け皿は書かない");
    assert_eq!(
        rig.world.get_resource::<LastInstalledBalloon>(),
        Some(&LastInstalledBalloon("Fluffy".to_owned())),
        "バルーンの lastinstalled の控えは目録の綴り"
    );
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

/// シェルを入れた: 記録（置換語）とシェルの `lastinstalled` の控えだけで、受け皿もバルーンの記憶も
/// 替えず、切り替えもしない。`%lastghostname` は宛先のゴーストの目録の `name`。
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
        rig.world.get_resource::<LastInstalledShell>(),
        Some(&LastInstalledShell {
            ghost_folder: Some("a".to_owned()),
            folder: "summer".to_owned(),
        })
    );
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

/// バルーンだけの書庫を入れた後の記録を頼む（書庫の綴り `fluffy`）。
fn record_fluffy(rig: &mut SwitchRig) -> Vec<CapturedEvent> {
    let (answered, events) = record(
        rig,
        InstalledRecord {
            kind: InstallKind::Balloon,
            object_name: "ふわふわ".to_owned(),
            folder: "fluffy".to_owned(),
            ghost_folder: None,
        },
    );
    assert!(answered, "反映が済んでから答える");
    events
}

/// バルーンだけの書庫は根へ入り、今のゴーストの中に同じフォルダ名（大文字小文字違い）が在れば
/// info `installed_balloon_hidden_by_ghost` が 1 件（`installed`＝根の場所・`by`＝ゴーストの中の場所）。
/// 控えと記憶は根の目録の綴りで書く。
#[test]
fn a_balloon_hidden_by_the_running_ghost_is_recorded_once() {
    let mut rig = rig();
    plant_balloon(&rig, "Fluffy", "ふわふわ");
    let by = rig.add_ghost_balloon_copy("A", "FLUFFY", "中のふわふわ");
    let events = record_fluffy(&mut rig);
    let hidden: Vec<_> = events_named(&events, "installed_balloon_hidden_by_ghost");
    assert_eq!(hidden.len(), 1, "{events:?}");
    assert_eq!(hidden[0].level, Level::INFO);
    assert_eq!(hidden[0].field_str("folder"), Some("Fluffy"));
    let installed = rig.root.balloon_dir("Fluffy");
    assert_eq!(
        hidden[0].field("installed"),
        Some(installed.display().to_string().as_str()),
        "入れた先は根"
    );
    assert_eq!(
        hidden[0].field("by"),
        Some(by.display().to_string().as_str())
    );
    assert_eq!(
        rig.world.get_resource::<LastInstalledBalloon>(),
        Some(&LastInstalledBalloon("Fluffy".to_owned())),
        "控えは根の目録の綴り"
    );
    assert_eq!(
        read_last_balloon(&rig.root.ghost_dir("A")).as_deref(),
        Some("Fluffy"),
        "記憶は根の目録の綴り"
    );
    assert!(rig.shutdown());
}

/// 今のゴーストの中に同じフォルダ名が無ければ記録 0 件。
#[test]
fn a_balloon_not_hidden_by_the_running_ghost_is_not_recorded() {
    let mut rig = rig();
    plant_balloon(&rig, "Fluffy", "ふわふわ");
    rig.add_ghost_balloon_copy("A", "other", "別のもの");
    let events = record_fluffy(&mut rig);
    assert_eq!(
        count(&events, "installed_balloon_hidden_by_ghost", Level::INFO),
        0
    );
    assert!(rig.shutdown());
}

/// 根が無いとき、目録の綴りは書庫の綴りのまま・%lastghostname はゴーストなら `name`、他は関わった
/// ゴーストのフォルダ名・関わったゴーストが無ければ None（窓口と起動の前の口の共通の決まり）。
#[test]
fn catalog_spelling_without_roots_keeps_the_archive_spelling() {
    let record = |kind, ghost_folder: Option<&str>| InstalledRecord {
        kind,
        object_name: "名前".to_owned(),
        folder: "dir".to_owned(),
        ghost_folder: ghost_folder.map(str::to_owned),
    };
    let ghost = catalog_spelling(None, &record(InstallKind::Ghost, Some("dir")));
    assert_eq!(ghost, ("dir".to_owned(), Some("名前".to_owned())));
    let shell = catalog_spelling(None, &record(InstallKind::Shell, Some("g")));
    assert_eq!(shell, ("dir".to_owned(), Some("g".to_owned())));
    let balloon = catalog_spelling(None, &record(InstallKind::Balloon, None));
    assert_eq!(balloon, ("dir".to_owned(), None));
}

/// 1 回目の書き込みで「入った」を知らせ、門の送り手が捨てられるまで止まる偽の IO。
struct GatedFirstCommit {
    entered: std::sync::mpsc::Sender<()>,
    open: std::sync::Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}

impl areka_sylphya::persist::PersistIo for GatedFirstCommit {
    fn read(&self, _path: &std::path::Path) -> std::io::Result<Option<String>> {
        Ok(None)
    }
    fn commit(&self, _path: &std::path::Path, _content: &str) -> std::io::Result<()> {
        // 受け手は文を分けて取り出す（if let の行に書くと、止まっている間も Mutex を握り続ける）。
        let open = self.open.lock().unwrap().take();
        if let Some(open) = open {
            let _ = self.entered.send(());
            let _ = open.recv_timeout(areka_runtime::spine::SPIN_WAIT);
        }
        Ok(())
    }
}

/// 書き手が書き込みの最中で止まっていても、記録の待ちは上限で戻り、`install_record_unsettled` を 1 件残す。
#[test]
fn a_stuck_writer_is_waited_for_only_up_to_the_limit() {
    use areka_runtime::spine::{SPIN_WAIT, join_bounded, join_thread_bounded};
    use areka_sylphya::{ScopeRoots, SylphyaInit, spawn_sylphya};

    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    // 門の送り手は待ちより先に捨てない（赤で巻き戻るときも捨てられて門が開く）。
    let (open_tx, open_rx) = std::sync::mpsc::channel::<()>();
    let parts = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        },
        io: Box::new(GatedFirstCommit {
            entered: entered_tx,
            open: std::sync::Mutex::new(Some(open_rx)),
        }),
        runtime_sink: None,
    });
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "1".into())],
    );
    entered_rx
        .recv_timeout(SPIN_WAIT)
        .expect("書き手が書き込みに入る");

    let publisher = parts.publisher.clone();
    let settler = std::thread::spawn(move || capture(|| settle_record(&publisher)).1);
    let events = join_thread_bounded("settle_record", None, settler);
    drop(open_tx);
    parts.publisher.close();
    join_bounded("sylphya", parts.handle).expect("書き手が閉じる");

    let events = events.expect("記録の待ちのスレッドは倒れない");
    assert_eq!(
        count(&events, "install_record_unsettled", Level::ERROR),
        1,
        "{events:?}"
    );
}

/// 置き場にゴーストが居なければ、バルーンの記憶を書かずに警告を 1 件残す。
#[test]
fn an_empty_slot_leaves_one_warning_instead_of_the_balloon_memory() {
    let mut world = desk_world();
    world.insert_non_send(areka_session::ghost_session::GhostSlot(None));
    let ((), events) = capture(|| remember_balloon(&world, "kaku".to_owned()));
    assert_eq!(
        count(&events, "install_balloon_not_remembered", Level::WARN),
        1,
        "{events:?}"
    );
    assert_eq!(count(&events, "install_balloon_remembered", Level::INFO), 0);
}
