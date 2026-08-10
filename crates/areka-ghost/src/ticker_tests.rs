use super::*;
use std::sync::mpsc::{self, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;

/// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
/// テストを失敗させる（どのテストもハングしないことを保証する・areka-actor 流儀）。
fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    let (done_tx, done_rx) = sync_channel::<()>(0);
    thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

// ---- 純粋層: BoundarySchedule ----

#[test]
fn next_grid_multiple_strictly_after_computes_smallest_future_multiple() {
    assert_eq!(next_grid_multiple_strictly_after(1000, 0), 1000);
    assert_eq!(next_grid_multiple_strictly_after(1000, 999), 1000);
    assert_eq!(next_grid_multiple_strictly_after(1000, 1000), 2000);
    assert_eq!(next_grid_multiple_strictly_after(1000, 1500), 2000);
    assert_eq!(next_grid_multiple_strictly_after(50, 149), 150);
}

#[test]
fn starting_at_sets_first_deadline_strictly_after_now() {
    let schedule = BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(0));
    assert_eq!(schedule.next_deadline_ms, 1000);

    // ちょうど境界上でも次の境界へ進む（起動直後の即時発火を避ける）。
    let schedule =
        BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(1000));
    assert_eq!(schedule.next_deadline_ms, 2000);
}

#[test]
fn starting_at_aligns_to_absolute_grid_not_relative_to_spawn_time() {
    // クロックがグリッド非整列値（2347ms）で起動しても、最初の締切は
    // 「起動時刻+interval」（3347）ではなく OS クロック絶対グリッド上の
    // 次の境界（3000）へ整列する——将来の複数 ticker インスタンスが
    // 共有機構なしで同一グリッドへ自然同期するための不変条件（design 参照）。
    let schedule =
        BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(2347));
    assert_eq!(
        schedule.next_deadline_ms, 3000,
        "絶対グリッド整列: 2347ms 起動でも次境界は 3000（3347 ではない）"
    );
}

#[test]
#[should_panic(expected = "positive")]
fn starting_at_panics_on_zero_interval() {
    let _ = BoundarySchedule::starting_at(Duration::from_millis(0), MonotonicMs(0));
}

#[test]
fn remaining_counts_down_to_zero_at_boundary() {
    let schedule = BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(0));
    assert_eq!(
        schedule.remaining(MonotonicMs(0)),
        Duration::from_millis(1000)
    );
    assert_eq!(
        schedule.remaining(MonotonicMs(400)),
        Duration::from_millis(600)
    );
    assert_eq!(schedule.remaining(MonotonicMs(1000)), Duration::ZERO);
    assert_eq!(schedule.remaining(MonotonicMs(1500)), Duration::ZERO);
}

#[test]
fn poll_does_not_fire_before_boundary() {
    let mut schedule =
        BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(0));
    let result = schedule.poll(MonotonicMs(999));
    assert_eq!(
        result,
        BoundaryPoll {
            fired: false,
            catch_up: false
        }
    );
    // 未到達の poll は状態を変えない。
    assert_eq!(schedule.next_deadline_ms, 1000);
}

#[test]
fn poll_fires_exactly_once_on_time_and_advances_one_boundary() {
    let mut schedule =
        BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(0));
    let result = schedule.poll(MonotonicMs(1000));
    assert_eq!(
        result,
        BoundaryPoll {
            fired: true,
            catch_up: false
        }
    );
    assert_eq!(schedule.next_deadline_ms, 2000);

    // 次の境界も定刻どおりなら catch-up にならない。
    let result = schedule.poll(MonotonicMs(2000));
    assert_eq!(
        result,
        BoundaryPoll {
            fired: true,
            catch_up: false
        }
    );
    assert_eq!(schedule.next_deadline_ms, 3000);
}

