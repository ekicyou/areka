//! クリック透過機構（`WS_EX_TRANSPARENT` 動的トグル）の自己完結モジュール。
//!
//! 表示層（GPU 合成 visual/content）を不変に保ちつつ、当たり判定層（HWND
//! 拡張スタイル）のみを制御して透明領域のクリックを背面プロセスへ透過させる。
//!
//! 監視対象レジストリ（[`ClickThroughRegistry`]）、カーソル監視ワーカ
//! （[`CursorMonitorBridge`]）、UI スレッド判定・適用ループ
//! （[`start_click_through`]／[`ClickThroughHandle`]）を提供する。

mod controller;
mod monitor;
mod registry;

/// areka が監視対象窓を登録するための **公開** ハンドル型。`runtime/mod.rs` の
/// `wire_click_through` が World へ NonSend リソースとして挿入する。
pub use controller::ClickThroughRegistryHandle;

pub(crate) use controller::{ClickThroughHandle, start_click_through};
pub(crate) use monitor::CursorMonitorBridge;
pub(crate) use registry::{ClickThroughRegistry, DesiredState};
