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

/// ループ tick レーン構成（既定: 16ms・`GetTickCount64`）。
///
/// [`spawn_ticker`] の 2 系統（dispatcher 50ms／kanade 1000ms）とは独立した、
/// SERIKO ループ評価専用の単発 tick レーン（[`spawn_loop_ticker`]）の運行構成。
/// 16ms は 60Hz 近似で、`\w` 系の最小 wait（22ms）を 1 tick 以内で拾える解像度
/// （design.md「spawn_loop_ticker」）。
pub struct LoopTickerConfig {
    /// ループ評価の基本周期（既定 16ms）。
    pub interval: Duration,
    /// 時刻供給源（既定 `GetTickCount64`）。決定論テストは任意の単調値を返すクロージャへ
    /// 差し替える。
    pub clock: Box<dyn Fn() -> MonotonicMs + Send>,
}

impl Default for LoopTickerConfig {
    /// 既定値: `interval = 16ms`・`clock = GetTickCount64`。
    fn default() -> Self {
        LoopTickerConfig {
            interval: Duration::from_millis(16),
            clock: Box::new(real_clock),
        }
    }
}

/// SERIKO ループ評価向けの単発 tick レーンを起動する。
///
/// [`spawn_ticker`] と同じ [`BoundarySchedule`]（絶対グリッド整列・catch-up 1 回）を
/// **再利用**し、単一系統として `config.clock` を絶対グリッドへ整列させる。グリッド境界へ
/// 到達するたびに `deliver` を**ちょうど 1 回**呼ぶ（複数境界を跨いだ大幅遅延でも catch-up
/// 政策により 1 回に畳む）。`TickerMsg::Close` 受領または制御チャンネル切断（`stop_rx`
/// disconnected）で正常終了する（design.md「spawn_loop_ticker」・R1.1/1.3/1.4）。
///
/// 配送は **クロージャ**（`Box<dyn FnMut(Tick) + Send>`）経由で行い、`From<Tick>` による
/// 型付きチャネル境界（[`spawn_ticker`] の流儀）は使わない。これは意図的な設計判断で、
/// ghost が seriko の型を知らずに済ませる（orphan rule 回避・型結合ゼロ・design D-1
/// 「areka-ghost は seriko に依存しない」）。配送失敗の観測は closure 側の責務とする。
///
/// worker スレッド駆動＝表示状態・vsync に非従属（R1.3）。既存 [`spawn_ticker`]・
/// [`TickerConfig`]・2 系統の挙動／シグネチャは一切変更しない純粋な additive 追加（R1.4）。
pub fn spawn_loop_ticker(
    config: LoopTickerConfig,
    mut deliver: Box<dyn FnMut(Tick) + Send>,
) -> (Sender<TickerMsg>, ActorHandle) {
    let LoopTickerConfig { interval, clock } = config;

    areka_actor::spawn_actor::<TickerMsg, _>("loop-ticker", move |stop_rx| {
        let start_now = clock();
        let mut schedule = BoundarySchedule::starting_at(interval, start_now);

        loop {
            let now = clock();
            let remaining = schedule.remaining(now);

            match stop_rx.recv_timeout(remaining) {
                Ok(TickerMsg::Close) => return,
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {
                    let now = clock();
                    let poll = schedule.poll(now);
                    if poll.fired {
                        if poll.catch_up {
                            tracing::info!(
                                target = "loop_ticker",
                                "loop ticker catch-up: skipped multiple boundaries, firing once"
                            );
                        }
                        // 発火ごとに 1 回だけ配送（catch-up 時も 1 回）。配送失敗の扱いは
                        // closure の内部責務。
                        deliver(Tick { now });
                    }
                }
            }
        }
    })
}

#[cfg(test)]
#[path = "ticker_tests.rs"]
mod tests;
