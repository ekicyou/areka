//! 起動中のゴーストへ入れる一周の決定論テスト（design「System Flows／起動中のゴーストへ入れる一周」・
//! 「Testing Strategy / 窓口と入口」・要件 6.5・7.1・7.2・7.3・11.7）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、本物の窓口・kanade・背景のスレッド・口（`DeskPorts`）・切替の
//! 道筋を通し、偽の SHIORI に届いた呼び出しの列と UI スレッドの記録の順を突き合わせる。確かめること:
//! `OnGhostChanging` も `OnClose` も 0 件・降ろした後（全窓を閉じた後・起こす前）に宛先の中身が
//! 替わる・同じゴーストが起き直す・定常到達の後に締めの知らせが出る。
//!
//! 切替の入口の判定と失敗の場合（要件 7.4〜7.7・7.9・11.7）: 別の切替の最中（`Busy`）は預かったまま
//! その予約が下りた tick に頼み直して一周する・別のゴーストへの切替では展開せず書庫を返す・目録に無い
//! （`NotFound`）ときはよそへの展開で続いて締めの知らせが 1 つ出る・確定が失敗すれば同じゴーストが元の
//! 中身で起き直って `OnInstallFailure` が出る・起こせなければ今日の切替の道（既定へ戻す・既定なら致命）に
//! 乗る。軽い判定（中止で「預かった」へ戻る・`NoContext`）は `desk_tests.rs`。
//!
//! 実時間の待ちに依らない: 台詞の時計は合成の Tick で進め（再生中の台詞の終わりを kanade の保留が
//! 待つ）、依頼の終わりは窓口が背景のスレッドから終わりの知らせを受けたことで揃える。

use std::path::{Path, PathBuf};

use areka_actor::reply_channel;
use areka_kanade::ChangeOrigin;
use areka_nar::NarArchive;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;
use wintf::ecs::Input;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::InstallDesk;
use super::overwrite::take;
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::boot_config::BootContext;
use crate::boot_resolve::DEFAULT_GHOST_FOLDER;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, LastInstalledGhost, PrevGhost, SwitchInFlight, SwitchRequest, SwitchStage,
    SwitchTarget, SwitchVerdict, request_ghost_switch,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::{Progress, RecordedCall, run_bounded_watching};
use crate::install::procedure::Overwritten;
use crate::install::{InstallOrder, InstallOrigin, SubmitVerdict, submit};
use crate::placement::persist::PersistWiring;

/// 書庫が置くファイル（宛先のゴーストのフォルダからの相対）と中身。
const MARKER: &str = "ghost/master/overwritten.txt";
const MARKER_BODY: &[u8] = b"new";

