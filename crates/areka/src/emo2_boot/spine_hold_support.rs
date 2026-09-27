//! 偽の SHIORI（[`ScriptedShioriBackend`]）の「解かれるまで固まる」台本（要件 7.1）。
//!
//! 指定した呼び出し（GET・NOTIFY の id 指定・UNLOAD）に入ると、外から解く手（`unblock_handle`）が
//! 呼ばれるまで待ち、解かれたら失敗を返す。
//!
//! - GET・NOTIFY は `RequestError::Timeout` を返す。本番で往復の最中に補助プロセスを終わらせると
//!   `send_request` が `IpcError::Timeout` を返し、kanade にはこの形で見える（本番と一致）。
//! - UNLOAD は `ShutdownError::ExitTimeout` を返すが、これは**代用**である。本番の
//!   `request_clean_shutdown` は UNLOAD の最中に終わらせられると
//!   `ShutdownError::Unload(SendError::Ipc(IpcError::Timeout))` を返し（ack の後なら `status()` が
//!   `Exited` で `Ok`）、`ExitTimeout` にはならない。areka は `shiori-host32-ipc` に依存せず
//!   `IpcError` を作れないので、作れる `ShutdownError` の値を使う。kanade は降ろし中の `Err` を
//!   種類に依らず同じ道（`unload_failed` の `error!` → `Stopped`）で扱うので、踏む道は本番と同じ。
//!
//! 固まるのは台本 1 件につき最初の 1 回だけで、固まった呼び出しは台本の応答を
//! 消費しない（2 回目以降の同じ呼び出しは台本どおりに返る）。解く手が先に呼ばれていれば待たずに
//! 通る（順序に依らない）。
//!
//! 本体（`spine.rs`）が 1,000 行の目安に近いので、台本の部品はこの兄弟ファイルに置く。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use areka_kanade::ShioriUnblock;
use shiori_host32_host::{RequestError, ShutdownError};

use super::{SPIN_WAIT, ScriptedShioriBackendBuilder, ScriptedShioriHandle};

/// どの呼び出しで固まるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoldAt {
    /// この id の GET（応答を要するイベント）。
    Get(&'static str),
    /// この id の NOTIFY（片道イベント）。
    Notify(&'static str),
    /// UNLOAD（降ろす往復）。
    Unload,
}

/// 固まる往復と解く手が共有する状態。
struct GateState {
    /// まだ固まっていない台本（固まったら `None`＝固まるのは 1 回だけ）。
    at: Option<HoldAt>,
    /// 解く手が呼ばれたか（一度立てたら下ろさない＝先に解かれていれば待たずに通る）。
    released: bool,
    /// いま固まっているか（テストが解く前に固まりを待つための口）。
    holding: bool,
}

/// 固まる往復（shiori のアクタースレッド）と解く手（別スレッド）の置き場。
pub(crate) struct HoldGate {
    state: Mutex<GateState>,
    wake: Condvar,
    unblock_calls: AtomicUsize,
}

impl HoldGate {
    pub(super) fn new(at: Option<HoldAt>) -> Self {
        Self {
            state: Mutex::new(GateState {
                at,
                released: false,
                holding: false,
            }),
            wake: Condvar::new(),
            unblock_calls: AtomicUsize::new(0),
        }
    }

    /// 台本が `is_it` に当たれば、解かれるまで待って `true` を返す（当たらなければ `false`）。
    ///
    /// 待ちは条件変数で、上限は [`SPIN_WAIT`] で有界にしてある: 解き忘れたテストがテストの走行
    /// ごと止まらないようにするため（ハーネスのほかの待ちと同じ猶予）。上限に達したら台本の誤りとして
    /// panic する（台本の不足で panic する既存の枝と同じ扱い）。
    fn hold_if(&self, is_it: impl FnOnce(HoldAt) -> bool) -> bool {
        let mut state = self.state.lock().expect("hold gate poisoned");
        if !state.at.is_some_and(is_it) {
            return false;
        }
        state.at = None;
        if !state.released {
            state.holding = true;
            let (mut state, waited) = self
                .wake
                .wait_timeout_while(state, SPIN_WAIT, |s| !s.released)
                .expect("hold gate poisoned");
            state.holding = false;
            // Mutex を poison させないよう、panic の前に錠を放す（後続の `holding()` の二次 panic を避ける）。
            drop(state);
            assert!(
                !waited.timed_out(),
                "ScriptedShioriBackend: the held call was never unblocked within {SPIN_WAIT:?}"
            );
        }
        true
    }

    /// GET の入口。固まる台本に当たれば、解かれた後に期限切れの失敗を返す。
    pub(super) fn pass_get(&self, id: &str) -> Result<(), RequestError> {
        match self.hold_if(|at| matches!(at, HoldAt::Get(h) if h == id)) {
            true => Err(RequestError::Timeout),
            false => Ok(()),
        }
    }

    /// NOTIFY の入口。固まる台本に当たれば、解かれた後に期限切れの失敗を返す。
    pub(super) fn pass_notify(&self, id: &str) -> Result<(), RequestError> {
        match self.hold_if(|at| matches!(at, HoldAt::Notify(h) if h == id)) {
            true => Err(RequestError::Timeout),
            false => Ok(()),
        }
    }

    /// UNLOAD の入口。固まる台本に当たれば、解かれた後に `ExitTimeout` を返す（本番の
    /// `Unload(SendError::Ipc(IpcError::Timeout))` の代用・モジュールの説明を参照）。
    pub(super) fn pass_unload(&self) -> Result<(), ShutdownError> {
        match self.hold_if(|at| at == HoldAt::Unload) {
            true => Err(ShutdownError::ExitTimeout),
            false => Ok(()),
        }
    }

    /// 解く手（別スレッドから呼ぶ）。呼ばれた回数を数え、固まっている往復を起こす。
    pub(super) fn unblock(self: &Arc<Self>) -> ShioriUnblock {
        let gate = Arc::clone(self);
        Arc::new(move || {
            gate.unblock_calls.fetch_add(1, Ordering::SeqCst);
            gate.state.lock().expect("hold gate poisoned").released = true;
            gate.wake.notify_all();
            Ok(())
        })
    }
}

impl ScriptedShioriBackendBuilder {
    /// 指定した呼び出しで解かれるまで固まる台本を足す（1 台本につき 1 か所・最初の 1 回だけ）。
    pub(crate) fn hold_at(mut self, at: HoldAt) -> Self {
        self.hold = Some(at);
        self
    }
}

impl ScriptedShioriHandle {
    /// いま固まっているか（解く手を呼ぶ前に固まりを待つ口）。
    pub(crate) fn holding(&self) -> bool {
        self.gate.state.lock().expect("hold gate poisoned").holding
    }

    /// 解く手が呼ばれた回数。
    pub(crate) fn unblock_calls(&self) -> usize {
        self.gate.unblock_calls.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
#[path = "spine_hold_tests.rs"]
mod tests;
