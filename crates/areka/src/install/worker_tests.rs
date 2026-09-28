//! 背景のスレッドの口の決定論テスト（design「Testing Strategy / 背景スレッドの口」・要件 1.11・8.4）。
//!
//! 窓口の代わりに素の受信端で頼みを受けて答える。確かめること: kanade の結果 5 値と返信端の
//! 切断の写し（「定常でない」は送り直しの印つきで 2 回目の頼みが来る）・窓口が居ないときの
//! 閉じた扱い・門が閉じていれば宛先に 1 バイトも書かずに「入らなかった」・スレッドが依頼を
//! 1 件走らせて終わりを知らせる。
//!
//! 実時間の待ちに依らない: 口は返信端の受け取りで、窓口の代わりは受信端の受け取りで揃える。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use areka_kanade::RaiseOutcome;
use areka_nar::NarArchive;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;

use super::*;
use crate::exit_wait::{begin_close, register_gate};
use crate::install::InstallOrigin;

// ---------------------------------------------------------------- 道具立て

const EVENT: &str = "OnInstallBegin";

fn refs() -> Vec<String> {
    vec!["a".to_owned(), "b".to_owned()]
}

/// 本物の口と、窓口の代わりの受信端と、門。
fn ports() -> (DeskPorts, Receiver<DeskAsk>, Arc<WorkGate>) {
    let (desk, asks) = mpsc::channel();
    let gate = Arc::new(WorkGate::default());
    (DeskPorts::new(desk, gate.clone()), asks, gate)
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 窓口の代わりに、イベントの頼みを `answers` の数だけ受けて順に答え（None は返信端を落とす）、
/// 頼みの送り直しの印を順に返す。答えの数より多く頼みが来たら赤。
fn raise_with(answers: &[Option<RaiseOutcome>]) -> (Raised, Vec<bool>, Vec<CapturedEvent>) {
    let (mut ports, asks, _gate) = ports();
    let answers = answers.to_vec();
    let desk = thread::spawn(move || {
        let mut resends = Vec::new();
        for answer in answers {
            let Ok(DeskAsk::Raise {
                id,
                references,
                resend,
                reply,
            }) = asks.recv()
            else {
                panic!("イベントの頼みが届く");
            };
            assert_eq!(id, EVENT);
            assert_eq!(references, refs());
            resends.push(resend);
            if let Some(outcome) = answer {
                let _ = reply.send(outcome);
            }
        }
        assert!(asks.recv().is_err(), "頼みは答えの数だけ");
        resends
    });
    let (raised, events) = capture(|| ports.raise(EVENT, refs()));
    drop(ports);
    let resends = desk.join().expect("窓口の代わりが最後まで走る");
    (raised, resends, events)
}

/// 素性（根だけが意味を持つ）。
fn facts_at(root: &Path) -> GhostFacts {
    GhostFacts {
        root: root.to_path_buf(),
        folder: Some("running".to_owned()),
        name: "Running".to_owned(),
        sakura_name: None,
        install_accept: Vec::new(),
    }
}

/// ゴースト `newbie` の書庫を `dir` へ置いて開く。
fn ghost_archive(dir: &TempPath) -> (PathBuf, NarArchive) {
    let path = dir.child("newbie.nar");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,あたらしい",
                "directory,newbie",
            ]),
        )
        .done()
        .file(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\nname,Newbie\r\n",
        )
        .done()
        .write_to(&path)
        .expect("書庫を置ける");
    let archive = NarArchive::open(&path).expect("書庫を開ける");
    (path, archive)
}

