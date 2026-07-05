//! 統合層: shiori アクター境界（`src/shiori/`）——`ShioriMsg` を受理する相手方の実体。
//!
//! 本モジュールは kanade の [`ShioriMsg`](crate::msg::ShioriMsg) 境界の**受理側**を実装する。
//! real（[`real`]）は既存 SHIORI 出口 API（`shiori-host32-host` の `Shiori3Client`）を専有
//! スレッドで包み、mock（観測ハーネス側）は同一 [`ShioriMsg`] 型を受ける別 body である
//! ——両者は trait を介さず**型レベルで差し替え可能**である（Req 5.1）。host32 型
//! （`Shiori3Client` / `RequestError` / `ParentMessageWindow` / `HelperHandle`）を import して
//! よいのは [`real`] モジュール**のみ**である（Boundary Commitment）。
//!
//! # ShioriMsg 受理規約（envelope 規約・停止規約）
//!
//! shiori アクターは単一 inbox（[`std::sync::mpsc::Receiver<ShioriMsg>`](crate::msg::ShioriMsg)）
//! を専有スレッドで受け、次の規約で応答する:
//!
//! - [`ShioriMsg::Request`](crate::msg::ShioriMsg::Request): 同梱 `reply`（oneshot 相当）へ
//!   [`ShioriOutcome`](crate::msg::ShioriOutcome) を**ちょうど 1 回**送る。GET は
//!   `Value`／`NoContent`、NOTIFY は `Notified`、呼出失敗は区別語彙を保った
//!   [`ShioriOutcome::Failed`](crate::msg::ShioriOutcome::Failed) を返す（Req 5.2/6.1）。
//!   `reply.send` の失敗（要求側が既に取り消し／切断）は無視してよい（envelope 規約: 積み残し
//!   drop による切断は要求側が `Err` として観測する）。
//! - [`ShioriMsg::Unload`](crate::msg::ShioriMsg::Unload): 同梱 `reply` へちょうど 1 回応答する。
//!   M1 は**暫定実装**で [`ShioriOutcome::Unloaded`](crate::msg::ShioriOutcome::Unloaded) を返す
//!   （実資材の解放は接続 drop 時の RAII に委ねる。正規 unload 経路が host32-lifecycle で確立
//!   された際に差し替え可能な境界を保つ——境界契約 `ShioriMsg::Unload`／`Unloaded` は不変）。
//! - [`ShioriMsg::Close`](crate::msg::ShioriMsg::Close): 停止規約どおり**即時停止**する（受信
//!   ループを直ちに抜け、接続資材を drop して RAII teardown する）。積み残しは drain しない。
//!
//! 受信ループの正常な終了経路は「Close 受領」と「全 [`Sender<ShioriMsg>`](crate::msg::ShioriMsg)
//! drop（inbox 切断）」の 2 経路である。
//!
//! # 接続確立と死活報告（Req 5.3/6.1・on_down の寿命）
//!
//! real アクターは接続手順を呼び手の connect クロージャに委ね、アクタースレッド上で一度だけ
//! 実行する（`ParentMessageWindow` が `!Send` のため spawn 前に実行できない）。接続確立に失敗
//! した場合は [`KanadeMsg::ShioriDown`](crate::msg::KanadeMsg::ShioriDown) を `on_down` へ送って
//! 死活報告とし、受信ループには入らず終了する。`on_down`（kanade inbox の送信端）は接続確立の
//! 成否確定後に**直ちに drop** し、受信ループ中は保持しない——保持すると kanade inbox が生き
//! 続け、kanade 側の「全 Sender drop で正常終了」（Req 4.9）を妨げるためである。

pub mod real;

pub use real::{ShioriConnection, spawn_shiori_actor};
