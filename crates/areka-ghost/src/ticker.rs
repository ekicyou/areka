//! 差し替え可能な時刻供給コンポーネント（ticker）。
//!
//! OS時計の絶対グリッド（既定 50ms・1000ms の2周期）に整列して時刻の刻みを配り、
//! 処理遅延が蓄積しないようにする。大幅な遅延で複数の境界を跨いだ場合は各系統
//! 1回のみ配り、次の境界へスナップする。決定論テストでは起動せず外部からの
//! 直接投函に道を譲れるようにする（design.md「ghost::ticker」）。
//!
//! # 構造
//!
//! - [`BoundarySchedule`] — 「今どの境界に到達したか・次はいつか」を持つ発火判定の
//!   純粋層。スレッド・mpsc・実クロックに一切依存しない（単体テストの主対象）。
//! - [`spawn_ticker`] — 上記を単一スレッドのアクターへ配線する薄い殻。`stop_rx`
//!   （＝ `spawn_actor` の inbox＝[`TickerMsg`] 受信端）を `recv_timeout` で待ち、
//!   タイムアウトのたびに実クロックを読み境界判定を適用する。
//!
//! # `DispatcherMsg` への forward dependency（設計メモ）
//!
//! task 2.5 が定義する `DispatcherMsg` はまだ存在しない。本実装は [`spawn_relay`]
//! （`crate::relay`）が採る「`B: From<A>` 汎用境界」の流儀にならい、`spawn_ticker`
//! を dispatcher 側メッセージ型 `D` について汎用化した（`D: From<Tick> + Send +
//! 'static`）。task 2.5 は `DispatcherMsg` に `impl From<Tick> for DispatcherMsg`
//! を用意し `spawn_ticker::<DispatcherMsg>(...)` で結線すればよく、本ファイルの
//! シグネチャ変更は不要になる見込み（既知の将来変更を先回りして避ける選択）。

use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::time::Duration;

use areka_actor::ActorHandle;
use areka_kanade::{KanadeMsg, MonotonicMs};

/// ticker の制御メッセージ（停止規約の Close のみ）。
pub enum TickerMsg {
    /// 即時停止。
    Close,
}

/// ticker が dispatcher へ渡す 1 回分の Tick 内容。
///
/// `DispatcherMsg` が定義され次第 `impl From<Tick> for DispatcherMsg` を実装し、
/// `spawn_ticker::<DispatcherMsg>(...)` の型引数として使う想定（上記モジュール doc 参照）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick {
    pub now: MonotonicMs,
}

/// ticker 運行構成。
pub struct TickerConfig {
    /// dispatcher 向け基本周期（既定 50ms・さくらスクリプト `\w` の解像度に一致）。
    pub base_interval: Duration,
    /// kanade 向け周期（既定 1000ms・OnSecondChange の 1 秒周期）。
    pub kanade_interval: Duration,
    /// 時刻供給源（既定 `GetTickCount64`）。決定論テストは任意の単調値を返す
    /// クロージャへ差し替える。
    pub clock: Box<dyn Fn() -> MonotonicMs + Send>,
}

impl Default for TickerConfig {
    /// 既定値: `base_interval = 50ms`・`kanade_interval = 1000ms`・`clock = GetTickCount64`。
    fn default() -> Self {
        TickerConfig {
            base_interval: Duration::from_millis(50),
            kanade_interval: Duration::from_millis(1000),
            clock: Box::new(real_clock),
        }
    }
}

/// 実クロック（`GetTickCount64` の安全な薄いラッパ）。OS 起動からの経過ミリ秒を返す。
fn real_clock() -> MonotonicMs {
    // SAFETY: GetTickCount64 は引数を取らず、ポインタや所有権を伴わない単純な
    // カウンタ読取であり、呼出規約上の前提条件はない（Win32 リファレンス通り）。
    let ticks = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() };
    MonotonicMs(ticks)
}

/// 1 系統ぶんの絶対グリッド整列状態（発火判定の純粋層・スレッド／mpsc／実クロック非依存）。
///
/// グリッドは clock ゼロ起点の絶対倍数（`interval_ms` の倍数: 0, interval, 2*interval, ...）。
/// 複数の `BoundarySchedule` インスタンス（別 ticker・別ゴースト）が同じ `interval_ms` を
/// 使えば、共有コンポーネント無しで同一グリッドに自然整列する。
#[derive(Debug, Clone, Copy)]
pub(crate) struct BoundarySchedule {
    interval_ms: u64,
    next_deadline_ms: u64,
}

/// [`BoundarySchedule::poll`] の判定結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoundaryPoll {
    /// この呼出で境界に到達し発火したか。
    pub fired: bool,
    /// 発火に伴い複数境界をスキップした（catch-up）か。`fired == false` なら常に `false`。
    pub catch_up: bool,
}

impl BoundarySchedule {
    /// `now` 以降で最初に来る境界（`now` より厳密に未来の最小グリッド倍数）から開始する。
    ///
    /// `now` がちょうど境界上であっても次の境界へ進める（起動直後の即時発火を避け、
    /// 常に「次はまだ来ていない未来の境界」を保つ不変条件）。
    ///
    /// # Panics
    /// `interval` がゼロの場合（境界グリッドが定義できない）。
    pub(crate) fn starting_at(interval: Duration, now: MonotonicMs) -> Self {
        let interval_ms = duration_to_millis(interval);
        assert!(interval_ms > 0, "ticker interval must be positive");
        BoundarySchedule {
            interval_ms,
            next_deadline_ms: next_grid_multiple_strictly_after(interval_ms, now.0),
        }
    }