/// 呼び出しの名前の列（Reference は見ない）。
fn ids(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|call| match call {
            RecordedCall::Get { id, .. } | RecordedCall::Notify { id, .. } => id.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

/// `name` の記録が最初に現れた位置。
fn first(events: &[CapturedEvent], name: &str) -> Option<usize> {
    events
        .iter()
        .position(|e| e.field_str("event") == Some(name))
}

/// 呼び出しの列の確かめに添える手がかり: 起こした回ごとの列・`OnInstallFailure` の Reference・捕まえた
/// 記録のうち warn 以上。手続きの失敗の詳しい記録（`install_failed`・`areka_nar` の失敗）は背景の
/// スレッド `install` で出るので、呼び手のスレッドだけを捕える `capture` には映らない。Reference の語
/// （`extraction` は展開か確定の入出力の失敗）がその代わりになる。
fn clues(rig: &SwitchRig, folder: &str, events: &[CapturedEvent]) -> String {
    let failures: Vec<_> = rig
        .calls(folder)
        .iter()
        .filter_map(|c| refs_of(c, "OnInstallFailure"))
        .collect();
    let troubles: Vec<_> = events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect();
    format!(
        "{:?}／OnInstallFailure の Reference: {failures:?}／warn 以上の記録: {troubles:?}",
        boots_of(rig, folder)
    )
}

/// 起動中のゴースト A（`directory,A` の `ghost`）へ、ファイルを 1 つ足す書庫を本番の道筋で入れる。
#[test]
fn overwriting_the_running_ghost_takes_it_down_installs_and_boots_it_again() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script(r"\0A\e")
                .get("OnGhostChanged", Ok(None))
                .get("OnInstallBegin", Ok(None))
                .get("OnInstallCompleteEx", Ok(None))
                .get("OnInstallComplete", Ok(None))
        })),
    )]);
    // 起こし直すときの窓の準備が閉包を投函する先（`Input` の段では走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    let marker = rig.root.ghost_dir("A").join(MARKER);
    assert!(!marker.exists(), "入れる前は無い");

    let dir = TempPath::new("desk-overwrite-running");
    let path = dir.child("a.nar");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&["charset,UTF-8", "type,ghost", "name,A", "directory,A"]),
        )
        .done()
        .file(MARKER, MARKER_BODY)
        .done()
        .write_to(&path)
        .expect("書庫を置ける");
    let order = InstallOrder {
        archives: vec![path],
        origin: InstallOrigin::Menu,
    };
    assert_eq!(submit(&mut rig.world, order), SubmitVerdict::Queued);

    let (finished, events) = capture(|| {
        rig.pump_talking_until(|rig| {
            let desk = rig.world.non_send::<InstallDesk>();
            rig.exit_requested() || (desk.queue.is_empty() && !desk.busy)
        })
    });
    assert!(finished, "依頼が終わる（期限切れ）: {events:?}");
    assert!(!rig.exit_requested(), "終了しない: {events:?}");

    let boots: Vec<Vec<String>> = rig.calls("A").iter().map(|c| ids(c)).collect();
    let current = rig
        .world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone());
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let last = rig
        .world
        .get_resource::<LastInstalledGhost>()
        .map(|g| g.0.clone());
    let body = std::fs::read(&marker).ok();
    let clue = clues(&rig, "A", &events);
    assert!(rig.shutdown());

    // 同じゴーストが起き直す（ほかのゴーストは起こさない）。予約は定常到達で下りている。
    assert_eq!(boots.len(), 2, "A を 1 回起こし直す: {clue}");
    assert_eq!(current.as_deref(), Some("A"));
    assert!(!reserved, "予約は残らない");
    // 知らせを送らない切替（要件 7.2）。
    for name in ["OnGhostChanging", "OnClose"] {
        assert!(
            boots.iter().flatten().all(|id| id != name),
            "{name} は 0 件: {boots:?}"
        );
    }
    // 始まりの知らせは降ろす前のゴーストへ、締めの知らせは起こし直したゴーストの起動の後へ。
    assert!(
        boots[0].iter().any(|id| id == "OnInstallBegin"),
        "{boots:?}"
    );
    assert!(
        boots[0]
            .iter()
            .all(|id| !id.starts_with("OnInstallComplete")),
        "{boots:?}"
    );
    let second = &boots[1];
    let booted = second
        .iter()
        .position(|id| id == "basewareversion")
        .expect("起こし直した起動の系列");
    assert_eq!(
        second[booted + 1..],
        ["OnInstallCompleteEx", "OnInstallComplete"],
        "{clue}"
    );
    // 降ろした後に宛先の中身が替わる（全窓を閉じた後・起こす前に展開する＝要件 7.3）。
    assert_eq!(body.as_deref(), Some(MARKER_BODY));
    let order_of = |name| first(&events, name).unwrap_or_else(|| panic!("{name}: {events:?}"));
    let down = order_of("ghost_switch_down_ms");
    let done = order_of("install_overwrite_done");
    let booted = order_of("ghost_switch_booted");
    let steady = order_of("ghost_switch_done");
    assert!(down < done && done < booted, "{events:?}");
    let done_event = &events[done];
    assert_eq!(done_event.field("ok"), Some("true"), "{done_event:?}");
    assert!(done_event.field("ms").is_some(), "{done_event:?}");
    // 締めの知らせは定常到達の後に送る。
    let sent_ex = events
        .iter()
        .position(|e| {
            e.field_str("event") == Some("install_event_sent")
                && e.field_str("id") == Some("OnInstallCompleteEx")
        })
        .expect("締めの知らせを送った");
    assert!(steady < sent_ex, "{events:?}");
    // 起動中のゴースト自身を入れても、追加の切替はしない（要件 6.5）。受け皿へは書く。
    assert_eq!(
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("ghost_switch_requested"))
            .count(),
        1,
        "{events:?}"
    );
    assert_eq!(last.as_deref(), Some("A"));
}

