//! 背景スレッドと本物の口の決定論テスト（design「Testing Strategy / 背景スレッドの口」・要件 1.11・
//! 1.18・4.4・7.1・7.3・7.4・9.7・9.12）。
//!
//! kanade の代わりに素の受信端で送出を受けて答える。確かめること: `RaiseOutcome` の 5 値と切断の写し・
//! 照会は `homeurl` と `useorigin1` を 1 回で・終了が始まった後の送出が 0 件・門が閉じていれば取得口
//! にも `run` にも入らない・門の開いた一周は書く段の中で `run` を呼び `label` が「更新先 → 対象」・
//! 取得口を作れなければ記録せずに門を手放す・スレッドが仕事を 1 件走らせて窓口へ頼む・本番の
//! 取得口の型を綴る本番ファイルは 2 つだけ（字面の検査）。
//!
//! 実時間の待ちに依らない: 答える側は受信端の受け取りで、口は返信端の受け取りで揃える。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod, WaitBudget};
use areka_update::{Fetch, FetchError};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::*;
use crate::exit_wait::{ClosingWaits, begin_close, register_gate};
use crate::install::fetch_url_test_support::FakeFetch;
use crate::update::{SummaryKind, UpdateOrder, UpdateReason};

// ---------------------------------------------------------------- 道具立て

const EVENT: &str = "OnUpdateBegin";
const HOMEURL: &str = "https://example.invalid/emo2/";

fn refs() -> Vec<String> {
    vec!["emo2".to_owned(), r"C:\root\ghost\emo2".to_owned()]
}

/// 呼ばれた回数を数える取得口の作り方（作る取得口は偽物・表は空＝どれも「無い」）。
fn counting_fetch() -> (NewFetch, Arc<AtomicUsize>) {
    let made = Arc::new(AtomicUsize::new(0));
    let counter = made.clone();
    let new_fetch: NewFetch = Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(FakeFetch::new()) as Box<dyn Fetch>)
    });
    (new_fetch, made)
}

/// 本物の口と、窓口の代わりの受信端。
fn ports(
    kanade: mpsc::Sender<KanadeMsg>,
    gate: Arc<WorkGate>,
    new_fetch: NewFetch,
) -> (KanadePorts, Receiver<DeskAsk>) {
    let (desk, asks) = mpsc::channel();
    (
        KanadePorts {
            kanade,
            gate,
            desk,
            new_fetch,
        },
        asks,
    )
}

/// 門を 1 つ登記した World で終了を始める（門が閉じる）。書いている最中だった仕事を返す。
fn close(gate: &Arc<WorkGate>) -> ClosingWaits {
    let mut world = World::new();
    register_gate(&mut world, "update", gate.clone(), |_| {});
    begin_close(&mut world)
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// kanade の代わりにイベントを 1 件だけ受けて `answer` を返し（None は返信端を落とす）、
/// 本物の口の写しと記録を返す。2 件目が届いていたら赤。
fn raise_answered(answer: Option<RaiseOutcome>) -> (Raised, Vec<CapturedEvent>) {
    let (kanade, rx) = mpsc::channel();
    let (ports, _asks) = ports(kanade, Arc::default(), counting_fetch().0);
    let answerer = thread::spawn(move || {
        match rx.recv().expect("イベントが 1 件届く") {
            KanadeMsg::RaiseEvent {
                id,
                references,
                method,
                reply,
            } => {
                assert_eq!(id, EVENT);
                assert_eq!(references, refs());
                assert_eq!(method, ShioriMethod::Get, "GET で送る");
                let reply = reply.expect("返事を待つので返信端つき");
                if let Some(outcome) = answer {
                    let _ = reply.send(outcome);
                }
            }
            _ => panic!("RaiseEvent のはず"),
        }
        rx
    });
    let (raised, events) = capture(|| ports.raise(EVENT, refs()));
    let rx = answerer.join().expect("答える側が赤を出さない");
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "送るのは 1 件だけ"
    );
    (raised, events)
}

// ---------------------------------------------------------------- イベントの写し

