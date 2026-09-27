//! SHIORI の待ちを見張って外から解く部品（[`ShioriProbe`]）。
//!
//! shiori のアクターの外に置く 3 つの置き場をまとめる（プロセス内で共有・`Arc`）:
//!
//! - **今の呼び出し**（[`ShioriBusy`]）: shiori のアクターだけが [`ShioriProbe::set_busy`] で
//!   書き、見張りだけが読む。
//! - **解く手**（[`ShioriUnblock`]）: 接続の成功後に [`ShioriProbe::install_unblock`] で一度だけ
//!   据える。別スレッドから呼んで、止まっている往復を終わらせる（本番は補助プロセスを終わらせる）。
//! - **見張り**: [`ShioriProbe::arm`] が 1 本のスレッドを起こし、上限（[`WaitBudget`]）で発火する。
//!   上限の前に後始末が終われば [`CutGuard::finish`] が見張りを止める。
//!
//! 発火の口は 2 つ: 期限と、テストの手動の口 [`ShioriProbe::cut_now`]（張る前に呼べば予約として
//! 残り、次に張った時点で即発火する）。後始末の終わりと期限が同時なら、`outcome` の
//! `compare_exchange` で 1 人だけが勝つ（終わりが勝てば切らない）。
//!
//! 「打ち切った」かどうかは、SHIORI の往復が返したエラーの種類でなく、この部品の結果
//! （[`ShioriCut`]）で決める（補助プロセスを終わらせた往復は kanade には期限切れと同じ形で見える）。
//!
//! host32 の型は持たない（host32 を import してよいのは `real.rs` だけ）。環境変数は読まない。

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// 往復の外から待ちを解く手（別スレッドから呼ぶ）。失敗は理由の文字列で返す。
pub type ShioriUnblock = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

/// shiori のアクターが今どの呼び出しで待っているか。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ShioriBusy {
    /// 呼び出しの外（待っていない）。
    #[default]
    Idle,
    /// GET／NOTIFY の往復の最中（id はイベント名）。
    Request(String),
    /// SHIORI を降ろしている最中（UNLOAD の応答と補助プロセスの終了の観測）。
    Unload,
    /// 降ろす処理が戻った後（成否を問わない）。SHIORI の待ちは終わっているので見張りは切らない。
    Unloaded,
}

/// 上限の数え方: `started` から `limit` まで（後始末に入った時点から数える）。
#[derive(Debug, Clone, Copy)]
pub struct WaitBudget {
    pub started: Instant,
    pub limit: Duration,
}

/// 上限で打ち切った結果。
///
/// `stage` は `"in_flight_request"`（後始末に入る前から待っていた往復）・`"on_close_notify"`・
/// `"unload"`・`"idle"`（呼び出しの外で上限に達した）のいずれか。`unblocked` は解く手が成功したか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShioriCut {
    pub stage: &'static str,
    pub unblocked: bool,
}

/// 見張りと後始末の終わりの勝者。
const ARMED: u8 = 0;
const FINISHED: u8 = 1;
const FIRED: u8 = 2;

enum Signal {
    Done,
    CutNow,
}

/// 張られた見張りへの送信端と、張る前に呼ばれた手動の口の予約。
#[derive(Default)]
struct Armed {
    tx: Option<Sender<Signal>>,
    pending_cut: bool,
}

#[derive(Default)]
struct ProbeInner {
    busy: Mutex<ShioriBusy>,
    unblock: OnceLock<Option<ShioriUnblock>>,
    armed: Mutex<Armed>,
    outcome: AtomicU8,
}

/// 置き場の鍵。持ち手が panic しても中身は単純な値なので、そのまま読み書きを続ける。
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 今の呼び出し・解く手・見張りの共有部品（複製しても同じ置き場を指す）。
#[derive(Clone, Default)]
pub struct ShioriProbe(Arc<ProbeInner>);

impl ShioriProbe {
    /// 今の呼び出しを書く（shiori のアクターだけが呼ぶ）。
    pub fn set_busy(&self, busy: ShioriBusy) {
        *lock(&self.0.busy) = busy;
    }

    /// 解く手を据える。一度だけ効き、2 度目は無視する。
    pub fn install_unblock(&self, unblock: Option<ShioriUnblock>) {
        if self.0.unblock.set(unblock).is_err() {
            tracing::debug!(
                target: "shiori-actor",
                event = "shiori_unblock_already_installed",
                "解く手は既に据わっている——2 度目は無視する"
            );
        }
    }