/// 窓口の代わりに素性の頼みを 1 件受けて答えた後の口（根を覚えている）。
fn ports_after_facts(root: &Path) -> (DeskPorts, Receiver<DeskAsk>, Arc<WorkGate>) {
    let (mut ports, asks, gate) = ports();
    let facts = facts_at(root);
    // 口が倒れたら `ports` と一緒に送出端が落ち、窓口の代わりも受け取りで倒れて終わる
    // （scope で包むと、倒れた口が送出端を握ったまま窓口の代わりを待って止まる）。
    let desk = thread::spawn(move || {
        let Ok(DeskAsk::Facts { reply }) = asks.recv() else {
            panic!("素性の頼みが届く");
        };
        let _ = reply.send(Some(facts));
        asks
    });
    assert_eq!(ports.ghost_facts(), Some(facts_at(root)));
    let asks = desk.join().expect("窓口の代わりが最後まで走る");
    (ports, asks, gate)
}

/// 根の下にある物の数（入れていなければ 0）。
fn entries_under(root: &Path) -> usize {
    std::fs::read_dir(root).expect("根を読める").count()
}

// ---------------------------------------------------------------- イベントの写し

/// 台本が返れば `Script`。頼みは 1 回で送り直しの印は無い。
#[test]
fn script_is_script() {
    let (raised, resends, events) = raise_with(&[Some(RaiseOutcome::Script)]);
    assert_eq!(raised, Raised::Script);
    assert_eq!(resends, [false]);
    assert!(
        events.iter().all(|e| e.level != tracing::Level::ERROR),
        "{events:?}"
    );
}

/// 返事なしは `NoReply`。
#[test]
fn no_reply_is_no_reply() {
    let (raised, resends, events) = raise_with(&[Some(RaiseOutcome::NoReply)]);
    assert_eq!(raised, Raised::NoReply);
    assert_eq!(resends, [false]);
    assert!(
        events.iter().all(|e| e.level != tracing::Level::ERROR),
        "{events:?}"
    );
}

/// 許可表に無い名前は `error!` を 1 件残して `NoReply`（手続きは続ける）。
#[test]
fn not_allowed_is_logged_and_read_as_no_reply() {
    let (raised, resends, events) = raise_with(&[Some(RaiseOutcome::NotAllowed)]);
    assert_eq!(raised, Raised::NoReply);
    assert_eq!(resends, [false]);
    let logged = events_named(&events, "install_event_not_allowed");
    assert_eq!(logged.len(), 1, "{events:?}");
    assert_eq!(logged[0].level, tracing::Level::ERROR);
    assert_eq!(logged[0].field_str("id"), Some(EVENT));
}

/// 往復の失敗は `error!(install_event_failed)` を 1 件残して `Closed`（以後は送らない）。
#[test]
fn failed_is_logged_and_read_as_closed() {
    let (raised, resends, events) = raise_with(&[Some(RaiseOutcome::Failed)]);
    assert_eq!(raised, Raised::Closed);
    assert_eq!(resends, [false]);
    let logged = events_named(&events, "install_event_failed");
    assert_eq!(logged.len(), 1, "{events:?}");
    assert_eq!(logged[0].level, tracing::Level::ERROR);
    assert_eq!(logged[0].field_str("id"), Some(EVENT));
}

/// 定常でなければ、送り直しの印を付けて頼み直す。2 回目の答えが口の答えになる。
#[test]
fn not_steady_asks_again_with_resend_mark() {
    let (raised, resends, _) =
        raise_with(&[Some(RaiseOutcome::NotSteady), Some(RaiseOutcome::Script)]);
    assert_eq!(raised, Raised::Script);
    assert_eq!(resends, [false, true]);
}

/// 返信端が答えずに落ちたら `Closed`（終了が始まった・kanade が止まった）。
#[test]
fn dropped_reply_is_closed() {
    let (raised, resends, _) = raise_with(&[None]);
    assert_eq!(raised, Raised::Closed);
    assert_eq!(resends, [false]);
}

