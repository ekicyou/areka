//! close 握手・quit 分岐・期限・強制終了の統合検証（Req 3.4・4.2・4.4・4.5・4.6・4.7）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、close
//! 握手の 4 シナリオを個別の `#[test]` として決定的に観測する（実時間 sleep なし・時刻は注入
//! [`MonotonicMs`] Tick のみ・全 join は期限付き）:
//!
//! 1. **終了拒否 → 定常復帰 → pump 再開**（Req 4.5・3.4）: OnClose が別れの Value を返すが
//!    close talk の TalkDone が quit:false → kanade は終了せず `Steady{None}` へ復帰し、以降の
//!    Tick で OnSecondChange GET（pump）が再開する。再開後の pump が起こす steady talk を quit:true に
//!    することで終了系列を駆動し、「拒否点で停止していない・pump 再開」を終了到達それ自体で保証する。
//! 2. **無言終了**（Req 4.6）: OnClose が 204 → 追加イベントなしで終了系列へ直行し、記録列の末尾は
//!    `[.., OnClose GET, Unload]`（OnCloseAll 非発行・close talk 非起動）。
//! 3. **再生完了待ちの時間超過**（Req 4.7）: 保留ハーネスで close talk の TalkDone を差し止め、
//!    小さな `close_talk_deadline_ms` を超える Tick を注入 → DeadlineExceeded で終了系列を継続
//!    （TalkDone 不着でも join 成功・宙吊りなし）。
//! 4. **強制終了直行**（Req 4.4・DD-10）: ForceQuit → best-effort OnClose NOTIFY → Unload →
//!    StopSelf へ直行し join 成功。
//!
//! # 追加: boot 挨拶の統合檻（DD-IT-12・Req 1.5/2.4/3.1）
//! さらに boot 起動挨拶（default fixture・挨拶 talk＝StartTalk index 0）を保留ハーネスで active に
//! 保ち、DD-IT-12 の「挨拶を正規追跡する」意味論を統合層で 3 本、決定的に観測する（いずれも
//! additive・既存 4 シナリオは無改変）:
//! 5. **挨拶 active 中の Tick → NOTIFY（Ref3=0・Status: talking）**（Req 1.5/2.4）: 挨拶 talk を保留し、
//!    その最中の Tick が BOOT 挨拶由来の playing-semantics（NOTIFY・talking）を出すことを観測する
//!    （＝boot が `Steady{Some(挨拶)}` へ完了し、その slot 由来で pump を発行している証左）。
//! 6. **挨拶 TalkDone → GET pump 再開**（Req 4.4・相関成立の統合証左）: 保留解放で挨拶 TalkDone{Ended}
//!    を着弾させ、次 Tick で GET（Ref3=1・Status なし）が再開する＝挨拶が slot と照合され `Steady{None}`
//!    へ復帰した証左（照合しなければ `Steady{Some}` のまま NOTIFY を出し続け GET は現れない）。
//! 7. **挨拶中 CloseRequest → CloseTalkWait 経由 OnClose**（Req 3.1）: 挨拶再生中の close は即握手せず
//!    `pending_close` に記録され、挨拶 TalkDone 着弾で通常 talk と同じ握手（OnClose GET→別れ→close talk）が
//!    始まる（DD-IT-12「挨拶中 close は通常 talk と同じ CloseTalkWait」）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT,
    Fixture, Harness, QuitPolicy, RecordedCall, drive_ticks_until_disconnect, expected_call,
    expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

// テーマ単位のテストモジュール接続宣言（areka-P0-file-slimming タスク 8.4・要件 1.7 / 3.1 / 3.2）。
// 本ファイルは檻全体の module doc と共通 import のみを保持し、複数テーマから参照される補助項目は
// `close_test_test_support.rs` へ、観測ケースはテーマごとの兄弟ファイル `close_test_<テーマ>_tests.rs`
// へ置く（子は `super::…` の明示 import で本ファイルの import 束縛と共有ヘルパを引く）。
#[cfg(test)]
#[path = "close_test_boot_greeting_tests.rs"]
mod boot_greeting_tests;
#[cfg(test)]
#[path = "close_test_handshake_tests.rs"]
mod handshake_tests;
#[cfg(test)]
#[path = "close_test_test_support.rs"]
mod test_support;
