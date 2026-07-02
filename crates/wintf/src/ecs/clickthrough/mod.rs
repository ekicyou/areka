//! クリック透過機構（`WS_EX_TRANSPARENT` 動的トグル）の自己完結モジュール。
//!
//! 表示層（GPU 合成 visual/content）を不変に保ちつつ、当たり判定層（HWND
//! 拡張スタイル）のみを制御して透明領域のクリックを背面プロセスへ透過させる。
//!
//! 監視対象レジストリ（[`ClickThroughRegistry`]）、カーソル監視ワーカ
//! （[`CursorMonitorBridge`]）、UI スレッド判定・適用ループ
//! （[`ClickThroughController`]／[`ClickThroughHandle`]）を提供する。
//!
//! NOTE: `ClickThroughController::start` の呼び出し（runtime 結線）は後続タスク（3.2）で
//! 行う。本タスク時点では start エントリ・register/remove API が未消費のため、
//! モジュール限定で dead_code/unused を許可する。
#![allow(dead_code, unused_imports)]

mod controller;
mod monitor;
mod registry;

pub(crate) use controller::{
    ClickThroughController, ClickThroughHandle, evaluate_targets, resolve_transition,
};
pub(crate) use monitor::CursorMonitorBridge;
pub(crate) use registry::{ClickThroughRegistry, ClickThroughTarget, DesiredState};