/// 窓口が居なければ、どの口も閉じた扱い（素性は None・上書きは Closed・記録は戻る）。
#[test]
fn every_port_reads_a_missing_desk_as_closed() {
    let (mut ports, asks, _gate) = ports();
    drop(asks);
    assert_eq!(ports.raise(EVENT, refs()), Raised::Closed);
    assert_eq!(ports.ghost_facts(), None);
    let dir = TempPath::new("install-worker-archive");
    let (_, archive) = ghost_archive(&dir);
    assert!(matches!(
        ports.overwrite_running(archive, None),
        Overwritten::Closed
    ));
    ports.record(InstalledRecord {
        kind: areka_nar::InstallKind::Ghost,
        object_name: "あたらしい".to_owned(),
        folder: "newbie".to_owned(),
        ghost_folder: Some("newbie".to_owned()),
    });
}

// ---------------------------------------------------------------- 門

/// 門が閉じていれば、宛先に 1 バイトも書かずに None。窓口へも頼まない（要件 8.4）。
#[test]
fn closed_gate_installs_nothing() {
    let root = TempPath::new("install-worker-root");
    let dir = TempPath::new("install-worker-archive");
    let (path, archive) = ghost_archive(&dir);
    let (mut ports, asks, gate) = ports_after_facts(root.path());
    ports.begin_archive(&path);
    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let _ = begin_close(&mut world);

    assert!(ports.install_elsewhere(&archive, None).is_none());
    assert_eq!(entries_under(root.path()), 0, "根の下に何も置かない");
    assert!(asks.try_recv().is_err(), "窓口へ頼まない");
}

/// 対照: 門が開いていれば入れる。書庫を扱い終えたら門を手放している（後で終了が始まっても
/// 途中でやめた書庫として記録されない）。
#[test]
fn open_gate_installs_and_releases_the_gate() {
    let root = TempPath::new("install-worker-root");
    let dir = TempPath::new("install-worker-archive");
    let (path, archive) = ghost_archive(&dir);
    let (mut ports, _asks, gate) = ports_after_facts(root.path());
    ports.begin_archive(&path);

    let outcome = ports
        .install_elsewhere(&archive, None)
        .expect("門が開いていれば入る")
        .expect("入れられる");
    assert_eq!(outcome.name, "あたらしい");
    assert!(root.path().join("ghost").join("newbie").is_dir());

    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let (_, events) = capture(|| begin_close(&mut world));
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "{events:?}"
    );
}

/// 書庫を始めた後・展開の前（利用条件の画面・読み取りと検査の段）に終了が始まったら、
/// 途中でやめた書庫として書庫のパスが 1 件記録される（要件 8.4）。
#[test]
fn exit_before_writing_records_the_archive() {
    let dir = TempPath::new("install-worker-archive");
    let path = dir.child("newbie.nar");
    let (mut ports, _asks, gate) = ports();
    ports.begin_archive(&path);

    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let (_, events) = capture(|| begin_close(&mut world));
    let abandoned = events_named(&events, "exit_wait_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(
        abandoned[0].field_str("label"),
        Some(path.display().to_string().as_str())
    );
}

/// 門の記録の名前には、書庫のパスと宛先の両方が載る（要件 8.3）。
#[test]
fn gate_label_names_archive_and_destination() {
    let root = TempPath::new("install-worker-root");
    let dir = TempPath::new("install-worker-archive");
    let (path, archive) = ghost_archive(&dir);
    let label = gate_label(&path, root.path(), &archive, Some("target"));
    assert!(label.contains(&path.display().to_string()), "{label}");
    assert!(
        label.contains(&root.path().display().to_string()),
        "{label}"
    );
    assert!(label.contains("newbie"), "{label}");
    assert!(label.contains("target"), "{label}");
}

// ---------------------------------------------------------------- スレッド