// ---------------------------------------------------------------- 切替の入口の判定と失敗の場合

/// 入れる知らせと起こし直しに応える偽の SHIORI（起こすたびに新品の台本）。
fn installing(folder: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        standard_script(&format!(r"\0{folder}\e"))
            .get("OnGhostChanged", Ok(None))
            .get("OnInstallBegin", Ok(None))
            .get("OnInstallCompleteEx", Ok(None))
            .get("OnInstallComplete", Ok(None))
            .get("OnInstallFailure", Ok(None))
    }))
}

/// `folder` を起こして定常に着いた土台（`others` は切替で起こせるゴースト）。
fn running(folder: &'static str, others: &[&'static str]) -> SwitchRig {
    let mut scripts = vec![(folder, installing(folder))];
    scripts.extend(others.iter().map(|other| (*other, installing(other))));
    let mut rig = SwitchRig::new(scripts);
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    for name in std::iter::once(&folder).chain(others) {
        rig.plant_boot_record(name);
    }
    rig.boot(folder);
    assert!(rig.wait_steady(), "{folder} が定常に着く");
    // 起動の直後の記憶（`LastShell`・`LastBalloon`＝ゴーストのフォルダの下の `profile/areka`）は投函だけで
    // 待たない。負荷で書き手が遅れると、定常の後に始めた展開（宛先のフォルダの名前替え）と書き込みが
    // 重なって確定が拒まれる。書き手のフェンスで書き込みを済ませてから渡す。
    let publisher = rig.world.non_send::<PersistWiring>().publisher.clone();
    run_bounded_watching(
        "起動の直後の記憶の書き込み",
        Progress::Unknown,
        move || {
            publisher.barrier().expect("記憶の書き手が生きている");
        },
    );
    rig
}

/// `directory,<folder>` の `ghost` の書庫（`extra` は `install.txt` に足す行・中身は目印 1 つ）。
fn write_archive(dir: &TempPath, folder: &str, extra: &[&str]) -> PathBuf {
    let path = dir.child(&format!("{folder}.nar"));
    let (name, directory) = (format!("name,{folder}"), format!("directory,{folder}"));
    let mut lines = vec![
        "charset,UTF-8",
        "type,ghost",
        name.as_str(),
        directory.as_str(),
    ];
    lines.extend(extra);
    NarBuilder::new()
        .file("install.txt", &install_txt(&lines))
        .done()
        .file(MARKER, MARKER_BODY)
        .done()
        .write_to(&path)
        .expect("書庫を置ける");
    path
}

/// 書庫をメニューから入れる依頼を出し、依頼が終わる（窓口の待ち行列が空で背景のスレッドが空いた）か
/// 終了が指示されるまで台詞を進めながら回す（期限切れは `false`）。
fn install_through(rig: &mut SwitchRig, path: PathBuf) -> bool {
    let order = InstallOrder {
        archives: vec![path],
        origin: InstallOrigin::Menu,
    };
    assert_eq!(submit(&mut rig.world, order), SubmitVerdict::Queued);
    rig.pump_talking_until(|rig| {
        let desk = rig.world.non_send::<InstallDesk>();
        rig.exit_requested() || (desk.queue.is_empty() && !desk.busy)
    })
}

/// 起こした回ごとの呼び出しの名前の列。
fn boots_of(rig: &SwitchRig, folder: &str) -> Vec<Vec<String>> {
    rig.calls(folder).iter().map(|c| ids(c)).collect()
}

