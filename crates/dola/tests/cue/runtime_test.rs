//! `CuePlayer`（受動的注入時刻ランタイム）の状態機械骨格の檻。
//!
//! Task 4.2: cue 再生の状態管理（再生中・入力待ち・選択肢待ち・完了）を dola の受動的
//! ランタイムへ一本化し、外部解決待ちの停止点（バリア seam）と選択肢の先積みを、動的な
//! 一時停止/再開の状態（Non-Goals）を持ち込まずに移植したことを公開 API 越しに固定する
//! （R11.2・D6/D7）。
//!
//! 主 observable: **バリアに到達すると停止し、外部からの解決通知で再開する**（バリア手前の
//! cue は配送されるが、バリア以降の cue は解決通知が来るまで配送されない）。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CuePlayer, CuePlayerState, CueSheet,
    CueSink, TalkCue, TimedSchedule, to_talk_schedule,
};
use std::cell::RefCell;
use std::rc::Rc;

// テーマ分割（タスク 8.7・要件 1.7）: 本ファイルは module doc・共有 use・接続宣言のみを持ち、
// テスト本体はテーマ別の兄弟ファイルに在る（接続規約は design 設計判断 #1／#13）。
#[cfg(test)]
#[path = "runtime_test_barrier_tests.rs"]
mod barrier_tests;
#[cfg(test)]
#[path = "runtime_test_broadcast_tests.rs"]
mod broadcast_tests;
#[cfg(test)]
#[path = "runtime_test_choice_delivery_tests.rs"]
mod choice_delivery_tests;
#[cfg(test)]
#[path = "runtime_test_occupancy_horizon_tests.rs"]
mod occupancy_horizon_tests;
#[cfg(test)]
#[path = "runtime_test_test_support.rs"]
mod test_support;
