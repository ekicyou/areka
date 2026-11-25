//! 背景のスレッドが倒れたときの決定論テスト。
//!
//! 守り手を持たせた素のスレッドを panic させ、窓口への知らせと門の後始末を見る。窓口の側は
//! 知らせを頼みの送出端へ直接入れ、`drain` を直接呼ぶ（フレームは回さない）。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::capture;

use super::*;
use crate::install::worker::OrderGuard;
use crate::install::{InstallOrigin, register};
use crate::session_tests::events_named;
use areka_actor::reply_channel;
use areka_runtime::spine::join_thread_bounded;
use areka_session::exit_wait::{begin_close, register_gate};
use areka_session::ghost_session::{GhostSession, GhostSlot};

fn order() -> InstallOrder {
    InstallOrder {
        archives: vec![PathBuf::from("C:/missing.nar")],
        origin: InstallOrigin::Menu,
    }
}

fn desk_world() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    world
}

/// 守り手を持ったスレッドが panic すると、窓口へ倒れの知らせが届き、門には名が残らない。
#[test]
fn panicking_thread_reports_and_clears_the_gate() {
    let (desk, asks) = mpsc::channel();
    let gate = Arc::new(WorkGate::default());
    let thread_gate = gate.clone();
    let handle = thread::spawn(move || {
        let _guard = OrderGuard {
            gate: thread_gate.clone(),
            desk,
        };
        assert!(thread_gate.begin("C:/panicked.nar".to_owned()));
        panic!("背景のスレッドが倒れる");
    });
    assert!(join_thread_bounded("倒れるスレッドの終わり", None, handle).is_err());

    assert!(matches!(asks.try_recv(), Ok(DeskAsk::WorkerPanicked)));

    let mut world = World::new();
    register_gate(&mut world, "install", gate, |_| {});
    let (_, events) = capture(|| begin_close(&mut world));
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "{events:?}"
    );
}

/// 倒れずに抜けた守り手は何も知らせない。
#[test]
fn guard_without_panic_sends_nothing() {
    let (desk, asks) = mpsc::channel();
    drop(OrderGuard {
        gate: Arc::new(WorkGate::default()),
        desk,
    });
    assert!(asks.try_recv().is_err());
}

/// 倒れの知らせを受けた窓口は忙しくなくなり、次の依頼で背景のスレッドを起こし直す。
#[test]
fn desk_respawns_the_worker_after_a_panic() {
    let mut world = desk_world();
    let (worker, _orders) = mpsc::channel();
    let asks = {
        let mut desk = world.non_send_mut::<InstallDesk>();
        desk.worker = Some(worker);
        desk.busy = true;
        desk.asks_tx.clone()
    };
    asks.send(DeskAsk::WorkerPanicked).unwrap();
    drain(&mut world);
    assert!(!is_busy(&world));
    assert!(world.non_send::<InstallDesk>().worker.is_none());

    world.non_send_mut::<InstallDesk>().queue.push_back(order());
    let (_, events) = capture(|| drain(&mut world));
    assert!(
        !events_named(&events, "install_worker_spawned").is_empty(),
        "{events:?}"
    );
    let desk = world.non_send::<InstallDesk>();
    assert!(desk.worker.is_some());
    assert!(desk.busy);
}

/// 終了が始まった後に倒れても、窓口は忙しさと送出端を戻す。
#[test]
fn desk_takes_the_panic_after_closing() {
    let mut world = desk_world();
    let (worker, _orders) = mpsc::channel();
    let asks = {
        let mut desk = world.non_send_mut::<InstallDesk>();
        desk.worker = Some(worker);
        desk.busy = true;
        desk.asks_tx.clone()
    };
    let _ = begin_close(&mut world);
    asks.send(DeskAsk::WorkerPanicked).unwrap();
    drain(&mut world);
    assert!(!is_busy(&world));
    assert!(world.non_send::<InstallDesk>().worker.is_none());
}

/// 背景のスレッドが居ないと分かった窓口は、送出端を捨てて次の依頼で起こし直せるようにする。
#[test]
fn desk_forgets_a_gone_worker() {
    let mut world = desk_world();
    let (worker, orders) = mpsc::channel();
    drop(orders);
    world.non_send_mut::<InstallDesk>().worker = Some(worker);
    world.non_send_mut::<InstallDesk>().queue.push_back(order());
    drain(&mut world);
    assert!(world.non_send::<InstallDesk>().worker.is_none());
}

/// kanade の受信端が落ちていれば、イベントの頼みの返信端を落として警告を 1 件残す。
#[test]
fn raise_to_a_stopped_kanade_drops_the_reply_and_warns_once() {
    let mut world = desk_world();
    let (kanade, stopped) = mpsc::channel();
    drop(stopped);
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(kanade),
        "ghost/A".into(),
    ))));
    let (reply, answer) = reply_channel();
    world
        .non_send::<InstallDesk>()
        .asks_tx
        .send(DeskAsk::Raise {
            id: "OnInstallBegin",
            references: Vec::new(),
            resend: false,
            reply,
        })
        .expect("窓口の頼みの受信端は生きている");
    let (_, events) = capture(|| drain(&mut world));
    assert!(answer.try_recv().is_err());
    let failed: Vec<_> = events_named(&events, "install_event_send_failed");
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(failed[0].level, tracing::Level::WARN);
}