/// `RaiseOutcome` の 5 値と返事の切断を「台本あり・返事なし・閉じた」へ写す（design の worker の節）。
#[test]
fn raise_maps_five_outcomes_and_dropped_reply() {
    // (返す値, 写し, 記録の語, 水準)
    let cases: [(Option<RaiseOutcome>, Raised, Option<(&str, tracing::Level)>); 6] = [
        (Some(RaiseOutcome::Script), Raised::Script, None),
        (Some(RaiseOutcome::NoReply), Raised::NoReply, None),
        (
            Some(RaiseOutcome::NotSteady),
            Raised::Closed,
            Some(("update_not_steady", tracing::Level::WARN)),
        ),
        (
            Some(RaiseOutcome::NotAllowed),
            Raised::NoReply,
            Some(("update_event_not_allowed", tracing::Level::ERROR)),
        ),
        (
            Some(RaiseOutcome::Failed),
            Raised::Closed,
            Some(("update_event_failed", tracing::Level::ERROR)),
        ),
        (
            None,
            Raised::Closed,
            Some(("update_kanade_gone", tracing::Level::DEBUG)),
        ),
    ];
    for (answer, expected, record) in cases {
        let (raised, events) = raise_answered(answer);
        assert_eq!(raised, expected, "{answer:?} の写し");
        match record {
            Some((name, level)) => {
                let found = events_named(&events, name);
                assert_eq!(found.len(), 1, "{answer:?} で {name} が 1 件: {events:?}");
                assert_eq!(found[0].level, level, "{answer:?} の水準");
                assert_eq!(found[0].field_str("id"), Some(EVENT));
            }
            None => assert!(
                !events.iter().any(|e| e.level <= tracing::Level::WARN),
                "{answer:?} は警告も失敗も出さない: {events:?}"
            ),
        }
    }
}

/// 送出端の先が居ない（kanade が止まった）なら送れずに閉じた扱い。
#[test]
fn raise_to_stopped_kanade_is_closed() {
    let (kanade, rx) = mpsc::channel();
    drop(rx);
    let (ports, _asks) = ports(kanade, Arc::default(), counting_fetch().0);
    let (raised, events) = capture(|| ports.raise(EVENT, refs()));
    assert_eq!(raised, Raised::Closed);
    assert_eq!(events_named(&events, "update_kanade_gone").len(), 1);
}

// ---------------------------------------------------------------- 照会

/// kanade の代わりに照会を 1 件受け、名前の列を確かめて `answers` を返す。
fn resources_answered(
    answers: Vec<ResourceOutcome>,
) -> (Option<GhostResources>, Vec<CapturedEvent>) {
    let (kanade, rx) = mpsc::channel();
    let (ports, _asks) = ports(kanade, Arc::default(), counting_fetch().0);
    let answerer = thread::spawn(move || {
        match rx.recv().expect("照会が 1 件届く") {
            KanadeMsg::ResourceQuery { ids, reply } => {
                assert_eq!(ids, ["homeurl", "useorigin1"], "2 つを 1 回で照会する");
                let _ = reply.send(ids.into_iter().zip(answers).collect());
            }
            _ => panic!("ResourceQuery のはず"),
        }
        rx
    });
    let (found, events) = capture(|| ports.resources());
    let rx = answerer.join().expect("答える側が赤を出さない");
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "照会は 1 件だけ"
    );
    (found, events)
}

/// 値は空でなければ Some、空・値なし・失敗は None（失敗は `warn!`）。
#[test]
fn resources_query_both_ids_once_and_map_values() {
    let (found, events) = resources_answered(vec![
        ResourceOutcome::Value(HOMEURL.to_owned()),
        ResourceOutcome::Value("1".to_owned()),
    ]);
    assert_eq!(
        found,
        Some(GhostResources {
            homeurl: Some(HOMEURL.to_owned()),
            useorigin1: Some("1".to_owned()),
        })
    );
    assert!(events_named(&events, "update_resource_failed").is_empty());

    let (found, events) = resources_answered(vec![
        ResourceOutcome::Value(String::new()),
        ResourceOutcome::NoContent,
    ]);
    assert_eq!(found, Some(GhostResources::default()), "空と値なしは None");
    assert!(events_named(&events, "update_resource_failed").is_empty());

    let (found, events) = resources_answered(vec![
        ResourceOutcome::Failed("timeout".to_owned()),
        ResourceOutcome::NoContent,
    ]);
    assert_eq!(found, Some(GhostResources::default()), "失敗は None");
    let failed = events_named(&events, "update_resource_failed");
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(failed[0].level, tracing::Level::WARN);
    assert_eq!(failed[0].field_str("id"), Some("homeurl"));
}

