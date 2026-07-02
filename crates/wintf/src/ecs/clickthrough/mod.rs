//! クリック透過機構（`WS_EX_TRANSPARENT` 動的トグル）の自己完結モジュール。
//!
//! 表示層（GPU 合成 visual/content）を不変に保ちつつ、当たり判定層（HWND
//! 拡張スタイル）のみを制御して透明領域のクリックを背面プロセスへ透過させる。
//!
//! 監視対象レジストリ（[`ClickThroughRegistry`]）、カーソル監視ワーカ
//! （[`CursorMonitorBridge`]）、UI スレッド判定・適用ループ
//! （[`ClickThroughController`]／[`ClickThroughHandle`]）を提供する。
//!
//! NOTE: `ClickThroughController::start` の runtime 結線は task 3.2 で `runtime/mod.rs` から
//! 行う（`ClickThroughRegistryHandle` を NonSend リソースとして World へ挿入し、areka の
//! 窓登録（task 4.1）へ橋渡しする）。一部の内部 API（`ClickThroughHandle` の一部アクセサ等）は
//! まだ全消費されないため、モジュール限定で dead_code/unused を許可する。
#![allow(dead_code, unused_imports)]

mod controller;
mod monitor;
mod registry;

/// areka（task 4.1）が監視対象窓を登録するための **公開** ハンドル型。`runtime/mod.rs`
/// （task 3.2）が World へ NonSend リソースとして挿入する。
pub use controller::ClickThroughRegistryHandle;

pub(crate) use controller::{
    ClickThroughController, ClickThroughHandle, evaluate_targets, prune_dead_targets,
    resolve_transition,
};
pub(crate) use monitor::CursorMonitorBridge;
pub(crate) use registry::{ClickThroughRegistry, ClickThroughTarget, DesiredState};
