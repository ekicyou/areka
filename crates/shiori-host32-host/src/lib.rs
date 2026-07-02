//! host-32 x64/arm64 ホスト側 transport lib。
//!
//! 本クレートは x64/arm64 ネイティブで動作するホスト側の transport を提供する
//! （i686 helper は別クレート `shiori-host32-helper`）。共有ワイヤ規約は
//! `shiori-host32-ipc`（proto）を cargo 依存で共有する。
//!
//! # モジュール構成
//! - [`process_host`] — `ProcessHost`（helper spawn / 非ブロッキング生存監視 /
//!   終了分類）。**std-only**（`windows` 非依存）。
//! - [`parent_window`] — `ParentMessageWindow`（HELLO ハンドシェイク観測 /
//!   `pump_until_hello_or`）。`wintf-winmsg-executor` の message-only 窓。
//! - [`error`] — 構造化エラー型（`thiserror`）。

pub mod error;
pub mod parent_window;
pub mod process_host;

pub use error::{HandshakeError, SpawnError};
pub use parent_window::{ParentMessageWindow, WindowCreationError};
pub use process_host::{ExitKind, HelperHandle, PARENT_HWND_ENV, poll_exit, poll_exit_kind, spawn};