/// スレッドは依頼を 1 件走らせ、窓口へ終わりを知らせる。依頼の送出端が落ちたら終わる。
#[test]
fn worker_runs_an_order_and_reports_done() {
    let (desk, asks) = mpsc::channel();
    let gate = Arc::new(WorkGate::default());
    let (orders, handle) = spawn_worker(desk, gate.clone());
    let dir = TempPath::new("install-worker-order");
    orders
        .send(InstallOrder {
            archives: vec![dir.child("missing.nar")],
            origin: InstallOrigin::Menu,
        })
        .expect("スレッドが受け取る");

    // 読めない書庫: 始まりの知らせ → 失敗の知らせ → 依頼の終わり。
    let mut raised = Vec::new();
    loop {
        match asks.recv().expect("頼みが届く") {
            DeskAsk::Raise { id, reply, .. } => {
                raised.push(id);
                let _ = reply.send(RaiseOutcome::NoReply);
            }
            DeskAsk::OrderDone => break,
            _ => panic!("読めない書庫ではイベントの頼みと終わりだけが届く"),
        }
    }
    assert_eq!(raised, ["OnInstallBegin", "OnInstallFailure"]);

    // 依頼を終えた門には何も残らない（失敗した書庫を途中でやめた書庫として記録しない）。
    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let (_, events) = capture(|| begin_close(&mut world));
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "{events:?}"
    );

    drop(orders);
    handle.join().expect("スレッドが終わる");
}

/// 本番の結線: 手続きは書庫の始め（`OnInstallBegin` を送る前）に門へ書庫のパスを置く。
/// 始まりの知らせの返事を待っている間に終了が始まれば、途中でやめた書庫として 1 件記録される。
/// その後に返信端を落とすと、スレッドは閉じた扱いで抜けて依頼の終わりを知らせる。
#[test]
fn worker_names_the_archive_on_the_gate_before_the_first_event() {
    let (desk, asks) = mpsc::channel();
    let gate = Arc::new(WorkGate::default());
    let (orders, handle) = spawn_worker(desk, gate.clone());
    let dir = TempPath::new("install-worker-order");
    let path = dir.child("waiting.nar");
    orders
        .send(InstallOrder {
            archives: vec![path.clone()],
            origin: InstallOrigin::Script,
        })
        .expect("スレッドが受け取る");

    // 始まりの知らせの頼みを受け取るが、まだ答えない（スレッドは返事待ちで止まっている）。
    let DeskAsk::Raise { id, reply, .. } = asks.recv().expect("頼みが届く") else {
        panic!("最初の頼みはイベント");
    };
    assert_eq!(id, "OnInstallBegin");

    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let (_, events) = capture(|| begin_close(&mut world));
    let abandoned = events_named(&events, "exit_wait_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(
        abandoned[0].field_str("label"),
        Some(path.display().to_string().as_str())
    );

    drop(reply);
    assert!(
        matches!(asks.recv(), Ok(DeskAsk::OrderDone)),
        "閉じた扱いで抜けて依頼の終わりが届く"
    );
    drop(orders);
    handle.join().expect("スレッドが終わる");
}

/// 起動中のゴーストへ入れ終えたら門を手放す。完了の知らせを待つ間に終了が始まっても、
/// 書く前にやめた書庫として記録されない。
#[test]
fn overwrite_ran_releases_the_gate() {
    let root = TempPath::new("install-worker-root");
    let dir = TempPath::new("install-worker-archive");
    let (path, archive) = ghost_archive(&dir);
    let (mut ports, asks, gate) = ports();
    ports.begin_archive(&path);

    let desk_root = root.path().to_path_buf();
    let desk = thread::spawn(move || {
        let Ok(DeskAsk::Overwrite {
            archive,
            target_ghost,
            reply,
        }) = asks.recv()
        else {
            panic!("上書きの頼みが届く");
        };
        let result = archive.install(&areka_nar::InstallRequest {
            root: &desk_root,
            target_ghost: target_ghost.as_deref(),
        });
        let _ = reply.send(Overwritten::Ran(result));
    });
    assert!(matches!(
        ports.overwrite_running(archive, None),
        Overwritten::Ran(Ok(_))
    ));
    desk.join().expect("窓口の代わりが最後まで走る");

    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let (_, events) = capture(|| begin_close(&mut world));
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "{events:?}"
    );
}
