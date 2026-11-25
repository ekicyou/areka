//! 背景スレッドが依頼の最中に倒れたときの決定論テスト。
//!
//! 偽の取得口の `get` を panic させて本物のスレッド `update` を倒し、窓口への知らせと門の後始末を見る。
//! 窓口の側は知らせを頼みの送出端へ直接入れ、`drain` を直接呼ぶ（フレームは回さない）。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use areka_ghost::BasewareRoots;
use areka_kanade::online::OnlineCounter;
use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{KanadeMsg, RaiseOutcome, WaitBudget};
use areka_runtime::spine::{Progress, join_bounded, join_thread_bounded, wait_recv};
use areka_update::{Fetch, FetchError};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::*;
use crate::session_tests::events_named;
use crate::update::{SummaryKind, TargetKind, TargetSpec, UpdateOrder, UpdateReason, register};
use areka_session::exit_wait::{begin_close, register_gate};

struct PanickingGet;

impl Fetch for PanickingGet {
    fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
        panic!("取得の最中に倒れる");
    }
}

fn balloon_order(dir: PathBuf) -> UpdateOrder {
    UpdateOrder {
        targets: vec![TargetSpec {
            kind: TargetKind::Balloon,
            dir,
            name: "B".to_owned(),
            descript_homeurl: Some("https://example.invalid/b1/".to_owned()),
        }],
        reason: UpdateReason::Script,
        summary: SummaryKind::Result,
        ghost_dir: PathBuf::from(r"C:\root\ghost\emo2"),
        ghost_folder: Some("emo2".to_owned()),
    }
}

/// kanade の代わり: 照会には「無い」、イベントには「返事なし」と答え、送出端が全部落ちたら抜ける。
fn answer_kanade(rx: mpsc::Receiver<KanadeMsg>) {
    for msg in rx {
        match msg {
            KanadeMsg::ResourceQuery { ids, reply } => {
                let _ = reply.send(
                    ids.into_iter()
                        .map(|id| (id, ResourceOutcome::NoContent))
                        .collect(),
                );
            }
            KanadeMsg::RaiseEvent {
                reply: Some(reply), ..
            } => {
                let _ = reply.send(RaiseOutcome::NoReply);
            }
            _ => {}
        }
    }
}

/// 取得の最中に倒れたスレッドは、「終わった」でなく倒れの知らせを窓口へ送り、門を書く段に残さない。
/// 知らせの時点で通信中の数は戻っている。
#[test]
fn panic_in_get_reports_worker_panicked_and_leaves_the_write_stage() {
    static ONLINE: OnlineCounter = OnlineCounter::new();
    let target = TempPath::under_target("update-worker-panic");
    std::fs::create_dir_all(target.path()).expect("対象のフォルダ");
    let gate = Arc::new(WorkGate::default());
    let new_fetch: NewFetch = Arc::new(|| Ok(Box::new(PanickingGet) as Box<dyn Fetch>));
    let (desk, asks) = mpsc::channel();
    let (jobs, handle) = spawn_worker(desk, gate.clone(), new_fetch, &ONLINE);
    let (kanade, rx) = mpsc::channel();
    let answerer = thread::spawn(move || answer_kanade(rx));
    jobs.send(UpdateJob {
        order: balloon_order(target.path().to_path_buf()),
        kanade,
        roots: BasewareRoots::new(Vec::new()),
        exe_dir: None,
    })
    .expect("仕事を渡す");

    let mut seen = Vec::new();
    loop {
        let ask = wait_recv("背景スレッドの頼み", Progress::Unknown, &asks).expect("頼みが届く");
        let last = matches!(ask, DeskAsk::OrderDone | DeskAsk::WorkerPanicked);
        seen.push(ask);
        if last {
            break;
        }
    }
    assert_eq!(seen, [DeskAsk::Started, DeskAsk::WorkerPanicked]);
    assert!(!ONLINE.is_online(), "倒れの知らせの前に数が戻っている");
    drop(jobs);
    assert!(join_bounded("倒れたスレッドの終わり", handle).is_err());
    join_thread_bounded("kanade の代わりの終わり", None, answerer).expect("答える側は倒れない");

    let mut world = World::new();
    register_gate(&mut world, "update", gate, |_| {});
    let (waits, events) = capture(|| begin_close(&mut world));
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "門に名が残らない: {events:?}"
    );
    let ((), events) = capture(|| {
        waits.wait(WaitBudget {
            started: Instant::now(),
            limit: Duration::ZERO,
        })
    });
    assert!(
        !events.iter().any(|e| {
            matches!(
                e.field_str("event"),
                Some("exit_wait_done" | "exit_wait_timeout")
            )
        }),
        "書く段に残らない: {events:?}"
    );
}

/// 倒れの知らせを受けた窓口は走っていないへ戻り、送出端を捨てる（次の依頼で起こし直す）。
#[test]
fn desk_returns_to_idle_and_forgets_the_worker_after_a_panic() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    let (worker, _jobs) = mpsc::channel();
    let asks = {
        let mut desk = world.non_send_mut::<UpdateDesk>();
        desk.worker = Some(worker);
        desk.stage = Stage::Running;
        desk.asks_tx.clone()
    };
    asks.send(DeskAsk::WorkerPanicked).expect("窓口へ入れる");
    drain(&mut world);
    let desk = world.non_send::<UpdateDesk>();
    assert_eq!(desk.stage, Stage::Idle);
    assert!(desk.worker.is_none());
}