    /// 次の境界までの残り時間（`now` が既に境界へ到達済みなら `Duration::ZERO`）。
    pub(crate) fn remaining(&self, now: MonotonicMs) -> Duration {
        if now.0 >= self.next_deadline_ms {
            Duration::ZERO
        } else {
            Duration::from_millis(self.next_deadline_ms - now.0)
        }
    }

    /// `now` が次境界へ到達済みかを判定し、到達していれば**1 回だけ**発火させて
    /// 次デッドラインを `now` より厳密に未来のグリッド倍数へスナップする
    /// （複数境界を跨いでいてもバーストで打ち返さない＝catch-up 政策）。
    pub(crate) fn poll(&mut self, now: MonotonicMs) -> BoundaryPoll {
        if now.0 < self.next_deadline_ms {
            return BoundaryPoll {
                fired: false,
                catch_up: false,
            };
        }
        let old_deadline_ms = self.next_deadline_ms;
        let new_deadline_ms = next_grid_multiple_strictly_after(self.interval_ms, now.0);
        self.next_deadline_ms = new_deadline_ms;

        // 通常の定刻発火は「境界1個分」進む。2個分以上進んでいれば途中の境界を
        // 跨いで（＝スキップして）1回にまとめて発火した catch-up。
        let boundaries_advanced = (new_deadline_ms - old_deadline_ms) / self.interval_ms;
        BoundaryPoll {
            fired: true,
            catch_up: boundaries_advanced > 1,
        }
    }
}

fn duration_to_millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// `interval_ms` の倍数のうち `now_ms` より厳密に大きい最小値を返す。
///
/// 例: `interval_ms = 1000`, `now_ms = 0` → `1000`／`now_ms = 1000` → `2000`／
/// `now_ms = 1500` → `2000`。
fn next_grid_multiple_strictly_after(interval_ms: u64, now_ms: u64) -> u64 {
    (now_ms / interval_ms + 1) * interval_ms
}

/// ticker を起動する。`config.clock` から得た時刻を絶対グリッドへ整列させ、
/// `kanade` へ `kanade_interval` 周期・`dispatcher` へ `base_interval` 周期で
/// Tick を配る。`TickerMsg::Close` 受領または制御チャンネル切断（`stop_rx`
/// disconnected）で停止する。
///
/// `dispatcher` の要素型 `D` は [`Tick`] から `From` 変換できればよい（`DispatcherMsg`
/// 未確定を吸収する forward-compat な汎用境界・モジュール doc 参照）。
pub fn spawn_ticker<D>(
    config: TickerConfig,
    kanade: Sender<KanadeMsg>,
    dispatcher: Sender<D>,
) -> (Sender<TickerMsg>, ActorHandle)
where
    D: From<Tick> + Send + 'static,
{
    let TickerConfig {
        base_interval,
        kanade_interval,
        clock,
    } = config;

    areka_actor::spawn_actor::<TickerMsg, _>("ticker", move |stop_rx| {
        let start_now = clock();
        let mut dispatcher_schedule = BoundarySchedule::starting_at(base_interval, start_now);
        let mut kanade_schedule = BoundarySchedule::starting_at(kanade_interval, start_now);

        // 送出先切断は対象ごとに sticky（一度切れたら以後その対象へは送らない）。
        let mut dispatcher_disconnected = false;
        let mut kanade_disconnected = false;

        loop {
            let now = clock();
            let remaining = dispatcher_schedule
                .remaining(now)
                .min(kanade_schedule.remaining(now));

            match stop_rx.recv_timeout(remaining) {
                Ok(TickerMsg::Close) => return,
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {
                    let now = clock();

                    let dispatcher_poll = dispatcher_schedule.poll(now);
                    if dispatcher_poll.fired {
                        if dispatcher_poll.catch_up {
                            tracing::info!(
                                target = "dispatcher",
                                "ticker catch-up: skipped multiple boundaries, firing once"
                            );
                        }
                        if !dispatcher_disconnected {
                            let tick: D = D::from(Tick { now });
                            if dispatcher.send(tick).is_err() {
                                dispatcher_disconnected = true;
                                tracing::info!(
                                    target = "dispatcher",
                                    "ticker: dispatcher disconnected; stopping ticks to this target"
                                );
                            }
                        }
                    }

                    let kanade_poll = kanade_schedule.poll(now);
                    if kanade_poll.fired {
                        if kanade_poll.catch_up {
                            tracing::info!(
                                target = "kanade",
                                "ticker catch-up: skipped multiple boundaries, firing once"
                            );
                        }
                        if !kanade_disconnected {
                            if kanade.send(KanadeMsg::Tick { now }).is_err() {
                                kanade_disconnected = true;
                                tracing::info!(
                                    target = "kanade",
                                    "ticker: kanade disconnected; stopping ticks to this target"
                                );
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
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
        assert_eq!(schedule.remaining(MonotonicMs(0)), Duration::from_millis(1000));
        assert_eq!(schedule.remaining(MonotonicMs(400)), Duration::from_millis(600));
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
        assert_eq!(schedule.remaining(MonotonicMs(0)), Duration::from_millis(50));
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

    #[test]
    fn real_clock_returns_monotonic_ms_from_get_tick_count64() {
        // 実クロックの往復確認: 2 回読んで非減少であることのみ検証する（実 OS 時計は
        // テスト側で制御できないため、値そのものではなく単調性のみを確認する）。
        let first = real_clock();
        let second = real_clock();
        assert!(second.0 >= first.0, "GetTickCount64 must be non-decreasing");
    }
}
