//! プロセスに 1 つの「通信中の数」（要件 2.1〜2.6）。
//!
//! 通信はゴーストでなく areka のものなので、ゴーストの切替をまたいで続く。数は
//! [`OnlineCounter::begin`] で 1 増え、返る [`OnlineGuard`] を落とすと必ず 1 減る
//! （成功・失敗・早期 return・`?`・panic のどの終わり方でも戻る＝要件 2.3）。
//! 重なっても [`OnlineCounter::is_online`] は 1 つの真偽である（要件 2.4）。
//!
//! 本番は [`PROCESS`] を使い、kanade の殻が毎メッセージ読んで運行表の写しへ写す。
//! テストは関数内の `static` で自分の数を持ち、`PROCESS` には触れない。
//! std だけに依存する葉（記録の `tracing` を除く）。

use std::sync::atomic::{AtomicUsize, Ordering};

/// 通信中の数（`AtomicUsize`）。本番はプロセスに 1 つの [`PROCESS`]、テストは関数内の `static` で自分の数を持つ。
#[derive(Debug)]
pub struct OnlineCounter(AtomicUsize);

impl OnlineCounter {
    /// 数 0 の新しい数。
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    /// 数を 1 増やし、落とすと 1 減る守り手を返す。`what` は記録用の名前（"update"・"install-fetch"）。
    pub fn begin(&'static self, what: &'static str) -> OnlineGuard {
        let count = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        tracing::info!(target: "kanade", event = "online_begin", what, count, "通信の始まり");
        OnlineGuard {
            counter: self,
            what,
        }
    }

    /// 0 より大きいか。
    pub fn is_online(&self) -> bool {
        self.0.load(Ordering::SeqCst) > 0
    }
}

impl Default for OnlineCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// 通信の区間の守り手。落とすと数を 1 減らす。
#[derive(Debug)]
#[must_use = "守り手を落とした時点で通信の区間が終わる"]
pub struct OnlineGuard {
    counter: &'static OnlineCounter,
    what: &'static str,
}

impl Drop for OnlineGuard {
    fn drop(&mut self) {
        let count = self.counter.0.fetch_sub(1, Ordering::SeqCst) - 1;
        let what = self.what;
        tracing::info!(target: "kanade", event = "online_end", what, count, "通信の終わり");
    }
}

/// 本番のプロセスに 1 つの通信中の数。
pub static PROCESS: OnlineCounter = OnlineCounter::new();

#[cfg(test)]
#[path = "online_tests.rs"]
mod tests;