#[test]
fn poll_skips_multiple_missed_boundaries_and_fires_only_once() {
    // サスペンド復帰等を模す: 次境界(1000)を大幅に過ぎた 8200 まで一気に進む。
    let mut schedule =
        BoundarySchedule::starting_at(Duration::from_millis(1000), MonotonicMs(0));
    let result = schedule.poll(MonotonicMs(8200));
    assert_eq!(
        result,
        BoundaryPoll {
            fired: true,
            catch_up: true
        }
    );
    // 次デッドラインは 8200 より厳密に未来のグリッド倍数（9000）へスナップ済み
    // ＝ 2000, 3000, ..., 8000 の中間境界はすべてスキップされ再送されない。
    assert_eq!(schedule.next_deadline_ms, 9000);

    // スキップ後の以降の呼出は再び定刻扱いに戻る。
    let result = schedule.poll(MonotonicMs(9000));
    assert_eq!(
        result,
        BoundaryPoll {
            fired: true,
            catch_up: false
        }
    );
    assert_eq!(schedule.next_deadline_ms, 10000);
}

#[test]
fn poll_50ms_base_interval_grid_matches_dispatcher_default() {
    let mut schedule = BoundarySchedule::starting_at(Duration::from_millis(50), MonotonicMs(0));
    assert_eq!(
        schedule.remaining(MonotonicMs(0)),
        Duration::from_millis(50)
    );
    let result = schedule.poll(MonotonicMs(50));
    assert!(result.fired && !result.catch_up);
    assert_eq!(schedule.next_deadline_ms, 100);
}

// ---- アクター統合層: spawn_ticker ----

/// 決定論的な `Fn`-backed clock を `Arc<Mutex<u64>>` 経由で包み、テストスレッドから
/// 任意に「時刻」を進められるようにするヘルパ。
///
/// `spawn_ticker` は起動直後に `start_now = clock()` を1回読んでから境界を初期化する
/// （[`BoundarySchedule::starting_at`]）。テスト側がこの初回読取の**完了後**に時計を
/// 進めないと、初回読取と書換えが競合し「初回読取が既に進めた後の値を拾ってしまい、
/// 以後その値を境界の起点にしてしまう」レースが起きる（本ヘルパ導入前に実際に発生した
/// flaky failure）。そこで初回呼出の**値読取が完了した直後**に `started_tx` へ一度だけ
/// 通知する——テストはこの通知を受けてから時計を書き換えることで、
/// 「起動時刻の読取は必ず旧い値を観測する」ことを決定論的に保証できる。
fn shared_clock(
    value: Arc<Mutex<u64>>,
    started_tx: mpsc::Sender<()>,
) -> Box<dyn Fn() -> MonotonicMs + Send> {
    let notified = std::sync::atomic::AtomicBool::new(false);
    Box::new(move || {
        let v = *value.lock().expect("clock mutex poisoned");
        if !notified.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let _ = started_tx.send(());
        }
        MonotonicMs(v)
    })
}

#[test]
fn spawn_ticker_delivers_kanade_and_dispatcher_ticks_on_injected_clock() {
    // dispatcher 側メッセージ型は task 2.5 未着手のため KanadeMsg を仮の D として使う
    // （設計メモのとおり同一型を暫定転用。`KanadeMsg` はここでは `From<Tick>` を実装
    // した専用のテスト用ラッパ型を介して満たす）。
    struct TestDispatcherMsg {
        now: MonotonicMs,
    }
    impl From<Tick> for TestDispatcherMsg {
        fn from(tick: Tick) -> Self {
            TestDispatcherMsg { now: tick.now }
        }
    }

    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, started_rx) = mpsc::channel::<()>();

    let config = TickerConfig {
        base_interval: Duration::from_millis(50),
        kanade_interval: Duration::from_millis(1000),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (dispatcher_tx, dispatcher_rx) = mpsc::channel::<TestDispatcherMsg>();

    let (stop_tx, handle) = spawn_ticker(config, kanade_tx, dispatcher_tx);

    // ticker の起動時刻読取（`start_now`）が完了するのを待ってから時計を進める
    // （進めるのが早すぎると起動時刻そのものが 1000 になってしまうレースを防ぐ）。
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("ticker should read the clock once at startup");

    // 時計を 1000ms へ進める: dispatcher 境界(50の倍数)・kanade 境界(1000)の両方に到達。
    *clock_value.lock().expect("lock") = 1000;

    let dispatcher_tick = dispatcher_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("dispatcher should receive a tick once clock reaches a 50ms boundary");
    assert_eq!(dispatcher_tick.now, MonotonicMs(1000));

    let kanade_tick = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade should receive a tick once clock reaches the 1000ms boundary");
    match kanade_tick {
        KanadeMsg::Tick { now } => assert_eq!(now, MonotonicMs(1000)),
        _ => unreachable!("only Tick is sent by ticker"),
    }

    stop_tx.send(TickerMsg::Close).expect("send Close");
    run_bounded(
        "ticker join after Close",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("ticker terminates normally after Close");
        },
    );
}

