//! 出力結線（sink）— 2 系統の出力先 trait 抽象とテスト用 mock。
//!
//! surface 系（→seriko⑤）と text 系（→emo text-layer⑥）を **型で分離**した 2 つの
//! 出力先抽象を宣言する（R3.3）。受け渡し単位は [`TalkCue`]（`at`・`actor` 込み＝
//! R9.2 の発火時刻観測を含む・R4.1）。本層は trait と決定的観測用 mock のみを持ち、
//! 本番の seriko/emo inbox への送出アダプタは後続 spec の領分。

use crate::contract::TalkCue;
use std::sync::{Arc, Mutex};

/// surface 系（→seriko⑤）の出力先抽象。text 系とは**別 trait**で 2 分岐を型で分離する（R3.3）。
///
/// `emit` は infallible（`Result` 化しない）。本番アダプタは送出失敗を `tracing::error!` で
/// 観測する（R11.1・後続 spec の領分）。
pub trait SurfaceSink {
    /// 1 発火を surface 側へ届ける（`TalkCue` は `at`・`actor` 込み）。
    fn emit(&mut self, cue: TalkCue);
}

/// text 系（→emo text-layer⑥）の出力先抽象。surface 系とは**別 trait**で 2 分岐を型で分離する（R3.3）。
///
/// `emit` は infallible（`SurfaceSink` と同契約）。
pub trait TextSink {
    /// 1 発火を text 側へ届ける（`TalkCue` は `at`・`actor` 込み＝R9.2 の発火時刻観測を含む）。
    fn emit(&mut self, cue: TalkCue);
}

/// テスト用 mock（surface/text 共用の実装を型別名で 2 本立てる）。
///
/// 発火を `Arc<Mutex<Vec<TalkCue>>>` の共有蓄積へ push し、[`MockSink::records`] が返す
/// Arc クローンを通じて、アクタースレッドへ move した後もテスト側が発火列・`at`（発火時刻）を
/// 照合できる（R9.2）。`Send + 'static`（`Arc<Mutex<..>>` は `Send + Sync`・`TalkCue` は `Send`）。
pub struct MockSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl MockSink {
    /// 空の共有蓄積を持つ mock を生成する。
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 共有蓄積の Arc クローンを返す。MockSink をアクタースレッドへ move した後も、
    /// テストスレッドがこのハンドルから発火列・発火時刻を照合できる。
    pub fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }

    /// 内部: 発火を共有蓄積へ FIFO 追記する（両 trait の emit が共用）。
    fn push(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("MockSink records mutex poisoned")
            .push(cue);
    }
}

impl Default for MockSink {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceSink for MockSink {
    fn emit(&mut self, cue: TalkCue) {
        self.push(cue);
    }
}

impl TextSink for MockSink {
    fn emit(&mut self, cue: TalkCue) {
        self.push(cue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ActorKey, CueCommand};

    /// 生成元スレッドとは別のスレッドで複数の発火を mock へ送った後、
    /// テストスレッドが clone した records ハンドルから**発火順どおりに全件**
    /// （発火列＝command と発火時刻＝at）を照合できることを検証する（R9.2）。
    #[test]
    fn mock_sink_records_are_readable_cross_thread_in_fifo_order() {
        let sink = MockSink::new();
        // records() は Arc クローンを返す＝MockSink を別スレッドへ move した後も保持できる。
        let handle: Arc<Mutex<Vec<TalkCue>>> = sink.records();

        // 発火列（distinct な at・command）。emission order = FIFO を照合対象にする。
        let emitted = vec![
            TalkCue {
                at: 0.0,
                actor: ActorKey::from("0"),
                command: CueCommand::Text("hello".into()),
                duration: 0.0,
            },
            TalkCue {
                at: 1.5,
                actor: ActorKey::from("0"),
                command: CueCommand::Emote { key: "smile".into() },
                duration: 0.0,
            },
            TalkCue {
                at: 3.25,
                actor: ActorKey::from("1"),
                command: CueCommand::Text("world".into()),
                duration: 0.0,
            },
        ];

        // MockSink を生成元と別スレッドへ move して発火させる。
        let emitted_for_thread = emitted.clone();
        let producer = std::thread::spawn(move || {
            // SurfaceSink trait 経由で emit（surface スロット相当の発火経路）。
            let mut sink: MockSink = sink;
            for cue in emitted_for_thread {
                SurfaceSink::emit(&mut sink, cue);
            }
        });
        producer.join().expect("producer thread panicked");

        // 生成元と別スレッド（元スレッド）から clone ハンドルで照合する。
        let recorded = handle.lock().expect("records mutex poisoned");
        assert_eq!(recorded.len(), emitted.len(), "全件取得できること");
        // 発火順（FIFO）どおりに at・command が一致すること（TalkCue の PartialEq で照合）。
        assert_eq!(&*recorded, &emitted, "発火列と発火時刻が発火順で一致すること");
    }

    /// 同一 MockSink 型が `TextSink` としても使えること（型別名 2 本立ての実装確認）。
    #[test]
    fn mock_sink_works_via_text_sink_trait() {
        let mut sink = MockSink::new();
        let handle = sink.records();
        let cue = TalkCue {
            at: 2.0,
            actor: ActorKey::from("0"),
            command: CueCommand::NewLine { ratio: 1.0 },
            duration: 0.0,
        };
        TextSink::emit(&mut sink, cue.clone());
        assert_eq!(&*handle.lock().unwrap(), &vec![cue]);
    }
}
