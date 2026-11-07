//! 観測ハーネス基盤（設計「検証層 → 観測ハーネス（tests/kanade/）」）。
//!
//! 統合テストが運行表を決定的に観測できるよう、以下を提供する:
//!
//! - [`RecordedCall`]: 受理された shiori 呼出（Method・id・References）の記録単位。
//! - [`spawn_mock_shiori`]: real と**同一の [`areka_kanade::ShioriMsg`] 型**を受け、
//!   [`Fixture`] 表に従い同梱 `reply` へ**即時**応答する mock shiori アクター
//!   （trait 不要＝Req 5.1 の型レベル差し替え）。受理呼出を [`RecordedCall`] 列へ蓄積。
//! - [`spawn_mock_sakura`]: [`TalkCommand`] を到着順に受領・記録し、`Start` についてシナリオ指示（quit true/false・
//!   遅延なし）どおり [`KanadeMsg::TalkDone`] を kanade inbox へ返す mock 再生系宛先。
//! - [`expected_call`]: `areka_kanade::events::*` の [`ShioriCall`] を [`RecordedCall`] へ
//!   変換する導出関数（fixture・検証・実装が単一の正本＝events 表を共有・Req 7.1）。
//! - [`join_bounded`]: 期限付き待機によるハング検出ヘルパ（どのテストもハングしない・Req 7.3）。
//! - [`Harness`] / [`spawn_harness`]: mock shiori・mock sakura sink・`spawn_kanade` を
//!   組み立てた駆動ハーネス（各テストのファイルが再利用する）。

// ============================================================================
// 責務単位の子モジュール（`#[path]` のフラット兄弟方式）
//
// 項目は責務単位の子ファイルに置き、ここは接続宣言とファサード再輸出だけを持つ。兄弟の
// テストファイルは子ファイルの分け方を知らず、`super::common::X` で引く。`tests/` 配下では
// `#[cfg(test)]` は意味論的に冗長だが、落とさず残す。
// ============================================================================

#[cfg(test)]
#[path = "common_bounded.rs"]
mod bounded;
#[cfg(test)]
#[path = "common_fixture.rs"]
mod fixture;
#[cfg(test)]
#[path = "common_harness.rs"]
mod harness;
#[cfg(test)]
#[path = "common_mock_sakura.rs"]
mod mock_sakura;
#[cfg(test)]
#[path = "common_mock_shiori.rs"]
mod mock_shiori;
#[cfg(test)]
#[path = "common_recording.rs"]
mod recording;
#[cfg(test)]
#[path = "common_window_actor.rs"]
mod window_actor;

// ============================================================================
// 疎通テスト（観測可能な完了条件）
// ============================================================================
#[cfg(test)]
#[path = "common_smoke.rs"]
mod smoke;

// ============================================================================
// ファサード再輸出（消費側の `super::common::X` を 0 変更に保つ・§17.5 に従い明示列挙）
//
// 子モジュール同士も、公開項目はこのファサード経由（`use super::{…}`）で引く。兄弟の直接
// パスを使うのは、ファサードに載らない `pub(super)` の内部項目（`FixtureState`・
// `run_join_bounded`・`starts_of`）だけである。
// ============================================================================

pub use bounded::{DEFAULT_TIMEOUT, drive_ticks_until_disconnect, join_bounded};
pub use fixture::{FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture};
pub use harness::{
    Driven, Harness, SinklessHarness, single_stop_cause, spawn_harness, spawn_harness_blocking,
    spawn_harness_failing, spawn_harness_failing_with_stop_sink, spawn_harness_gated,
    spawn_harness_gated_failing, spawn_harness_no_sink, spawn_harness_with_stop_sink,
};
pub use mock_sakura::{
    MockSakura, QuitPolicy, SakuraGate, spawn_mock_sakura, spawn_mock_sakura_gated,
};
pub use mock_shiori::{
    FailKind, FailOn, MockShiori, ShioriGate, spawn_mock_shiori, spawn_mock_shiori_blocking,
    spawn_mock_shiori_failing,
};
pub use recording::{
    CallMethod, RecordedCall, binding, command_tag, expected_call, expected_translate,
    expected_unload, position_of, pumps, resumed_get_after_active_window, send, tick,
};
pub use window_actor::{WINDOW_CREATE_SERIAL, spawn_window_actor};
