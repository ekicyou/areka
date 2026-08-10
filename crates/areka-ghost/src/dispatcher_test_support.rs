use super::*;
use areka_sakura::contract::TalkCue;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

/// task 8.3: 退役した `crate::runtime::default_system_vars()` の忠実な代役スタンドイン。
///
/// `{"username": DEFAULT_USERNAME}` のみを充填した凍結スナップショットを毎回新規構築して
/// 返す（退役前 provider と同一挙動）。`spawn_dispatcher` の刻印点は [`SystemVarSource`] のまま
/// 無改変で、既存テストは従来どおり既定 username 前提の直接注入を保つ（R7.1・R9.1）。
pub(super) fn test_system_vars() -> SystemVarSource {
    Box::new(|| {
        let mut snapshot = areka_sakura::contract::SystemVarSnapshot::default();
        snapshot.insert("username", areka_sakura::sysvar::DEFAULT_USERNAME);
        snapshot
    })
}

/// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
/// テストを失敗させる（どのテストもハングしないことを保証する・areka-actor 流儀）。
pub(super) fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
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

/// テスト専用の `Clone` 可能な記録 sink（sakura `MockSink` は `Clone` でないため、
/// dispatcher の per-talk 注入（`S: Clone`/`T: Clone`）を満たすために本モジュール限定で
/// 定義する・sakura の凍結面 `sink.rs` には手を入れない）。
#[derive(Clone)]
pub(super) struct RecordingSink {
    records: std::sync::Arc<std::sync::Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    pub(super) fn new() -> Self {
        Self {
            records: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub(super) fn records(&self) -> std::sync::Arc<std::sync::Mutex<Vec<TalkCue>>> {
        std::sync::Arc::clone(&self.records)
    }
}

// broadcast: 単一の `CueSink` として登録され、全 cue を受ける（surface/text スロットの別なく
// 両スロットが同一の全 cue を受信する）。演者側 relevance が action を選別する（本 sink は記録のみ）。
impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("records mutex poisoned")
            .push(cue);
    }
}
