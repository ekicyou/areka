//! 観測ハーネス基盤（設計「検証層 → 観測ハーネス（tests/kanade/）」）。
//!
//! 後続の統合テスト（4.2〜4.6・5）が個別テストファイルを追加するだけで運行表を
//! 決定的に観測できるよう、以下を提供する:
//!
//! - [`RecordedCall`]: 受理された shiori 呼出（Method・id・References）の記録単位。
//! - [`spawn_mock_shiori`]: real と**同一の [`areka_kanade::ShioriMsg`] 型**を受け、
//!   [`Fixture`] 表に従い同梱 `reply` へ**即時**応答する mock shiori アクター
//!   （trait 不要＝Req 5.1 の型レベル差し替え）。受理呼出を [`RecordedCall`] 列へ蓄積。
//! - [`spawn_mock_sakura`]: [`TalkCommand`] を到着順に受領・記録し、`Start` についてシナリオ指示（quit true/false・
//!   遅延なし）どおり [`KanadeMsg::TalkDone`] を kanade inbox へ返す mock 再生系宛先。
//! - [`expected_call`]: `areka_kanade::events::*` の [`ShioriCall`] を [`RecordedCall`] へ
//!   変換する導出関数（fixture・検証・実装が単一の正本＝events 表を共有・Req 7.1）。
//! - [`run_bounded`] / [`join_bounded`]: 期限付き待機によるハング検出ヘルパ
//!   （どのテストもハングしない・Req 7.3）。
//! - [`Harness`] / [`spawn_harness`]: mock shiori・mock sakura sink・`spawn_kanade` を
//!   組み立てた駆動ハーネス（4.2〜4.6 が再利用する）。
//!
//! # 未使用許容（プレースホルダ段階）
//! 本モジュールが提供する要素の多くは後続タスク（4.2〜4.6・5）の `#[test]` から
//! 初めて使われる。エントリポイントが全モジュールを先出し宣言する 4.1 時点では
//! 一部が未使用となりコンパイル警告を招くため、モジュール全体に
//! `#![allow(dead_code)]` を付す（プレースホルダ段階に限定した意図的許容）。

#![allow(dead_code)]

// ============================================================================
// 責務単位の子モジュール（タスク 8.2・設計判断 #1 の `#[path]` フラット兄弟方式）
//
// 本ファイルは 1,657 行あったため、項目を責務単位の子ファイルへ純移動し、ここは接続宣言と
// ファサード再輸出だけを持つ（設計判断 #5 の D1 ファサード再輸出型）。`super::common::X` で
// 引く兄弟テストファイル 9 本は 1 行も変更していない。`tests/` 配下では `#[cfg(test)]` は
// 意味論的に冗長だが、設計判断 #13 に従い落とさず残す。
// ============================================================================

#[cfg(test)]
#[path = "common_recording.rs"]
mod recording;
#[cfg(test)]
#[path = "common_fixture.rs"]
mod fixture;
#[cfg(test)]
#[path = "common_mock_shiori.rs"]
mod mock_shiori;
#[cfg(test)]
#[path = "common_mock_sakura.rs"]
mod mock_sakura;
#[cfg(test)]
#[path = "common_bounded.rs"]
mod bounded;
#[cfg(test)]
#[path = "common_harness.rs"]
mod harness;

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
// `run_join_bounded`）だけである。
// ============================================================================

pub use bounded::{DEFAULT_TIMEOUT, drive_ticks_until_disconnect, join_bounded};
// `run_bounded` は現時点で消費者を持たない（module doc「未使用許容（プレースホルダ段階）」）。
// 項目そのものは `#![allow(dead_code)]` が許容するが、ファサード再輸出は `unused_imports` の
// 対象になるため、この 1 本だけ局所的に許容する（ワークスペース警告の基準値を維持）。
#[allow(unused_imports)]
pub use bounded::run_bounded;
pub use fixture::{
    ChoiceResponse, FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture,
    MouseResponse,
};
pub use harness::{
    Harness, SinklessHarness, spawn_harness, spawn_harness_blocking, spawn_harness_failing,
    spawn_harness_gated, spawn_harness_gated_failing, spawn_harness_no_sink,
};
pub use mock_sakura::{
    MockSakura, QuitPolicy, SakuraGate, spawn_mock_sakura, spawn_mock_sakura_gated,
};
pub use mock_shiori::{
    BlockOn, FailKind, FailOn, MockShiori, ShioriGate, spawn_mock_shiori,
    spawn_mock_shiori_blocking, spawn_mock_shiori_failing,
};
pub use recording::{CallMethod, RecordedCall, expected_call, expected_unload};