/// 起動の系列（`basewareversion`）の後の呼び出しの名前。
fn after_boot(calls: &[String]) -> &[String] {
    let booted = calls
        .iter()
        .position(|id| id == "basewareversion")
        .unwrap_or_else(|| panic!("起動の系列が無い: {calls:?}"));
    &calls[booted + 1..]
}

/// 起こした回の `id` の GET の Reference。
fn refs_of(calls: &[RecordedCall], id: &str) -> Option<Vec<String>> {
    calls.iter().find_map(|call| match call {
        RecordedCall::Get {
            id: got,
            references,
        } if got == id => Some(references.clone()),
        _ => None,
    })
}

/// `name` の記録。
fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 今の切替の予約の切替先のフォルダ名。
fn reserved_for(rig: &SwitchRig) -> Option<String> {
    rig.world
        .get_non_send::<SwitchInFlight>()
        .map(|f| f.target.folder.clone())
}

/// 今のゴーストのフォルダ名。
fn current_of(rig: &SwitchRig) -> Option<String> {
    rig.world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone())
}

/// 読むことだけを共有して開いたまま持つ（起動中の `shiori.dll` と同じ状態＝`areka-nar` のテストの
/// 助手と同じ形）。共有を 0 にすると組み上げの段の複写が先に落ちて確定まで届かない。
fn hold(path: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1 /* FILE_SHARE_READ */)
        .open(path)
        .unwrap_or_else(|err| panic!("{} を掴めるはず: {err}", path.display()))
}

