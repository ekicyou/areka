//! host-32 x64/arm64 ホスト側 transport lib。
//!
//! 本クレートは x64/arm64 ネイティブで動作するホスト側の transport を提供する
//! （i686 helper は別クレート `shiori-host32-helper`）。共有ワイヤ規約は
//! `shiori-host32-ipc`（proto）を cargo 依存で共有する。
//!
//! # モジュール構成
//! - [`process_host`] — `ProcessHost`（helper spawn / 非ブロッキング生存監視 /
//!   終了分類）。**std-only**（`windows` 非依存）。
//! - [`error`] — 構造化エラー型（`thiserror`）。

pub mod error;
pub mod process_host;

pub use error::SpawnError;
pub use process_host::{ExitKind, HelperHandle, PARENT_HWND_ENV, poll_exit, poll_exit_kind, spawn};