/// kanade が居なければ照会は None（手続きが要求をやめる）。
#[test]
fn resources_without_kanade_is_none() {
    let (kanade, rx) = mpsc::channel();
    drop(rx);
    let (ports, _asks) = ports(kanade, Arc::default(), counting_fetch().0);
    let (found, events) = capture(|| ports.resources());
    assert_eq!(found, None);
    assert_eq!(events_named(&events, "update_kanade_gone").len(), 1);
}

// ---------------------------------------------------------------- 終了の後

/// 終了が始まった後はイベントも照会も送らない（受信端に 0 件・要件 7.4）。
///
/// 受信端は別スレッドが受けた端から数えて落とす（返信端も落ちる）ので、送ってしまっても口は
/// 返事待ちで止まらず、数が 1 以上になって赤になる。
#[test]
fn nothing_is_sent_after_exit_began() {
    let (kanade, rx) = mpsc::channel::<KanadeMsg>();
    let counter = thread::spawn(move || rx.iter().count());
    let gate = Arc::new(WorkGate::default());
    let (ports, _asks) = ports(kanade, gate.clone(), counting_fetch().0);
    let _ = close(&gate);

    assert_eq!(ports.raise(EVENT, refs()), Raised::Closed);
    assert_eq!(ports.resources(), None);
    drop(ports);
    let sent = counter.join().expect("数える側が終わる");
    assert_eq!(sent, 0, "終了の後の送出は 0 件");
}

// ---------------------------------------------------------------- エンジンの一周と門

/// 門が閉じていれば、取得口も `run`（観測）も呼ばずに「閉じた」（要件 7.1・7.3）。
#[test]
fn closed_gate_enters_neither_fetch_nor_run() {
    let (kanade, _rx) = mpsc::channel();
    let gate = Arc::new(WorkGate::default());
    let (new_fetch, made) = counting_fetch();
    let (ports, _asks) = ports(kanade, gate.clone(), new_fetch);
    let _ = close(&gate);

    let mut observed = 0;
    let run = ports.run_engine(HOMEURL, Path::new(r"C:\no-such"), &mut |_| observed += 1);
    assert!(matches!(run, EngineRun::Closed), "{run:?}");
    assert_eq!(made.load(Ordering::SeqCst), 0, "取得口を作らない");
    assert_eq!(observed, 0, "run に入らない");
}

/// 最初の取得で終了を始める偽の取得口。そのとき書いている最中だった仕事を預かる。
struct CloseOnFirstGet {
    world: Arc<Mutex<World>>,
    waits: Arc<Mutex<Option<ClosingWaits>>>,
}

impl Fetch for CloseOnFirstGet {
    fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
        let mut waits = self.waits.lock().expect("鍵");
        if waits.is_none() {
            *waits = Some(begin_close(&mut self.world.lock().expect("鍵")));
        }
        Err(FetchError::NotFound)
    }
}

/// 門の開いた一周: `run` は書く段の中で呼ばれ（取得の最中に終了が始まっても途中でやめた扱いに
/// ならない）、門の `label` は「更新先 → 対象」、返ったら書く段を出ている（待ちは直ちに終わる）。
#[test]
fn open_gate_runs_engine_inside_write_stage_with_label() {
    let tmp = TempPath::new("update-worker-run");
    let target = tmp.child("ghost");
    std::fs::create_dir_all(&target).expect("対象のフォルダを作る");

    let gate = Arc::new(WorkGate::default());
    let mut world = World::new();
    register_gate(&mut world, "update", gate.clone(), |_| {});
    let world = Arc::new(Mutex::new(world));
    let waits = Arc::new(Mutex::new(None));
    let new_fetch: NewFetch = Arc::new({
        let (world, waits) = (world.clone(), waits.clone());
        move || {
            Ok(Box::new(CloseOnFirstGet {
                world: world.clone(),
                waits: waits.clone(),
            }) as Box<dyn Fetch>)
        }
    });
    let (kanade, _rx) = mpsc::channel();
    let (ports, _asks) = ports(kanade, gate, new_fetch);

    let (run, events) = capture(|| ports.run_engine(HOMEURL, &target, &mut |_| {}));
    assert!(matches!(run, EngineRun::Done(Err(_))), "{run:?}");
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "取得の最中は書く段の中（途中でやめた扱いにしない）: {events:?}"
    );

    let waits = waits
        .lock()
        .expect("鍵")
        .take()
        .expect("run が取得口を呼んだ");
    let ((), events) = capture(|| {
        waits.wait(WaitBudget {
            started: Instant::now(),
            limit: Duration::from_secs(3600),
        })
    });
    let done = events_named(&events, "exit_wait_done");
    assert_eq!(done.len(), 1, "返ったら書く段を出ている: {events:?}");
    let label = format!("{HOMEURL} → {}", target.display());
    assert_eq!(done[0].field_str("label"), Some(label.as_str()));
}

