//! 選択確定カスケードの統合檻。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、注入
//! `ChoiceWaiting`／`Choice`／`Tick` のみで選択確定の全帰結を観測する。実時間 sleep も実時刻も
//! 用いず、全 join は期限付きである。
//! 期待 Reference 構成は必ず `events::on_choice_*` 構築子から `expected_call` で導出し、檻側に
//! ハードコードしない（fixture・assert・実装の三点一正本）。
//!
//! 群は任意名形／正典形／選択起因の失敗／実行状態表示／タイムアウト／一回性と棄却の 6 つで、ほかに時間の指令の檻と期限の知らせの檻がある（いずれも兄弟の `choice_test_*_tests.rs`）。
//!
//! # 決定性と同期イディオム
//! steady_test.rs／mouse_test.rs と同じ枠組み: 挨拶なし boot（`without_boot_greeting`）で
//! `Steady{None}` へ直行させ、Tick1 の `OnSecondChange` Value で steady talk（id=1）を起こし、
//! 保留ハーネス（`spawn_harness_gated`・hold_indices=[0]）でその `TalkDone` を park して active talk
//! 窓を維持する。選択待ち帳簿は「現行 talk と一致する `ChoiceWaiting`」でしか確立しないため、この
//! 窓が全群の前提である。カスケードは kanade drive ループの同期往復（execute-batch/reinject-last）で
//! 1 メッセージ処理内に完結するため、段の途中に Tick も別の選択確定も割り込まない。終了は末尾 talk を
//! quit:true にして駆動し（群 3 の `non_choice_failure_during_choice_wait_still_faults` だけは例外で、
//! 終了は Fault 経路が駆動する）、kanade の期限付き join 成功をもって全記録の確定点とする。
//!
//! # 期限（deadline）の作り方——実時刻を一切読まない
//! 注入する `ChoiceWaiting` は `timeout_directive_secs: None`（＝既定値
//! `CHOICE_TIMEOUT_DEFAULT_MS` へ委譲）で、起点は `display_end = CHOICE_DISPLAY_END_MS` ゆえ
//! 期限は `CHOICE_DEADLINE_MS` である。したがって**注入 Tick の `now` を選ぶだけ**で期限の手前／
//! 到達を作り分けられる（実時間待機も `Instant` 読み取りも一切ない）。
//! 群 1〜4 は期限より手前の Tick しか注入しないためタイムアウト経路へは構造上到達せず、群 5 が
//! 期限手前 → 期限到達の 2 点を注入してタイムアウトのみを踏む。

use std::sync::mpsc::Sender;

use areka_kanade::{
    ChoiceInput, CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, TalkCommand, TalkDone, TalkEndReason, TalkId, events,
};

use super::common::{
    CallMethod, Driven, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, FailKind, FailOn, Fixture,
    QuitPolicy, RecordedCall, expected_call, expected_unload, position_of, pumps, send,
    spawn_harness_gated, spawn_harness_gated_failing, tick,
};

// テーマ単位のテストモジュール接続宣言。
// 本ファイルは檻全体の module doc と共通 import のみを保持し、共有ヘルパは `choice_test_test_support.rs`
// へ、観測ケースは群ごとの兄弟ファイル `choice_test_<テーマ>_tests.rs` へ置く（子は `super::…` で本ファイルの
// import 束縛と共有ヘルパを引く）。
#[cfg(test)]
#[path = "choice_test_canonical_cascade_tests.rs"]
mod canonical_cascade_tests;
#[cfg(test)]
#[path = "choice_test_choosing_status_tests.rs"]
mod choosing_status_tests;
#[cfg(test)]
#[path = "choice_test_named_event_tests.rs"]
mod named_event_tests;
#[cfg(test)]
#[path = "choice_test_rejection_tests.rs"]
mod rejection_tests;
#[cfg(test)]
#[path = "choice_test_stage_failure_tests.rs"]
mod stage_failure_tests;
#[cfg(test)]
#[path = "choice_test_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "choice_test_timeout_directive_tests.rs"]
mod timeout_directive_tests;
#[cfg(test)]
#[path = "choice_test_timeout_tests.rs"]
mod timeout_tests;
#[cfg(test)]
#[path = "choice_test_wake_tests.rs"]
mod wake_tests;