#[test]
fn spawn_ticker_stops_on_control_channel_disconnect() {
    struct TestDispatcherMsg;
    impl From<Tick> for TestDispatcherMsg {
        fn from(_: Tick) -> Self {
            TestDispatcherMsg
        }
    }

    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, _started_rx) = mpsc::channel::<()>();
    let config = TickerConfig {
        base_interval: Duration::from_millis(50),
        kanade_interval: Duration::from_millis(1000),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (dispatcher_tx, _dispatcher_rx) = mpsc::channel::<TestDispatcherMsg>();

    let (stop_tx, handle) = spawn_ticker(config, kanade_tx, dispatcher_tx);

    // 制御チャンネルの送信端を手放す＝disconnected 経路。
    drop(stop_tx);

    run_bounded(
        "ticker join after control channel disconnect",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("ticker terminates normally on control channel disconnect");
        },
    );
}

#[test]
fn spawn_ticker_sticky_stops_sending_to_disconnected_dispatcher_but_keeps_kanade_alive() {
    struct TestDispatcherMsg;
    impl From<Tick> for TestDispatcherMsg {
        fn from(_: Tick) -> Self {
            TestDispatcherMsg
        }
    }

    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let config = TickerConfig {
        base_interval: Duration::from_millis(50),
        kanade_interval: Duration::from_millis(1000),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (dispatcher_tx, dispatcher_rx) = mpsc::channel::<TestDispatcherMsg>();

    // dispatcher 受信端を即座に落とす＝以後 dispatcher.send は Err になる。
    drop(dispatcher_rx);

    let (stop_tx, handle) = spawn_ticker(config, kanade_tx, dispatcher_tx);

    // ticker の起動時刻読取が完了するのを待ってから時計を進める（レース防止・上記
    // `shared_clock` doc 参照）。
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("ticker should read the clock once at startup");

    // 1000ms へ進めて両系統の境界に到達させる。dispatcher は切断済みだが kanade は届く。
    *clock_value.lock().expect("lock") = 1000;

    let kanade_tick = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade must keep receiving ticks even though dispatcher disconnected");
    match kanade_tick {
        KanadeMsg::Tick { now } => assert_eq!(now, MonotonicMs(1000)),
        _ => unreachable!("only Tick is sent by ticker"),
    }

    stop_tx.send(TickerMsg::Close).expect("send Close");
    run_bounded(
        "ticker join after sticky-disconnect scenario",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("ticker terminates normally after Close");
        },
    );
}

// ---- 単発ループレーン: spawn_loop_ticker ----

#[test]
fn loop_ticker_config_default_is_16ms_and_real_clock() {
    let config = LoopTickerConfig::default();
    assert_eq!(config.interval, Duration::from_millis(16));
    // 既定 clock が実クロック（非減少）であることのみ確認する。
    let first = (config.clock)();
    let second = (config.clock)();
    assert!(second.0 >= first.0, "default clock must be non-decreasing");
}

