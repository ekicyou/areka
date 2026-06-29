//! メッセージループ層。自作 `PeekMessageW` ポンプを撤去し、ライブラリ
//! （`wintf-winmsg-executor`）の `block_on` / `MessageLoop::run` へ委譲する。
//!
//! 本層は OS メッセージをライブラリのループ経由でウィンドウ手続きへ取りこぼしなく
//! 配送する最小経路（`MessageLoopDriver`）のみを提供する。`WinApp::run` の全結線
//! （tick タスクの spawn・shutdown future の所有・vsync/registry の生存期間管理）は
//! 後続タスクで結線する。
//!
//! # filter 方針
//!
//! wintf 側 filter は **原則 [`FilterResult::Forward`]** を返し、OS メッセージを
//! そのままウィンドウ手続きへ配送する。自前の `WM_VSYNC` pop 分岐は持たない
//! （wake メッセージはライブラリが `run_loop` 内部で保護しており、filter で drop
//! できない。アプリ固有の wake/tick 駆動はライブラリ executor 側へ委ねる）。
//!
//! また filter クロージャ内から [`MessageLoop::run`] を再帰呼び出ししてはならない
//! （ライブラリはネストした `run` で panic する）。非同期処理をループ内で駆動したい
//! 場合は [`block_on`] によるネストが正規経路となる。

use std::future::Future;

use wintf_winmsg_executor::{FilterResult, MessageLoop, block_on};
use windows::Win32::UI::WindowsAndMessaging::MSG;

/// ライブラリのメッセージループへ委譲する driver。
///
/// 旧 `WinThreadMgr::run` の自作 `PeekMessageW` ポンプを置換する building block。
/// 状態を持たないため値ではなく関連関数の名前空間として用いる。
///
/// - [`block_on`](MessageLoopDriver::block_on): shutdown future が完了するまで
///   ループを駆動する主経路（`WinApp::run` が後続タスクで利用する）。
/// - [`run`](MessageLoopDriver::run): `MessageLoop::run` への薄い委譲。default filter は
///   常に [`FilterResult::Forward`] を返す。
///
/// いずれの経路でも、filter は `WM_VSYNC` を pop せず、`MessageLoop::run` を再帰呼び
/// 出ししない（モジュールレベルの doc 参照）。
pub(crate) struct MessageLoopDriver;

impl MessageLoopDriver {
    /// shutdown future が完了するまでメッセージループを駆動し、その値を返す。
    ///
    /// ライブラリの [`block_on`] へ委譲する。`block_on` は内部で
    /// [`MessageLoop`] を生成して呼び出しスレッド上でループを回し、`future` 完了時に
    /// ループを quit する。`spawn_local` で投入済みの UI タスクも並行に駆動される。
    ///
    /// # Panics
    ///
    /// `future` 完了前にメッセージループが quit した場合（`future` / spawn 済みタスクが
    /// `PostQuitMessage` を呼んだ場合など）に panic する（ライブラリ仕様）。
    // NOTE: `WinApp::run` の全結線（後続タスク）で利用される主経路。現状は
    // ヘッドレステストのみが呼び出すため lib ビルドでは未使用となる。
    #[allow(dead_code)]
    pub(crate) fn block_on<T>(future: impl Future<Output = T>) -> T {
        block_on(future)
    }

    /// 既定 filter（常に [`FilterResult::Forward`]）でメッセージループを実行する。
    ///
    /// ライブラリの [`MessageLoop::run`] へ委譲する。OS メッセージはすべてウィンドウ
    /// 手続きへ配送され、wake メッセージはライブラリが保護する。`future` 完了による
    /// 自動 quit は無いため、終了は filter 経由の `MessageLoop::quit` 等に委ねる
    /// （その結線は後続タスク）。
    ///
    /// # Panics
    ///
    /// filter クロージャ内から（直接・間接に）本関数を再入した場合に panic する
    /// （ライブラリ仕様：ネストした `MessageLoop::run` は不可）。
    #[allow(dead_code)]
    pub(crate) fn run() {
        MessageLoop::run(|_loop, msg| Self::default_filter(msg));
    }

    /// wintf 既定の filter 判定。常に [`FilterResult::Forward`] を返す。
    ///
    /// `WM_VSYNC` を含む一切の OS メッセージを drop せずウィンドウ手続きへ転送する。
    /// `MSG` は読み取りのみで副作用を持たない。
    pub(crate) fn default_filter(_msg: &MSG) -> FilterResult {
        FilterResult::Forward
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `block_on` 委譲が即時完了 future の値を返し、ループがクリーンに quit すること
    /// （"received unexpected quit message" panic が起きないこと）を確認する。
    #[test]
    fn block_on_ready_future_returns_value() {
        let v = MessageLoopDriver::block_on(async { 42u32 });
        assert_eq!(v, 42);
    }

    /// 既定 filter が任意の `MSG`（ゼロ値）に対し `Forward` を返すこと。
    #[test]
    fn default_filter_forwards_arbitrary_message() {
        // SAFETY: `MSG` は POD（plain old data）であり、全ゼロは有効な表現。
        // default_filter は中身を読まずに Forward を返すため未初期化由来の UB は無い。
        let msg: MSG = unsafe { std::mem::zeroed() };
        assert_eq!(
            MessageLoopDriver::default_filter(&msg),
            FilterResult::Forward
        );
    }
}
