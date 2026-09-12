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
//! - [`charset`] — `Charset`（任意の文字コードを列挙せず表す newtype）と `LabelError`。
//!   ラベル解決・符号化・復号を `encoding_rs` へ委ねる。純粋・決定的。
//! - [`shiori3`] — SHIORI/3.0 ワイヤコーデック（`build_request` / `parse_response`）。純粋・決定的。
//! - [`client`] — `Shiori3Client`（`get`/`notify` の出口 API）。codec build → `send_request` →
//!   parse → `RequestError` 統合を結線する。
//! - [`error`] — 構造化エラー型（`thiserror`）。
//! - [`lifecycle`] — helper のライフサイクル管理（`HelperLifecycle` による正常終了要求 /
//!   `classify_failure` による統一終了分類 / `LifecycleReport` によるレポート /
//!   `ShutdownError` shutdown 失敗語彙）。クレート直下に公開 re-export。

/// 通信の文字コード（`Charset`）とラベル解決の失敗理由（`LabelError`）。
pub mod charset;
pub mod client;
pub mod error;
/// helper 孤児化防止（Job Object・KILL_ON_JOB_CLOSE）。windows 依存をここへ隔離する。
mod job;
pub mod lifecycle;
pub mod parent_window;
pub mod process_host;
pub mod shiori3;

pub use charset::LabelError;
pub use client::Shiori3Client;
pub use error::{HandshakeError, RequestError, ShioriError, SpawnError};
pub use lifecycle::{
    FailureClass, HelperLifecycle, HelperStatus, LifecycleReport, ShutdownError, classify_failure,
};
pub use parent_window::{ParentMessageWindow, SendError, WindowCreationError};
pub use process_host::{
    ExitKind, HelperHandle, PARENT_HWND_ENV, REQUEST_TIMEOUT, poll_exit, poll_exit_kind, spawn,
};
pub use shiori3::{Charset, Method, ParsedResponse, ShioriRequest, build_request, parse_response};
