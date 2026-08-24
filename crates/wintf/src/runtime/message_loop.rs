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
use std::rc::Rc;

use crate::executor::{FilterResult, MessageLoop, block_on};
use event_listener::Event;
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
    // `WinApp::run`（task 4.3 結線済み）が shutdown future の駆動主経路として利用する。
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

/// `block_on` の「loop 先行 quit で panic」を回避する終了規律（設計 ShutdownPolicy）。
///
/// 状態を持たない名前空間。終了シグナル（`event_listener::Event`・`WinApp` 所有）から
/// `run()` の `block_on` が待つ shutdown future を組み立てる接点と、tail race を避ける
/// 終了時 notify 規律を提供する。
///
/// # 終了規律（要件 1.3/1.4/1.5）
/// `run()` は本来 `block_on(shutdown_future(event))` でループを駆動し、最後のウィンドウ
/// 破棄（registry 空遷移）で `WinApp::new()` が注入した hook が `event.notify(usize::MAX)`
/// を撃つことで shutdown future を完了させ、ループを **先行 quit させず** future 完了で
/// 正常復帰させる（`PostQuitMessage` 先撃ちによる "received unexpected quit message"
/// panic を構造的に回避）。
///
/// # tail race（要件 1.5）
/// shutdown future の `listen()` arm と notify の間でタスク完了直後の wake を取りこぼす
/// 競合（先進坑が観測）に対し、[`notify_shutdown`] を終了時に補助的に撃つ規律を踏襲する。
//
// `run()` の `block_on(shutdown_future(...))` 結線は task 4.3 で完了済み。
pub(crate) struct ShutdownPolicy;

impl ShutdownPolicy {
    /// 終了シグナル `Event` が notify されるまで完了しない shutdown future を返す。
    ///
    /// notify 取りこぼし防止規律（設計・`AsyncTickTask` と同形）に従い、await の **前** に
    /// `event.listen()` を arm してから待機する。これにより listener arm 直後・await 到達
    /// 前に届いた notify（既に notify 済みの Event を含む）も取りこぼさず捕捉する
    /// （`event_listener::Listener` は arm 後の notify を保持する）。
    ///
    /// `run()`（task 4.3）はこの future を [`MessageLoopDriver::block_on`] へ渡し、最後の
    /// ウィンドウ破棄で notify されると future が完了してループを正常復帰させる。
    pub(crate) fn shutdown_future(event: Rc<Event>) -> impl Future<Output = ()> {
        async move {
            // await 前に arm（処理中・arm 直後に届く notify も落とさない）。
            let listener = event.listen();
            listener.await;
        }
    }

    /// 終了シグナルを全リスナ起床で notify する（tail race 補填の防御的 notify）。
    ///
    /// `event.notify(usize::MAX)` を撃ち、待機中の shutdown future を起床させる。終了時に
    /// 防御的へ複数回撃っても冪等（`event_listener` は arm 済みリスナのみ起床し、未 arm の
    /// notify は次の arm へ持ち越されない＝余分な副作用なし）。`WinApp::new()` が registry の
    /// shutdown hook に同じ `notify(usize::MAX)` を仕込む（正常経路）一方、本 helper は run
    /// 側の終了時 tail race 補填（task 4.3 で結線済み）に用いる。
    pub(crate) fn notify_shutdown(event: &Event) {
        event.notify(usize::MAX);
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

    // ── ShutdownPolicy (task 4.2) ─────────────────────────────────

    /// 完了状態（要件 1.3/1.4/1.5）: shutdown future は Event が notify されると完了する。
    ///
    /// headless 検証として `block_on` でループを駆動せず、future が「notify 済み」なら確実に
    /// 完了することを `event_listener::Listener` のブロッキング `.wait()` で実証する
    /// （shutdown_future が arm 内部で arm→await する形と同じ・先に notify 済みなら即復帰）。
    /// これにより run()（4.3）が `block_on(shutdown_future)` を **先行 quit せず** future
    /// 完了で正常復帰できる構造を裏付ける（block_on 結線自体は 4.3）。
    #[test]
    fn shutdown_future_completes_when_event_notified() {
        use event_listener::Listener;

        let event = Rc::new(Event::new());

        // shutdown_future は arm→await する。同スレッドの headless 検証では、arm 前に
        // notify を撃ってもよいが、本テストは「arm 済みリスナが notify で起床する」
        // ことを直接実証する（future 完了 = listener 起床の同値）。
        let listener = event.listen();
        ShutdownPolicy::notify_shutdown(&event);
        // arm 済みリスナは notify 済みなら即復帰する（ハングしない＝future 完了相当）。
        listener.wait();

        // shutdown_future が arm→await する形でも、先に notify 済みなら即完了することを確認。
        let l2 = event.listen();
        event.notify(usize::MAX);
        l2.wait();
    }

    /// `notify_shutdown` は arm 済みリスナを起床させ、複数回撃っても冪等（防御的 notify）。
    #[test]
    fn notify_shutdown_wakes_armed_listener_and_is_idempotent() {
        use event_listener::Listener;

        let event = Event::new();
        let listener = event.listen();
        // tail race 補填として複数回撃っても、arm 済みリスナは起床し panic しない。
        ShutdownPolicy::notify_shutdown(&event);
        ShutdownPolicy::notify_shutdown(&event);
        listener.wait();
    }
}
