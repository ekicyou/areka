//! 取得の待ち行列の決定論テスト。
//!
//! 窓口の取得の道具に偽の作り口を差し、外のホストへはつながない。「取得の途中」は作り口を
//! `mpsc` の受信端で止めて作り、時刻の長さには頼らない。依頼は待ち行列に残す（背景のスレッドは
//! 起こさない）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use areka_kanade::online::OnlineCounter;
use areka_update::{Fetch, FetchError};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::*;
use crate::install::fetch_url::MakeFetch;
use crate::install::fetch_url_test_support::FakeFetch;
use crate::install::{queued_orders, register};
use crate::session_tests::events_named;
use areka_runtime::spine::{Progress, SPIN_WAIT, wait_until};
use areka_session::exit_wait::{begin_close, register_gate};

const A: &str = "https://example.com/files/a.nar";
const B: &str = "https://example.com/files/b.nar";
const C: &str = "https://example.com/files/c.nar";

static ONLINE: OnlineCounter = OnlineCounter::new();

fn served() -> FakeFetch {
    FakeFetch::new()
        .serve(A, b"a")
        .serve(B, b"b")
        .serve(C, b"c")
}

/// 呼ばれると `entered` へ知らせ、`release` が来るまで止まる作り口の操り手。`peak` は同時に
/// 作り口の中に居た数の最大。`release` を落とすと止まっている作り口は失敗で抜ける。
struct Held {
    entered: Receiver<()>,
    release: Sender<()>,
    peak: Arc<AtomicUsize>,
}

fn held_make() -> (MakeFetch, Held) {
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);
    let live = AtomicUsize::new(0);
    let peak = Arc::new(AtomicUsize::new(0));
    let make: MakeFetch = {
        let peak = peak.clone();
        Arc::new(move || {
            let now = live.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            entered_tx.send(()).expect("テストが知らせを待っている");
            let released = release_rx
                .lock()
                .expect("作り口は同時に 1 つ")
                .recv_timeout(SPIN_WAIT);
            live.fetch_sub(1, Ordering::SeqCst);
            released.map_err(|_| FetchError::Timeout)?;
            Ok(Box::new(served()) as Box<dyn Fetch>)
        })
    };
    (
        make,
        Held {
            entered,
            release,
            peak,
        },
    )
}

/// 窓口を据え、取得の道具を差し替えた World。依頼は待ち行列に残す。
fn desk_world(make: MakeFetch, dir: PathBuf) -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    {
        let mut desk = world.non_send_mut::<InstallDesk>();
        desk.fetch_tools = FetchTools {
            dir,
            make,
            online: &ONLINE,
        };
        desk.busy = true;
    }
    world
}

fn send_url(world: &World, url: &str) {
    raw_sender(world)
        .send(RawInstallRequest::Url(url.to_owned()))
        .expect("窓口の生の要求の受信端は生きている");
}

/// 待ち行列に `count` 件の依頼が積まれるまで取り出しを回す。
fn drain_until_orders(world: &mut World, count: usize) {
    wait_until(
        "取得した書庫が依頼になる",
        Progress::Unknown,
        || {
            drain(world);
            queued_orders(world).len() == count
        },
    )
    .expect("依頼が積まれる");
}

/// 積まれた依頼の書庫の名前の末尾（`<pid>-<連番>-` を除いた部分）。
fn order_tails(world: &World) -> Vec<String> {
    queued_orders(world)
        .iter()
        .flat_map(|order| order.archives.iter())
        .map(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.rsplit('-').next().unwrap_or_default().to_owned()
        })
        .collect()
}

/// 取得の途中で終了が始まると、取得の門の名として URL が `exit_wait_abandoned` に 1 件残る。
#[test]
fn closing_mid_fetch_records_the_url_on_the_gate() {
    let tmp = TempPath::new("fetch-queue-close");
    let (make, held) = held_make();
    let mut world = desk_world(make, tmp.child("download"));
    send_url(&world, A);
    drain(&mut world);
    held.entered.recv_timeout(SPIN_WAIT).expect("取得が始まる");

    let (_, events) = capture(|| begin_close(&mut world));

    let abandoned = events_named(&events, "exit_wait_abandoned");
    let [record] = abandoned.as_slice() else {
        panic!("記録は 1 件: {events:?}");
    };
    assert_eq!(record.field_str("name"), Some("install-fetch"));
    assert_eq!(record.field_str("label"), Some(A));
}

