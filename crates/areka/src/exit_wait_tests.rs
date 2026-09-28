//! 終了で背景の仕事を待つ口の決定論テスト（要件 8.1・8.3・8.4・8.10・11.8・12.4）。
//!
//! 確かめること: 書いていない仕事は待たずに記録を 1 件残す・書いている最中は書き終わりの合図で戻る・
//! 残り 0 の予算では直ちに戻って `warn!(exit_wait_timeout)` が 1 件・前の段と同じ出発点から数える・
//! 閉じた門は「始める」「書く段へ入る」を断る・登記した片付けが `begin_close` で呼ばれる。
//!
//! 実時間の待ちに依らない: 書き終わりは別スレッドが合図で知らせ、上限は出発点を過去に置いて使い切る。

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use log_capture_kit::{CapturedEvent, capture};

use super::*;

// ---------------------------------------------------------------- 道具立て

const NAME: &str = "test-work";
const LABEL: &str = r"C:\areka-exit-wait-tests\sample.nar";

/// 門を 1 つ登記した World と、その門を返す。片付けは何もしない。
fn world_with_gate() -> (World, Arc<WorkGate>) {
    let mut world = World::new();
    let gate = Arc::new(WorkGate::default());
    register_gate(&mut world, NAME, gate.clone(), |_| {});
    (world, gate)
}

