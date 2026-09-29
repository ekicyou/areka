//! 終了の後始末が、背景で書いている仕事の終わりを上限つきで待つ口（design「areka / 終了 / exit_wait」）。
//!
//! 背景の仕事は 1 本ごとに門 [`WorkGate`] を持ち、[`register_gate`] で World へ登記する。
//! 終了が始まったら [`begin_close`] が全部の門を閉じ、登記された片付けを UI スレッドで呼ぶ。
//! 書く前の段に居た仕事は待たずに記録を残し、書いている最中の仕事だけを
//! [`ClosingWaits::wait`] が予算の残りの時間だけ待つ。
//!
//! この部品は `install` も `update` も知らない。どちらの背景の仕事も同じ口で待ち（要件 8.10）、
//! 書きかけの場所は門の `label`（書庫 → 根／更新先 → 対象）が運ぶ。

use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use areka_kanade::WaitBudget;
use bevy_ecs::prelude::*;

/// 展開の終わりを待つ上限（要件 8.1・裁定 4）。
pub(crate) const EXIT_WAIT_LIMIT: Duration = Duration::from_secs(3);

/// 門の状態。3 つを 1 つの鍵の下で持つので、終了が始まった後に書く段へ入ることはない。
#[derive(Default)]
struct GateState {
    /// 今扱っている物の名前（記録用）。何も扱っていなければ None。
    label: Option<String>,
    /// 書いている最中か。
    writing: bool,
    /// 終了が始まったか。
    closing: bool,
}

/// 背景の仕事 1 本ぶんの門（複製して持つのは `Arc`）。
#[derive(Default)]
pub(crate) struct WorkGate {
    state: Mutex<GateState>,
    /// 書く段を出たことの合図。
    left: Condvar,
}

impl WorkGate {
    fn lock(&self) -> MutexGuard<'_, GateState> {
        // 鍵を持ったまま倒れたスレッドが居ても、状態は 3 つの欄の代入だけなので読み続けてよい。
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 今扱っている物の名前を置く（記録用）。終了が始まっていれば偽。
    pub(crate) fn begin(&self, label: String) -> bool {
        let mut s = self.lock();
        if s.closing {
            return false;
        }
        s.label = Some(label);
        true
    }

    /// 書く段へ入る。終了が始まっていれば偽（入らない）。
    pub(crate) fn enter_write(&self) -> bool {
        let mut s = self.lock();
        if s.closing {
            return false;
        }
        s.writing = true;
        true
    }

    /// 書く段を出る。待っている後始末を起こす。
    pub(crate) fn leave_write(&self) {
        self.lock().writing = false;
        self.left.notify_all();
    }

    /// 扱っていた物を手放す（書く段に居れば出る）。
    pub(crate) fn end(&self) {
        let mut s = self.lock();
        s.label = None;
        s.writing = false;
        drop(s);
        self.left.notify_all();
    }

    pub(crate) fn is_closing(&self) -> bool {
        self.lock().closing
    }
}

/// 登記された門と片付け。
struct GateEntry {
    name: &'static str,
    gate: Arc<WorkGate>,
    on_close: fn(&mut World),
}

/// 門の登記の表（UI スレッドの World に 1 つ）。
#[derive(Resource, Default)]
struct ExitGates(Vec<GateEntry>);

/// 門と、終了が始まったときに UI スレッドで呼ぶ片付けを登記する。
pub(crate) fn register_gate(
    world: &mut World,
    name: &'static str,
    gate: Arc<WorkGate>,
    on_close: fn(&mut World),
) {
    world
        .get_resource_or_insert_with(ExitGates::default)
        .0
        .push(GateEntry {
            name,
            gate,
            on_close,
        });
}

/// 閉じた時点で書いている最中だった仕事。
struct WritingWork {
    name: &'static str,
    label: Option<String>,
    gate: Arc<WorkGate>,
}

/// 閉じた門のうち、書いている最中だった仕事。
pub(crate) struct ClosingWaits(Vec<WritingWork>);

/// 終了が始まった: 全部の門を閉じ、登記された片付けを呼び、その時点の仕事を記録に残す。
pub(crate) fn begin_close(world: &mut World) -> ClosingWaits {
    let entries = world
        .get_resource_mut::<ExitGates>()
        .map(|mut gates| std::mem::take(&mut gates.0))
        .unwrap_or_default();

    let mut writing = Vec::new();
    for entry in &entries {
        let mut s = entry.gate.lock();
        s.closing = true;
        if s.writing {
            writing.push(WritingWork {
                name: entry.name,
                label: s.label.clone(),
                gate: entry.gate.clone(),
            });
        } else if let Some(label) = &s.label {
            tracing::warn!(
                event = "exit_wait_abandoned",
                name = entry.name,
                label = label.as_str(),
                "[exit_wait] 書く前の段で終了が始まった——待たずに途中でやめる"
            );
        }
    }
    for entry in &entries {
        (entry.on_close)(world);
    }
    ClosingWaits(writing)
}

impl ClosingWaits {
    /// 書いている最中の門だけを、`budget` の残りの時間だけ待つ。
    ///
    /// 期限は `budget.started + budget.limit`（今から数え直さない）。前の段と同じ出発点を渡せば、
    /// 合わせて上限を超えない（要件 8.2）。
    pub(crate) fn wait(self, budget: WaitBudget) {
        let deadline = budget.started + budget.limit;
        let waited_from = Instant::now();
        for work in self.0 {
            let mut s = work.gate.lock();
            while s.writing {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                s = work
                    .gate
                    .left
                    .wait_timeout(s, left)
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
            let label = work.label.as_deref().unwrap_or("");
            if s.writing {
                tracing::warn!(
                    event = "exit_wait_timeout",
                    name = work.name,
                    label,
                    "[exit_wait] 上限に達したので待つのをやめた——取得か書き込みの途中だった。書きかけの物が作業場所に残っているかもしれない（label を見よ）"
                );
            } else {
                tracing::info!(
                    event = "exit_wait_done",
                    name = work.name,
                    label,
                    ms = waited_from.elapsed().as_millis() as u64,
                    "[exit_wait] 書き終わりを待った"
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "exit_wait_tests.rs"]
mod tests;