/// `get` の呼ばれた回数を数える偽の取得口（どれも「無い」）。
struct CountingGet(Arc<AtomicUsize>);

impl Fetch for CountingGet {
    fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(FetchError::NotFound)
    }
}

/// 取得口を作っている間に終了が始まった: 書く段へ入れないので `run` に入らず「閉じた」。
/// 門の `label`（「更新先 → 対象」）は取得口を作る前に置かれていて、終了は書く前の段として 1 件
/// 記録し、書いている最中の仕事は無い（待ちは何も記録しない）。
#[test]
fn gate_closed_while_making_fetch_skips_run_and_releases_gate() {
    let tmp = TempPath::new("update-worker-enter-write");
    let target = tmp.child("ghost");
    std::fs::create_dir_all(&target).expect("対象のフォルダを作る");

    let gate = Arc::new(WorkGate::default());
    let mut world = World::new();
    register_gate(&mut world, "update", gate.clone(), |_| {});
    let world = Arc::new(Mutex::new(world));
    let waits = Arc::new(Mutex::new(None));
    let gets = Arc::new(AtomicUsize::new(0));
    let new_fetch: NewFetch = Arc::new({
        let (world, waits, gets) = (world.clone(), waits.clone(), gets.clone());
        move || {
            *waits.lock().expect("鍵") = Some(begin_close(&mut world.lock().expect("鍵")));
            Ok(Box::new(CountingGet(gets.clone())) as Box<dyn Fetch>)
        }
    });
    let (kanade, _rx) = mpsc::channel();
    let (ports, _asks) = ports(kanade, gate, new_fetch);

    let mut observed = 0;
    let (run, events) = capture(|| ports.run_engine(HOMEURL, &target, &mut |_| observed += 1));
    assert!(matches!(run, EngineRun::Closed), "{run:?}");
    assert_eq!(observed, 0, "run に入らない");
    assert_eq!(gets.load(Ordering::SeqCst), 0, "取得しない");
    let abandoned = events_named(&events, "exit_wait_abandoned");
    assert_eq!(abandoned.len(), 1, "書く前の段で終了: {events:?}");
    let label = format!("{HOMEURL} → {}", target.display());
    assert_eq!(abandoned[0].field_str("label"), Some(label.as_str()));

    let waits = waits.lock().expect("鍵").take().expect("取得口を作った");
    let ((), events) = capture(|| {
        waits.wait(WaitBudget {
            started: Instant::now(),
            limit: Duration::from_secs(3600),
        })
    });
    assert!(
        events_named(&events, "exit_wait_done").is_empty()
            && events_named(&events, "exit_wait_timeout").is_empty(),
        "書いている最中の仕事は無い: {events:?}"
    );
}

/// 取得口を作れなければ記録を出さずに（手続きが `error!` を 1 件残す）門を手放し `Unavailable`。
#[test]
fn unavailable_fetch_releases_gate_without_logging() {
    let (kanade, _rx) = mpsc::channel();
    let gate = Arc::new(WorkGate::default());
    let new_fetch: NewFetch = Arc::new(|| Err(FetchError::Connect));
    let (ports, _asks) = ports(kanade, gate.clone(), new_fetch);

    let (run, events) =
        capture(|| ports.run_engine(HOMEURL, Path::new(r"C:\no-such"), &mut |_| {}));
    assert!(
        matches!(run, EngineRun::Unavailable(FetchError::Connect)),
        "{run:?}"
    );
    assert!(
        events_named(&events, "update_fetch_unavailable").is_empty(),
        "記録は手続きの 1 件だけ: {events:?}"
    );

    let (_waits, events) = capture(|| close(&gate));
    assert!(
        events.is_empty(),
        "門に名前が残っていない（途中でやめた記録 0）: {events:?}"
    );
}

