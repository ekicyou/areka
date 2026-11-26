//! `CuePlayer`（受動的注入時刻ランタイム）の状態機械骨格の檻。
//!
//! cue 再生の状態管理（再生中・入力待ち・選択肢待ち・完了）が、外部解決待ちの停止点（バリア）と選択肢の先積みを持つことを公開 API 越しに固定する。
//!
//! 主 observable: **バリアに到達すると停止し、外部からの解決通知で再開する**（バリア手前の
//! cue は配送されるが、バリア以降の cue は解決通知が来るまで配送されない）。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePlayer, CuePlayerState, CueSheet, CueSink, TalkCue,
};
use dola::time::TimePoint;
use std::cell::RefCell;
use std::rc::Rc;

/// 起点 0 に刻印した台本の組み立て（刻印が無いと再生機は組めない）。
fn sheet_at_zero(cues: Vec<Cue>) -> CueSheet {
    CueSheet::new(cues).with_absolute_start_time(TimePoint::ZERO)
}

/// ミリ秒の絶対時刻（100ns の刻みへ ちょうどの値で）。
fn at_ms(millis: i64) -> TimePoint {
    TimePoint::from_ticks(millis * 10_000)
}

#[path = "runtime_test_barrier_tests.rs"]
mod barrier_tests;
#[path = "runtime_test_broadcast_tests.rs"]
mod broadcast_tests;
#[path = "runtime_test_choice_delivery_tests.rs"]
mod choice_delivery_tests;
#[path = "runtime_test_fire_time_tests.rs"]
mod fire_time_tests;
#[path = "runtime_test_late_delivery_tests.rs"]
mod late_delivery_tests;
#[path = "runtime_test_occupancy_horizon_tests.rs"]
mod occupancy_horizon_tests;
#[path = "runtime_test_origin_notice_tests.rs"]
mod origin_notice_tests;
#[path = "runtime_test_preview_tests.rs"]
mod preview_tests;
#[path = "runtime_test_reanchor_tests.rs"]
mod reanchor_tests;