/// 別の切替の最中に預かった書庫は「預かった」のまま置き（切替を重ねない）、その予約が下りた tick に
/// 頼み直して一周する（判定 `Busy`）。頼み直しは同じフォルダの名指し・知らせなし・出どころ「自動」。
/// 一周の途中に届いた切替要求は無視される（要件 7.7）。別の切替の予約は UI の側だけに置く（kanade へは
/// 届かず、定常到達を伴わずに下りる＝中止された切替と同じ形）。
#[test]
fn a_busy_overwrite_retries_on_the_tick_the_reservation_clears_and_runs_through() {
    let mut rig = running("A", &["B"]);
    let dir = TempPath::new("desk-overwrite-busy");
    let archive = NarArchive::open(&write_archive(&dir, "A", &[])).expect("書庫を開ける");
    rig.world.insert_non_send(SwitchInFlight {
        target: SwitchTarget {
            dir: rig.root.ghost_dir("B"),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: rig.root.ghost_dir("A"),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
        boot_event: None,
    });
    let (reply, answer) = reply_channel();
    let mut got = None;

    let (flow, events) = capture(|| {
        take(&mut rig.world, archive, None, reply);
        rig.world.run_schedule(Input);
        rig.world.run_schedule(Input);
        let waited = (reserved_for(&rig), matches!(answer.try_recv(), Ok(None)));
        rig.world.remove_non_send::<SwitchInFlight>();
        let asked = rig.pump_input_until(|rig| reserved_for(rig).is_some());
        let ours = reserved_for(&rig);
        let midway = request_ghost_switch(
            &mut rig.world,
            SwitchRequest {
                ghost: GhostSpec::Folder("B".to_owned()),
                raise_event: true,
                origin: ChangeOrigin::Manual,
                boot_event: None,
            },
        );
        let finished = rig.pump_talking_until(|_| {
            if got.is_none() {
                got = answer.try_recv().ok().flatten();
            }
            got.is_some()
        });
        (waited, asked, ours, midway, finished)
    });
    let body = std::fs::read(rig.root.ghost_dir("A").join(MARKER)).ok();
    let (a, b) = (boots_of(&rig, "A").len(), boots_of(&rig, "B").len());
    let exited = rig.exit_requested();
    assert!(rig.shutdown());

    assert_eq!(
        flow,
        (
            (Some("B".to_owned()), true),
            true,
            Some("A".to_owned()),
            SwitchVerdict::Busy,
            true
        ),
        "(予約の間は待つ・下りた tick に頼み直す・頼んだ切替先・途中の要求・一周した): {events:?}"
    );
    assert!(
        matches!(got, Some(Overwritten::Ran(Ok(_)))),
        "展開した結果が返る"
    );
    assert_eq!(
        body.as_deref(),
        Some(MARKER_BODY),
        "降ろした後に中身が替わる"
    );
    assert_eq!((a, b, exited), (2, 0, false), "A を 1 回起こし直すだけ");
    let requested = named(&events, "ghost_switch_requested");
    assert_eq!(
        requested.len(),
        1,
        "切替を頼むのは頼み直しの 1 回: {events:?}"
    );
    assert_eq!(requested[0].field_str("origin"), Some("automatic"));
    assert_eq!(requested[0].field("raise_event"), Some("false"));
    assert_eq!(requested[0].field("to"), Some("A"));
    assert_eq!(
        named(&events, "ghost_switch_busy").len(),
        2,
        "預かった時と一周の途中の要求: {events:?}"
    );
    let done = named(&events, "install_overwrite_done");
    assert_eq!(done.len(), 1, "{events:?}");
    assert_eq!(done[0].field("ok"), Some("true"));
}

/// 利用者が頼んだ別のゴーストへの切替の最中に預かった書庫は、その切替では展開せず（`Busy`）、切替先が
/// 定常に入った時点で宛先がもう起動中のゴーストでないので背景のスレッドへ返す（よそへの展開へ進む）。
#[test]
fn a_switch_to_another_ghost_does_not_install_and_returns_the_archive() {
    let mut rig = running("A", &["B"]);
    let dir = TempPath::new("desk-overwrite-another");
    let archive = NarArchive::open(&write_archive(&dir, "A", &[])).expect("書庫を開ける");
    let (reply, answer) = reply_channel();
    let mut got = None;

    let (flow, events) = capture(|| {
        let verdict = request_ghost_switch(
            &mut rig.world,
            SwitchRequest {
                ghost: GhostSpec::Folder("B".to_owned()),
                raise_event: false,
                origin: ChangeOrigin::Manual,
                boot_event: None,
            },
        );
        take(&mut rig.world, archive, None, reply);
        let finished = rig.pump_talking_until(|_| {
            if got.is_none() {
                got = answer.try_recv().ok().flatten();
            }
            got.is_some()
        });
        (verdict, finished)
    });
    let touched = rig.root.ghost_dir("A").join(MARKER).exists();
    let current = current_of(&rig);
    let (a, b) = (boots_of(&rig, "A").len(), boots_of(&rig, "B").len());
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    match &got {
        Some(Overwritten::NotRunning(back)) => assert_eq!(back.manifest().directory, "A"),
        _ => panic!("書庫が返る: {events:?}"),
    }
    assert!(!touched, "A のフォルダには展開しない");
    assert!(
        named(&events, "install_overwrite_done").is_empty(),
        "{events:?}"
    );
    assert_eq!(named(&events, "ghost_switch_busy").len(), 1, "{events:?}");
    assert_eq!((current.as_deref(), a, b), (Some("B"), 1, 1));
}

/// 起動中のゴーストのフォルダが目録から外れていたら（判定 `NotFound`）、`warn!` を 1 件残して書庫を
/// 背景のスレッドへ返し、手続きはよそへの展開で続いて締めの知らせが 1 つ出る（止まらない）。
#[test]
fn a_running_ghost_missing_from_the_catalog_is_installed_elsewhere_and_closed_once() {
    let mut rig = running("A", &[]);
    std::fs::remove_file(
        rig.root
            .ghost_dir("A")
            .join("ghost")
            .join("master")
            .join("descript.txt"),
    )
    .expect("A を目録から外す");
    let dir = TempPath::new("desk-overwrite-not-found");

    let (finished, events) = capture(|| install_through(&mut rig, write_archive(&dir, "A", &[])));
    let exited = rig.exit_requested();
    let boots = boots_of(&rig, "A");
    let body = std::fs::read(rig.root.ghost_dir("A").join(MARKER)).ok();
    let clue = clues(&rig, "A", &events);
    assert!(rig.shutdown());

    assert!(finished && !exited, "依頼が終わり、終了しない: {events:?}");
    let warned = named(&events, "install_overwrite_unavailable");
    assert_eq!(warned.len(), 1, "{events:?}");
    assert_eq!(warned[0].level, tracing::Level::WARN);
    assert_eq!(warned[0].field("verdict"), Some("NotFound"));
    assert!(
        named(&events, "ghost_switch_requested").is_empty(),
        "{events:?}"
    );
    assert_eq!(boots.len(), 1, "降ろさない: {clue}");
    let closing: Vec<&String> = boots[0]
        .iter()
        .filter(|id| id.starts_with("OnInstallComplete") || *id == "OnInstallFailure")
        .collect();
    assert_eq!(
        closing,
        ["OnInstallCompleteEx", "OnInstallComplete"],
        "{clue}"
    );
    assert_eq!(body.as_deref(), Some(MARKER_BODY), "よそへの展開で入る");
}

/// 宛先の中のファイルを開いたまま入れると確定が失敗して巻き戻り、同じゴーストが元の中身で起き直って、
/// 定常に入ってから `OnInstallFailure`（Reference0＝`extraction`）が出る。`areka-nar` の失敗の記録の
/// 作業フォルダの欄は空でなく、根の `.nar-work` の配下を指す（要件 7.4・11.7）。
#[test]
fn a_failed_commit_boots_the_same_ghost_with_its_old_contents_and_reports_failure() {
    let mut rig = running("A", &[]);
    let held = rig
        .root
        .ghost_dir("A")
        .join("ghost")
        .join("master")
        .join("held.txt");
    std::fs::write(&held, b"old").expect("掴むファイルを置く");
    let handle = hold(&held);
    let dir = TempPath::new("desk-overwrite-failed");

    let (finished, events) = capture(|| install_through(&mut rig, write_archive(&dir, "A", &[])));
    drop(handle);
    let exited = rig.exit_requested();
    let boots = boots_of(&rig, "A");
    let failure = rig
        .calls("A")
        .get(1)
        .and_then(|c| refs_of(c, "OnInstallFailure"));
    let touched = rig.root.ghost_dir("A").join(MARKER).exists();
    let kept = std::fs::read(&held).ok();
    let (current, work_root) = (current_of(&rig), rig.root.dir().join(".nar-work"));
    let clue = clues(&rig, "A", &events);
    assert!(rig.shutdown());

    assert!(finished && !exited, "依頼が終わり、終了しない: {events:?}");
    assert_eq!(boots.len(), 2, "同じゴーストを 1 回起こし直す: {clue}");
    assert_eq!(current.as_deref(), Some("A"));
    assert_eq!(after_boot(&boots[1]), ["OnInstallFailure"], "{clue}");
    assert_eq!(
        failure
            .as_ref()
            .and_then(|refs| refs.first())
            .map(String::as_str),
        Some("extraction"),
        "{failure:?}"
    );
    assert!(!touched, "元の中身のまま（目印は入らない）");
    assert_eq!(kept.as_deref(), Some(&b"old"[..]));
    let done = named(&events, "install_overwrite_done");
    assert_eq!(done.len(), 1, "{events:?}");
    assert_eq!(done[0].field("ok"), Some("false"));
    let nar: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR && e.target.starts_with("areka_nar"))
        .collect();
    assert_eq!(nar.len(), 1, "失敗の記録は 1 件: {events:?}");
    // 失敗したのは確定の段（組み上げの段の複写ではない）で、巻き戻して確定は 0 件。
    let reason = nar[0].field("reason").unwrap_or_default();
    assert!(
        reason.contains("Commit"),
        "確定の段で失敗した: {:?}",
        nar[0]
    );
    assert_eq!(
        (nar[0].field("rolled_back"), nar[0].field("committed")),
        (Some("true"), Some("0")),
        "{:?}",
        nar[0]
    );
    let work = nar[0].field("work").unwrap_or_default();
    assert!(!work.is_empty(), "作業フォルダの欄が空: {:?}", nar[0]);
    assert!(
        Path::new(work).starts_with(&work_root),
        "作業フォルダは根の .nar-work の配下: {work}"
    );
}

