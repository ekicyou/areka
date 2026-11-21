//! `CuePlayer`（受動的注入時刻ランタイム）の状態機械骨格の檻。
//!
//! cue 再生の状態管理（再生中・入力待ち・選択肢待ち・完了）が、外部解決待ちの停止点（バリア）と選択肢の先積みを持つことを公開 API 越しに固定する。
//!
//! 主 observable: **バリアに到達すると停止し、外部からの解決通知で再開する**（バリア手前の
//! cue は配送されるが、バリア以降の cue は解決通知が来るまで配送されない）。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePlayer, CuePlayerState, CueSheet, CueSink, TalkCue,
};
use std::cell::RefCell;
use std::rc::Rc;

#[path = "runtime_test_barrier_tests.rs"]
mod barrier_tests;
#[path = "runtime_test_broadcast_tests.rs"]
mod broadcast_tests;
#[path = "runtime_test_choice_delivery_tests.rs"]
mod choice_delivery_tests;
#[path = "runtime_test_occupancy_horizon_tests.rs"]
mod occupancy_horizon_tests;
#[path = "runtime_test_preview_tests.rs"]
mod preview_tests;