// ---------------------------------------------------------------- 窓口への頼みとスレッド

/// 読み直しと「始まった」は窓口へ頼むだけ（返事を待たない）。
#[test]
fn reload_and_started_are_asked_to_desk() {
    let (kanade, _rx) = mpsc::channel();
    let (ports, asks) = ports(kanade, Arc::default(), counting_fetch().0);
    ports.standard_started();
    ports.request_reload(Path::new(r"C:\root\ghost\emo2"));
    assert_eq!(asks.try_recv(), Ok(DeskAsk::Started));
    assert_eq!(
        asks.try_recv(),
        Ok(DeskAsk::Reload {
            ghost_dir: PathBuf::from(r"C:\root\ghost\emo2"),
        })
    );
    assert!(matches!(asks.try_recv(), Err(TryRecvError::Empty)));
}

/// スレッド `update` は仕事を受けて手続きを走らせ（仕事の送出端へ照会し）、終わったら「終わった」を頼む。
#[test]
fn worker_runs_a_job_and_asks_order_done() {
    let (desk, asks) = mpsc::channel();
    let (new_fetch, made) = counting_fetch();
    let (jobs, handle) = spawn_worker(desk, Arc::default(), new_fetch);
    let (kanade, rx) = mpsc::channel();
    let order = UpdateOrder {
        targets: Vec::new(),
        reason: UpdateReason::Script,
        summary: SummaryKind::Result,
        ghost_dir: PathBuf::from(r"C:\root\ghost\emo2"),
        ghost_folder: Some("emo2".to_owned()),
    };
    jobs.send(UpdateJob { order, kanade }).expect("仕事を渡す");

    assert_eq!(asks.recv(), Ok(DeskAsk::Started));
    match rx.recv().expect("仕事の送出端へ照会が届く") {
        KanadeMsg::ResourceQuery { ids, reply } => {
            let _ = reply.send(
                ids.into_iter()
                    .map(|id| (id, ResourceOutcome::NoContent))
                    .collect(),
            );
        }
        _ => panic!("ResourceQuery のはず"),
    }
    assert_eq!(asks.recv(), Ok(DeskAsk::OrderDone));
    assert_eq!(
        made.load(Ordering::SeqCst),
        0,
        "対象 0 ならエンジンは回らない"
    );

    drop(jobs);
    handle
        .join()
        .expect("仕事の送出端が落ちたらスレッドは終わる");
}

// ---------------------------------------------------------------- 字面の検査

/// 本体の本番ファイル（`*_tests.rs`・`*_test_support.rs` を除く）で `WinHttp` で始まる識別子は
/// `WinHttpFetch` だけで、綴るのは `install/fetch_url.rs` と `update/worker.rs` の 2 つだけ（裁定 3）。
#[test]
fn winhttp_is_spelled_only_in_two_production_files() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src.clone()];
    let mut spelled: Vec<(String, String)> = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("ソースの木を辿る") {
            let path = entry.expect("項目を読む").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.ends_with(".rs")
                || name.ends_with("_tests.rs")
                || name.ends_with("_test_support.rs")
            {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("ソースを読む");
            let relative = path
                .strip_prefix(&src)
                .expect("src の下")
                .to_string_lossy()
                .replace('\\', "/");
            for (at, _) in text.match_indices("WinHttp") {
                let before = text[..at].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let ident: String = text[at..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                spelled.push((relative.clone(), ident));
            }
        }
    }

    let strays: Vec<_> = spelled
        .iter()
        .filter(|(file, ident)| {
            ident != "WinHttpFetch"
                || !matches!(file.as_str(), "install/fetch_url.rs" | "update/worker.rs")
        })
        .collect();
    assert!(
        strays.is_empty(),
        "本番の取得口の型を綴る所が増えた: {strays:?}"
    );
    let mut files: Vec<&str> = spelled.iter().map(|(f, _)| f.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    assert_eq!(
        files,
        ["install/fetch_url.rs", "update/worker.rs"],
        "2 つとも綴っている（検査が空振りしていない）"
    );
}