/// 入れた後に同じゴーストを起こせなければ（ここでは `refresh,1` で `descript.txt` が消える）、今日の
/// 切替の道のまま既定ゴーストへ戻し（`OnBoot` の Reference6＝`halt`・Reference7＝そのゴーストの名前）、
/// 戻した既定ゴーストが定常に入ってから締めの知らせを送る（要件 7.5）。
#[test]
fn an_overwritten_ghost_that_cannot_boot_falls_back_to_the_default_ghost() {
    let mut rig = running("A", &[DEFAULT_GHOST_FOLDER]);
    let dir = TempPath::new("desk-overwrite-fallback");

    let (finished, events) =
        capture(|| install_through(&mut rig, write_archive(&dir, "A", &["refresh,1"])));
    let exited = rig.exit_requested();
    let current = current_of(&rig);
    let defaults = boots_of(&rig, DEFAULT_GHOST_FOLDER);
    let on_boot = rig
        .calls(DEFAULT_GHOST_FOLDER)
        .first()
        .and_then(|c| refs_of(c, "OnBoot"));
    let clue = clues(&rig, DEFAULT_GHOST_FOLDER, &events);
    assert!(rig.shutdown());

    assert!(finished && !exited, "依頼が終わり、終了しない: {events:?}");
    let failed = named(&events, "ghost_switch_boot_failed");
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(failed[0].field("ghost"), Some("A"));
    assert_eq!(current.as_deref(), Some(DEFAULT_GHOST_FOLDER));
    assert_eq!(defaults.len(), 1, "{clue}");
    let on_boot = on_boot.expect("既定ゴーストの OnBoot");
    assert_eq!(
        (
            on_boot.get(6).map(String::as_str),
            on_boot.get(7).map(String::as_str)
        ),
        (Some("halt"), Some("A")),
        "{on_boot:?}"
    );
    assert_eq!(
        after_boot(&defaults[0]),
        ["OnInstallCompleteEx", "OnInstallComplete"],
        "{clue}"
    );
}

