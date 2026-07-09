//! 統合層: shiori アクター境界（`src/shiori/`）——`ShioriMsg` を受理する相手方の実体。
//!
//! 本モジュールは kanade の [`ShioriMsg`](crate::msg::ShioriMsg) 境界の**受理側**を実装する。
//! real（[`real`]）は既存 SHIORI 出口 API（`shiori-host32-host` の `Shiori3Client`／
//! `HelperLifecycle`）を専有スレッドで包み、公開呼出面 [`real::ShioriBackend`] 越しに
//! dispatch する——`spawn_shiori_actor` の connect closure が `Box<dyn ShioriBackend>` を返す
//! 形へ一般化されているため、mock（テスト／観測ハーネス側）は同一 trait を実装した scripted
//! fake を**型レベルで差し替え可能**である（Req 5.1/7.1/7.6）。host32 型
//! （`Shiori3Client` / `RequestError` / `ParentMessageWindow` / `HelperLifecycle`）を import して
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
//!   正規 clean shutdown（[`ShioriBackend::unload`](real::ShioriBackend::unload)＝unload →
//!   helper 正常終了観測）へ委譲する。`Ok(ExitKind::Clean)` は
//!   [`ShioriOutcome::Unloaded`](crate::msg::ShioriOutcome::Unloaded)、`Ok(その他)` も完了として
//!   同じく `Unloaded`（終了種別が Clean でない旨を `warn!` する）、`Err` は `error!` の上
//!   [`ShioriOutcome::Failed`](crate::msg::ShioriOutcome::Failed) を返す（境界契約
//!   `ShioriMsg::Unload`／`Unloaded`／`Failed` 自体は不変）。
//! - [`ShioriMsg::Close`](crate::msg::ShioriMsg::Close): 停止規約どおり**即時停止**する（受信
//!   ループを直ちに抜け、接続資材を drop して RAII teardown する）。積み残しは drain しない。
//!
//! 受信ループの正常な終了経路は「Close 受領」と「全 [`Sender<ShioriMsg>`](crate::msg::ShioriMsg)
//! drop（inbox 切断）」の 2 経路である。
//!
//! # 接続確立と死活報告（Req 3.2/3.4/5.3/6.1・on_down の寿命）
//!
//! real アクターは接続手順を呼び手の connect クロージャに委ね、アクタースレッド上で一度だけ
//! 実行する（`ParentMessageWindow` が `!Send` のため spawn 前に実行できない）。接続確立に失敗
//! した場合は [`KanadeMsg::ShioriDown`](crate::msg::KanadeMsg::ShioriDown) を `on_down` へ送って
//! 死活報告とし、受信ループには入らず終了する。
//!
//! 接続確立成功後は、`on_down`（kanade inbox の送信端）を**受信ループの生存期間中保持する**
//! （死活監視の届け先・Req 3.4）。メッセージ到達のたびに冒頭で
//! [`ShioriBackend::status`](real::ShioriBackend::status) を確認し、`Exited(kind)` を初回観測
//! したら `error!`＋`ShioriDown` を一度だけ送る（sticky）。正規 clean shutdown（unload 成功）後
//! はこの死活チェック自体を行わない（正規終了は死ではない）。この保持は
//! kanade→shiori→on_down の Sender 環を作るが、ループは Close 受領または全 Sender drop で終了
//! し、その時点で `on_down` は自然に drop される——「アクター別の停止経路」マトリクス参照
//! （kanade の「全 Sender drop で正常終了」は on_down 保持構成では環の解体後にのみ成立する）。

pub mod real;

pub use real::{ShioriBackend, ShioriConnection, spawn_shiori_actor};