#[test]
fn spawn_loop_ticker_delivers_once_per_grid_firing_on_injected_clock() {
    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, started_rx) = mpsc::channel::<()>();

    let config = LoopTickerConfig {
        interval: Duration::from_millis(16),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    // deliver クロージャは Tick を検証用チャネルへ横流しする（配送 = クロージャ経由・
    // From<Tick> 型結合なし）。
    let (tick_tx, tick_rx) = mpsc::channel::<Tick>();
    let deliver: Box<dyn FnMut(Tick) + Send> = Box::new(move |tick: Tick| {
        let _ = tick_tx.send(tick);
    });

    let (stop_tx, handle) = spawn_loop_ticker(config, deliver);

    // 起動時刻読取（start_now）完了を待ってから時計を進める（レース防止・shared_clock doc）。
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("loop ticker should read the clock once at startup");

    // 16ms グリッド境界へ到達させる。
    *clock_value.lock().expect("lock") = 16;

    let tick = tick_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("deliver should be called once the clock reaches a 16ms boundary");
    assert_eq!(tick.now, MonotonicMs(16));

    stop_tx.send(TickerMsg::Close).expect("send Close");
    run_bounded(
        "loop ticker join after Close",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("loop ticker terminates normally after Close");
        },
    );
}

#[test]
fn spawn_loop_ticker_catch_up_delivers_exactly_once_across_multiple_missed_boundaries() {
    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, started_rx) = mpsc::channel::<()>();

    let config = LoopTickerConfig {
        interval: Duration::from_millis(16),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    let (tick_tx, tick_rx) = mpsc::channel::<Tick>();
    let deliver: Box<dyn FnMut(Tick) + Send> = Box::new(move |tick: Tick| {
        let _ = tick_tx.send(tick);
    });

    let (stop_tx, handle) = spawn_loop_ticker(config, deliver);

    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("loop ticker should read the clock once at startup");

    // 0 から 100 まで一気に進める＝16,32,48,64,80,96 の 6 境界を跨ぐ（サスペンド復帰等）。
    // catch-up 政策により deliver は**ちょうど 1 回**だけ呼ばれる。
    *clock_value.lock().expect("lock") = 100;

    let tick = tick_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("catch-up must deliver exactly once with the current clock");
    assert_eq!(tick.now, MonotonicMs(100));

    // 2 回目の配送が来ないこと＝跨いだ中間境界を再送しない（catch-up = 1）。
    // 時計は 100 のまま（次デッドライン 112 未満）なので追加発火は起きない。
    assert!(
        tick_rx.recv_timeout(Duration::from_millis(200)).is_err(),
        "catch-up must collapse multiple missed boundaries into a single deliver"
    );

    stop_tx.send(TickerMsg::Close).expect("send Close");
    run_bounded(
        "loop ticker join after catch-up scenario",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("loop ticker terminates normally after Close");
        },
    );
}

#[test]
fn spawn_loop_ticker_stops_on_close() {
    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, _started_rx) = mpsc::channel::<()>();
    let config = LoopTickerConfig {
        interval: Duration::from_millis(16),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    // deliver は呼ばれない想定（境界未到達で即 Close）。
    let deliver: Box<dyn FnMut(Tick) + Send> = Box::new(|_tick: Tick| {});
    let (stop_tx, handle) = spawn_loop_ticker(config, deliver);

    stop_tx.send(TickerMsg::Close).expect("send Close");
    run_bounded(
        "loop ticker join after Close",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("loop ticker terminates normally after Close");
        },
    );
}

#[test]
fn spawn_loop_ticker_stops_on_control_channel_disconnect() {
    let clock_value = Arc::new(Mutex::new(0u64));
    let (started_tx, _started_rx) = mpsc::channel::<()>();
    let config = LoopTickerConfig {
        interval: Duration::from_millis(16),
        clock: shared_clock(Arc::clone(&clock_value), started_tx),
    };

    let deliver: Box<dyn FnMut(Tick) + Send> = Box::new(|_tick: Tick| {});
    let (stop_tx, handle) = spawn_loop_ticker(config, deliver);

    // 制御チャンネルの送信端を手放す＝disconnected 経路で正常終了する。
    drop(stop_tx);

    run_bounded(
        "loop ticker join after control channel disconnect",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("loop ticker terminates normally on control channel disconnect");
        },
    );
}

#[test]
fn real_clock_returns_monotonic_ms_from_get_tick_count64() {
    // 実クロックの往復確認: 2 回読んで非減少であることのみ検証する（実 OS 時計は
    // テスト側で制御できないため、値そのものではなく単調性のみを確認する）。
    let first = real_clock();
    let second = real_clock();
    assert!(second.0 >= first.0, "GetTickCount64 must be non-decreasing");
}