/// 起こせないのが既定ゴースト自身なら、今日の致命の経路で終わる（終了の指示・最初の出所は「既定
/// ゴーストへ戻せなかった」）。印の判定は今日の引数のまま、理由の語も今日の `switch_fatal`（要件 7.6・7.9）。
#[test]
fn an_overwritten_default_ghost_that_cannot_boot_is_fatal() {
    let mut rig = running(DEFAULT_GHOST_FOLDER, &[]);
    let dir = TempPath::new("desk-overwrite-fatal");

    let (finished, events) = capture(|| {
        install_through(
            &mut rig,
            write_archive(&dir, DEFAULT_GHOST_FOLDER, &["refresh,1"]),
        )
    });
    let first = rig.world.get_resource::<FirstExit>().map(|f| f.0.clone());
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    assert!(rig.shutdown());

    assert!(finished && rig.exit_requested(), "終了する: {events:?}");
    assert!(
        matches!(first, Some(ExitOrigin::GhostFallbackFailed(_))),
        "最初の出所: {first:?}"
    );
    assert!(!reserved, "予約は残らない");
    assert_eq!(named(&events, "ghost_switch_fatal").len(), 1, "{events:?}");
    assert_eq!(
        named(&events, "install_overwrite_done").len(),
        1,
        "{events:?}"
    );
    let end = crate::Teardown {
        run_ok: true,
        down_ok: true,
        shiori_cut: false,
    };
    assert_eq!(
        crate::session_mark_verdict(first.as_ref(), false, false, end),
        crate::MarkVerdict::Keep("switch_fatal")
    );
}
