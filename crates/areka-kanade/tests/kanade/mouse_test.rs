//! マウスイベント発行の単一 pass/fail 檻（Req8.1・8.2 — 設計 Testing Strategy「Unit Tests
//! （kanade・mouse_test.rs — Req8 (a)(b)(d)＋フェーズ規律）」の 6 檻）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、`KanadeMsg::Mouse`
//! 注入に対する運行系の観測可能な振る舞いを、実時間 sleep なし・注入時刻/入力のみ・全 join 期限付きで
//! 単一の合否として検証する。期待 Reference 構成は必ず `events::on_mouse_*` 構築子から `expected_call`
//! で導出し、ハーネス側にハードコードしない（fixture・assert・実装の三点一正本・Req7.1）。
//!
//! # 檻一覧（設計 Testing Strategy #1〜#6）
//! 1. **(a) OnMouseMove layout**: `Steady{None}` で Move（region=Some("Head")）注入 → 記録 GET が
//!    `expected_call(on_mouse_move(x,y,0,Some("Head"),&INACTIVE))` と一致（Ref0..6・Ref2="0"・Ref5="0"・
//!    Ref6="mouse"・Status 行なし）。
//! 2. **(a') Ref4 None**: region=None → Ref4 が空文字 `""`（references 長は 7 のまま）。
//! 3. **(b) Ref5 左右**: DoubleClick Left→Ref5="0"／Right→Ref5="1"（`on_mouse_double_click` 導出共有）。
//! 4. **(d) 204→無動作**: マウス GET へ NoContent → StartTalk 不発（close talk のみ）・`Steady{None}` 維持。
//! 5. **フェーズ無視**: 非 Steady（Boot 完了前=Idle／close 系列中=CloseTalkWait）で Mouse 注入 →
//!    マウス GET は記録に現れない・状態不変（終了系列は正常完走）。
//! 6. **pending_close ガード**: active talk 中に CloseRequest（→pending_close）→ Mouse 注入 →
//!    マウス GET 不発・close 握手は既存どおり完走。
//!
//! # 決定性（Req8.2）と同期イディオム
//! steady_test.rs／close_test.rs と同じ枠組み: 挨拶なし boot（`without_boot_greeting`）で
//! `Steady{None}` へ直行させ（DD-IT-12 の挨拶 talk race を断つ）、末尾 talk（close talk か保留解放 talk）を
//! quit:true にして終了系列（Unload→StopSelf）を駆動する。kanade の期限付き join が成功した時点で、
//! それまでの全 shiori 呼出・全 StartTalk 配送は確定済みであり、実時間 sleep を一切用いずに記録列を
//! 確定できる。マウス GET は kanade drive ループの同期往復ゆえ、Boot・CloseRequest の同期完走後に
//! FIFO 順で処理される（in-flight ≤ 1・割り込みなし）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, MouseInput, StartTalk, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture, Harness,
    MouseResponse, QuitPolicy, RecordedCall, expected_call, expected_unload, join_bounded,
    spawn_harness, spawn_harness_gated,
};

// テーマ単位のテストモジュール接続宣言（areka-P0-file-slimming タスク 8.4・要件 1.7 / 3.1 / 3.2）。
// 本ファイルは檻全体の module doc と共通 import のみを保持し、複数テーマから参照される補助項目は
// `mouse_test_test_support.rs` へ、観測ケースはテーマごとの兄弟ファイル `mouse_test_<テーマ>_tests.rs`
// へ置く（子は `super::…` の明示 import で本ファイルの import 束縛と共有ヘルパを引く）。
#[cfg(test)]
#[path = "mouse_test_event_layout_tests.rs"]
mod event_layout_tests;
#[cfg(test)]
#[path = "mouse_test_phase_guard_tests.rs"]
mod phase_guard_tests;
#[cfg(test)]
#[path = "mouse_test_talk_start_tests.rs"]
mod talk_start_tests;
#[cfg(test)]
#[path = "mouse_test_test_support.rs"]
mod test_support;
