//! クリック透過機構（`WS_EX_TRANSPARENT` 動的トグル）の自己完結モジュール。
//!
//! 表示層（GPU 合成 visual/content）を不変に保ちつつ、当たり判定層（HWND
//! 拡張スタイル）のみを制御して透明領域のクリックを背面プロセスへ透過させる。
//!
//! 現状は監視対象レジストリ（[`ClickThroughRegistry`]）のみを提供する。
//! カーソル監視ワーカ（`monitor`）・UI スレッド判定ループ（`controller`）は
//! 後続タスクで追加する。
//!
//! NOTE: 本モジュールの型・API は後続タスク（controller）が消費する。
//! 骨格段階では未消費のため、モジュール限定で dead_code/unused を許可する。
#![allow(dead_code, unused_imports)]

mod monitor;
mod registry;

pub(crate) use monitor::CursorMonitorBridge;
pub(crate) use registry::{ClickThroughRegistry, ClickThroughTarget, DesiredState};