/// 取得の最中に 2 本目を投げても作り口は同時に 1 つで、1 本目を終えてから 2 本目を取りに行き、
/// 2 本とも届いた順に依頼になる。
#[test]
fn second_url_waits_for_the_first_and_both_become_orders_in_order() {
    let tmp = TempPath::new("fetch-queue-order");
    let (make, held) = held_make();
    let mut world = desk_world(make, tmp.child("download"));
    send_url(&world, A);
    drain(&mut world);
    held.entered
        .recv_timeout(SPIN_WAIT)
        .expect("1 本目が始まる");
    send_url(&world, B);
    drain(&mut world);
    drain(&mut world);
    assert!(
        held.entered.try_recv().is_err(),
        "1 本目の最中に 2 本目の作り口は呼ばれない"
    );

    held.release.send(()).expect("1 本目を放す");
    held.release.send(()).expect("2 本目を放す");
    drain_until_orders(&mut world, 2);

    assert_eq!(held.peak.load(Ordering::SeqCst), 1, "作り口は同時に 1 つ");
    assert_eq!(order_tails(&world), ["a.nar", "b.nar"]);
}

/// 終了では、取得を待っている URL が `install_fetch_abandoned` で 1 件ずつ記録され、取得の
/// スレッドへは渡らない。
#[test]
fn exit_records_each_waiting_url_once() {
    let tmp = TempPath::new("fetch-queue-exit");
    let (make, held) = held_make();
    let mut world = desk_world(make, tmp.child("download"));
    send_url(&world, A);
    drain(&mut world);
    held.entered
        .recv_timeout(SPIN_WAIT)
        .expect("1 本目が始まる");
    send_url(&world, B);
    send_url(&world, C);
    drain(&mut world);

    let (_, events) = capture(|| begin_close(&mut world));
    drop(held.release);
    drain(&mut world);

    let urls: Vec<Option<&str>> = events_named(&events, "install_fetch_abandoned")
        .iter()
        .map(|e| e.field_str("url"))
        .collect();
    assert_eq!(urls, [Some(B), Some(C)]);
    let desk = world.non_send::<InstallDesk>();
    assert!(desk.fetch_queue.is_empty());
    assert!(desk.fetch_tx.is_none(), "取得のスレッドへの送出端を落とす");
    assert!(
        held.entered.try_recv().is_err(),
        "積んだ残りは取りに行かない"
    );
}

/// 取得の口が Err を返しても panic しても、次の URL が取りに行かれて依頼になる。
#[test]
fn next_url_is_fetched_after_an_error_or_a_panic() {
    for panics in [false, true] {
        let tmp = TempPath::new("fetch-queue-next");
        let calls = Arc::new(AtomicUsize::new(0));
        let make: MakeFetch = {
            let calls = calls.clone();
            Arc::new(move || {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    assert!(!panics, "取得の口が倒れる");
                    return Err(FetchError::Timeout);
                }
                Ok(Box::new(served()) as Box<dyn Fetch>)
            })
        };
        let mut world = desk_world(make, tmp.child("download"));
        send_url(&world, A);
        send_url(&world, B);

        drain_until_orders(&mut world, 1);

        assert_eq!(order_tails(&world), ["b.nar"], "panics = {panics}");
        assert_eq!(calls.load(Ordering::SeqCst), 2, "panics = {panics}");
        wait_until("2 本目の取得の終わり", Progress::Unknown, || {
            drain(&mut world);
            !world.non_send::<InstallDesk>().fetching
        })
        .expect("取得の終わりが届く");
    }
}

/// 取得のスレッドが URL を受けた時点で門が閉じていれば、取りに行かずに `install_fetch_closing` を
/// 1 件残し、取得の終わりは知らせてスレッドは抜ける。
#[test]
fn closed_gate_skips_the_fetch_and_still_reports_done() {
    let tmp = TempPath::new("fetch-queue-closing");
    let gate = Arc::new(WorkGate::default());
    let mut world = World::new();
    register_gate(&mut world, "install-fetch", gate.clone(), |_| {});
    let _ = begin_close(&mut world);
    let calls = Arc::new(AtomicUsize::new(0));
    let make: MakeFetch = {
        let calls = calls.clone();
        Arc::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(served()) as Box<dyn Fetch>)
        })
    };
    let tools = FetchTools {
        dir: tmp.child("download"),
        make,
        online: &ONLINE,
    };
    let (raw, raw_rx) = mpsc::channel();
    let (desk, desk_rx) = mpsc::channel();

    let ((urls, done), events) = capture(|| {
        let urls = fetch_url::spawn_fetcher(A.to_owned(), tools, gate, raw, desk);
        (urls, desk_rx.recv_timeout(SPIN_WAIT))
    });

    assert!(urls.is_some(), "install-fetch が起きる");
    assert!(
        matches!(done, Ok(DeskAsk::FetchDone { panicked: false })),
        "取得の終わりが届く"
    );
    let closing = events_named(&events, "install_fetch_closing");
    let [record] = closing.as_slice() else {
        panic!("記録は 1 件: {events:?}");
    };
    assert_eq!(record.field_str("url"), Some(A));
    assert_eq!(calls.load(Ordering::SeqCst), 0, "作り口は呼ばれない");
    assert_eq!(
        raw_rx.recv_timeout(SPIN_WAIT).err(),
        Some(mpsc::RecvTimeoutError::Disconnected),
        "依頼は 0 件でスレッドは抜ける"
    );
}