    /// 見張りを張る。`budget.started` からの残り時間で待ち、期限か手動の口で発火する。
    pub fn arm(&self, budget: WaitBudget) -> CutGuard {
        let (tx, rx) = mpsc::channel();
        self.0.outcome.store(ARMED, Ordering::Release);
        {
            let mut armed = lock(&self.0.armed);
            if std::mem::take(&mut armed.pending_cut) {
                let _ = tx.send(Signal::CutNow);
            }
            armed.tx = Some(tx.clone());
        }
        let inner = Arc::clone(&self.0);
        let spawned = std::thread::Builder::new()
            .name("shiori-watchdog".into())
            .spawn(move || {
                let wait = budget.limit.saturating_sub(budget.started.elapsed());
                match rx.recv_timeout(wait) {
                    Ok(Signal::CutNow) | Err(RecvTimeoutError::Timeout) => try_fire(&inner, budget),
                    Ok(Signal::Done) | Err(RecvTimeoutError::Disconnected) => None,
                }
            });
        let handle = match spawned {
            Ok(handle) => Some(handle),
            Err(error) => {
                tracing::error!(
                    target: "shiori-actor",
                    event = "shiori_watchdog_spawn_failed",
                    error = %error,
                    "見張りのスレッドを起こせなかった——上限で打ち切れない"
                );
                None
            }
        };
        CutGuard {
            probe: Arc::clone(&self.0),
            tx,
            handle,
            stopped: false,
        }
    }

    /// テストの手動の口。張られた見張りを今すぐ起こす。張られていなければ予約として残し、
    /// 次に張った時点で即発火させる（呼ぶ順序に依らない）。本番コードは呼ばない。
    pub fn cut_now(&self) {
        let mut armed = lock(&self.0.armed);
        match &armed.tx {
            Some(tx) => {
                let _ = tx.send(Signal::CutNow);
            }
            None => armed.pending_cut = true,
        }
    }
}

/// 張った見張りの持ち手。[`CutGuard::finish`] で見張りを止めて結果を受け取る。
pub struct CutGuard {
    probe: Arc<ProbeInner>,
    tx: Sender<Signal>,
    handle: Option<JoinHandle<Option<ShioriCut>>>,
    stopped: bool,
}

impl CutGuard {
    /// 後始末の終わり。見張りより先なら切らせずに止め、見張りが先に発火していればその結果を
    /// 待って返す。見張りのスレッドが終わってから戻る。
    pub fn finish(mut self) -> Option<ShioriCut> {
        self.stop()
    }

    /// 見張りを畳む（`finish` と `Drop` の共通・2 度目は何もしない）。
    fn stop(&mut self) -> Option<ShioriCut> {
        if std::mem::replace(&mut self.stopped, true) {
            return None;
        }
        lock(&self.probe.armed).tx = None;
        if self
            .probe
            .outcome
            .compare_exchange(ARMED, FINISHED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let _ = self.tx.send(Signal::Done);
        }
        match self.handle.take()?.join() {
            Ok(cut) => cut,
            Err(_) => {
                tracing::error!(
                    target: "shiori-actor",
                    event = "shiori_watchdog_panicked",
                    "見張りのスレッドが panic した——打ち切りの結果は不明として扱う"
                );
                None
            }
        }
    }
}

/// `finish` せずに落とされた持ち手も見張りを畳む（古い見張りが期限まで生き残って、次に張った
/// 回を切らないように）。
impl Drop for CutGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 見張りの決め手。後始末の終わりと取り合って勝ったときだけ、段を決めて 1 回だけ解く。
fn try_fire(inner: &ProbeInner, budget: WaitBudget) -> Option<ShioriCut> {
    if inner
        .outcome
        .compare_exchange(ARMED, FIRED, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return None;
    }
    let limit_ms = budget.limit.as_millis() as u64;
    let elapsed_ms = budget.started.elapsed().as_millis() as u64;
    let busy = lock(&inner.busy).clone();
    let (stage, id) = match busy {
        ShioriBusy::Unloaded => {
            tracing::debug!(
                target: "shiori-actor",
                event = "shiori_wait_limit_after_unload",
                limit_ms,
                elapsed_ms,
                "上限に達したが SHIORI は既に降りている——切らない"
            );
            return None;
        }
        ShioriBusy::Request(id) if id == "OnClose" => ("on_close_notify", Some(id)),
        ShioriBusy::Request(id) => ("in_flight_request", Some(id)),
        ShioriBusy::Unload => ("unload", None),
        ShioriBusy::Idle => ("idle", None),
    };
    let unblocked = match inner.unblock.get() {
        Some(Some(unblock)) => match unblock() {
            Ok(()) => true,
            Err(error) => {
                tracing::error!(
                    target: "shiori-actor",
                    event = "shiori_unblock_failed",
                    error = %error,
                    stage,
                    "補助プロセスを終わらせられなかった"
                );
                false
            }
        },
        _ => {
            tracing::error!(
                target: "shiori-actor",
                event = "shiori_unblock_unavailable",
                stage,
                "外から解く手が無い（接続前か InProc）——待ちは今日どおり続く"
            );
            false
        }
    };
    tracing::warn!(
        target: "shiori-actor",
        event = "shiori_wait_cut",
        stage,
        id = ?id,
        limit_ms,
        elapsed_ms,
        unblocked,
        "SHIORI の待ちを上限で打ち切った——補助プロセスを終わらせて待ちを解く"
    );
    Some(ShioriCut { stage, unblocked })
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod tests;