/// 出発点から上限まで使い切った予算（残り 0）。
fn spent_budget() -> WaitBudget {
    WaitBudget {
        started: Instant::now() - EXIT_WAIT_LIMIT,
        limit: EXIT_WAIT_LIMIT,
    }
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

// ---------------------------------------------------------------- 書いていない仕事

/// 書く前の段に居る仕事は待たない。途中でやめた物の名前を `warn!` に 1 件残す（要件 8.4）。
#[test]
fn work_not_writing_is_abandoned_without_waiting() {
    let (mut world, gate) = world_with_gate();
    assert!(gate.begin(LABEL.to_owned()));

    let (waits, events) = capture(|| begin_close(&mut world));
    let abandoned = events_named(&events, "exit_wait_abandoned");
    assert_eq!(abandoned.len(), 1, "途中でやめた記録は 1 件: {events:?}");
    assert_eq!(abandoned[0].level, tracing::Level::WARN);
    assert_eq!(abandoned[0].field_str("name"), Some(NAME));
    assert_eq!(abandoned[0].field_str("label"), Some(LABEL));
    assert_eq!(events.len(), 1, "記録は途中でやめた 1 件だけ: {events:?}");

    // 待つ物が無いので、上限の遠い予算でも直ちに戻り、記録を足さない。
    let t0 = Instant::now();
    let ((), events) = capture(|| {
        waits.wait(WaitBudget {
            started: Instant::now(),
            limit: Duration::from_secs(3600),
        })
    });
    assert!(t0.elapsed() < Duration::from_secs(1), "待たない");
    assert!(events.is_empty(), "待つ物が無ければ記録 0 件: {events:?}");
}

/// 何も扱っていない門・門の登記が無い World は、記録 0 件で直ちに戻る。
#[test]
fn idle_gate_and_empty_world_leave_no_record() {
    let (mut world, _gate) = world_with_gate();
    let (waits, events) = capture(|| begin_close(&mut world));
    assert!(events.is_empty(), "{events:?}");
    let ((), events) = capture(|| waits.wait(spent_budget()));
    assert!(events.is_empty(), "{events:?}");

    let mut bare = World::new();
    let (waits, events) = capture(|| begin_close(&mut bare));
    assert!(events.is_empty(), "{events:?}");
    let ((), events) = capture(|| waits.wait(spent_budget()));
    assert!(events.is_empty(), "{events:?}");
}

// ---------------------------------------------------------------- 書いている最中の仕事

/// 書いている最中の仕事は、書き終わりの合図で戻り `info!(exit_wait_done)` を 1 件残す（要件 8.1）。
#[test]
fn writing_work_returns_when_write_ends() {
    let (mut world, gate) = world_with_gate();
    assert!(gate.begin(LABEL.to_owned()));
    assert!(gate.enter_write());

    let (waits, events) = capture(|| begin_close(&mut world));
    assert!(
        events.is_empty(),
        "書いている最中は途中でやめた扱いにしない: {events:?}"
    );

    let (go_tx, go_rx) = mpsc::channel::<()>();
    let worker_gate = gate.clone();
    let worker = thread::spawn(move || {
        go_rx.recv().expect("合図を受ける");
        worker_gate.leave_write();
        worker_gate.end();
    });
    go_tx.send(()).expect("合図を送る");

    let ((), events) = capture(|| {
        waits.wait(WaitBudget {
            started: Instant::now(),
            limit: Duration::from_secs(3600),
        })
    });
    worker.join().expect("背景の仕事が終わる");

    let done = events_named(&events, "exit_wait_done");
    assert_eq!(done.len(), 1, "間に合った記録は 1 件: {events:?}");
    assert_eq!(done[0].level, tracing::Level::INFO);
    assert!(done[0].field("ms").is_some(), "待った時間を載せる");
    assert!(events_named(&events, "exit_wait_timeout").is_empty());
}

/// 残り 0 の予算では待たずに戻り、`warn!(exit_wait_timeout)` を 1 件残す（要件 8.3）。
#[test]
fn spent_budget_returns_at_once_with_one_timeout_warning() {
    let (mut world, gate) = world_with_gate();
    assert!(gate.begin(LABEL.to_owned()));
    assert!(gate.enter_write());
    let waits = begin_close(&mut world);

    let t0 = Instant::now();
    let ((), events) = capture(|| waits.wait(spent_budget()));
    assert!(t0.elapsed() < Duration::from_secs(1), "残り 0 なら待たない");

    let timeout = events_named(&events, "exit_wait_timeout");
    assert_eq!(timeout.len(), 1, "上限の記録は 1 件: {events:?}");
    assert_eq!(timeout[0].level, tracing::Level::WARN);
    assert_eq!(timeout[0].field_str("name"), Some(NAME));
    assert_eq!(timeout[0].field_str("label"), Some(LABEL));
    assert!(
        timeout[0].message().contains(".nar-work"),
        "元の中身が作業フォルダに残りうることを書く: {}",
        timeout[0].message()
    );
    assert_eq!(events.len(), 1, "{events:?}");
}

/// OS の終了の形: 前の段（ゴーストを降ろす待ち）と同じ出発点を渡すと、使った分だけ短く待ち、
/// 出発点からの合計は上限を超えない（要件 8.2・11.8）。今から上限を数え直すと 1 秒以上待つ。
#[test]
fn shared_start_keeps_total_within_limit() {
    // 前の段が上限を使い切った（さらに超えた）後: 待たずに戻る。
    let (mut world, gate) = world_with_gate();
    assert!(gate.begin(LABEL.to_owned()));
    assert!(gate.enter_write());
    let waits = begin_close(&mut world);
    let t0 = Instant::now();
    waits.wait(WaitBudget {
        started: Instant::now() - Duration::from_secs(10),
        limit: EXIT_WAIT_LIMIT,
    });
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "使い切った後は待たない"
    );

    // 前の段が上限のほとんどを使った後: 残り（20 ミリ秒）だけ待つ。
    let (mut world, gate) = world_with_gate();
    assert!(gate.enter_write());
    let waits = begin_close(&mut world);
    let used = Duration::from_secs(1);
    let budget = WaitBudget {
        started: Instant::now() - used,
        limit: used + Duration::from_millis(20),
    };
    let t0 = Instant::now();
    waits.wait(budget);
    assert!(
        t0.elapsed() < used,
        "残りだけ待つ（今から上限を数え直さない）: {:?}",
        t0.elapsed()
    );
    assert!(budget.started.elapsed() >= budget.limit, "上限までは待つ");
}

// ---------------------------------------------------------------- 閉じた門・片付け

/// 門が閉じた後は「始める」「書く段へ入る」が偽を返す（要件 12.4・設計で決めたこと 8）。
#[test]
fn closed_gate_refuses_begin_and_enter_write() {
    let (mut world, gate) = world_with_gate();
    assert!(!gate.is_closing());
    let _ = begin_close(&mut world);
    assert!(gate.is_closing());
    assert!(!gate.begin(LABEL.to_owned()), "閉じた後は始めない");
    assert!(!gate.enter_write(), "閉じた後は書く段へ入らない");
}

#[derive(Resource)]
struct Cleaned(&'static str);

/// 登記した片付けは `begin_close` で UI スレッドの World を渡されて呼ばれる（要件 8.10）。
#[test]
fn begin_close_runs_registered_cleanup() {
    let mut world = World::new();
    register_gate(&mut world, NAME, Arc::new(WorkGate::default()), |w| {
        w.insert_resource(Cleaned(NAME))
    });
    assert!(world.get_resource::<Cleaned>().is_none());
    let _ = begin_close(&mut world);
    assert_eq!(world.get_resource::<Cleaned>().map(|c| c.0), Some(NAME));
}
