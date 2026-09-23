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

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

use crate::executor::{FilterResult, MessageLoop, block_on};
use event_listener::Event;
use tracing::{debug, info};
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

/// 明示の終了の指示の受け口（NonSend リソース・`Clone` は同じ実体の共有）。
///
/// 「指示済み」の 1 ビットと終了シグナル（`event_listener::Event`）を 1 つにまとめる。
/// `WinApp` が 1 つ持ち、その clone を World へ NonSend として据える（`ClickThroughRegistryHandle`
/// と同型）。利用側は `&mut World` から取り出して [`request_exit`](Self::request_exit) を呼ぶ。
/// 既定の「最後の窓が閉じたら終了」の仕掛けも同じ口を通す（終了の完了機構は 1 本）。
///
/// 指示は 1 度だけ効く。2 回目以降は `debug!` で流し、通知も撃たない。指示済みは戻せない。
///
/// # 契約: 窓は閉じてから指示すること
/// 指示の前に、閉じたい窓を閉じて（despawn して）おくこと。指示の時点で残っていた窓は
/// `WinApp::run` が戻る前に壊すが、その entity の資源（`WindowHandle`・WUC）は `WinApp` の
/// drop まで World に残る。本口は「窓を閉じる前に呼んでよい口」ではない。
#[derive(Clone)]
pub struct AppExit {
    /// 指示済みか（false → true の一方向）。
    requested: Rc<Cell<bool>>,
    /// 終了シグナル。`run()` の待ち（[`ShutdownPolicy::shutdown_future`]）を起こす。
    signal: Rc<Event>,
}

impl AppExit {
    /// 未指示の受け口を作る（`WinApp` と、素の World で終了経路を検査するテストが使う）。
    pub fn new() -> Self {
        Self {
            requested: Rc::new(Cell::new(false)),
            signal: Rc::new(Event::new()),
        }
    }

    /// 終了を指示する。1 回目は指示済みを立て、記録を残して待ちを起こす。2 回目以降は流す。
    ///
    /// 呼ぶ前に窓を閉じておくこと（型の doc の契約を参照）。
    pub fn request_exit(&self) {
        if self.requested.replace(true) {
            debug!("[AppExit] exit already requested — ignored");
            return;
        }
        info!("[AppExit] exit requested");
        ShutdownPolicy::notify_shutdown(&self.signal);
    }

    /// 指示済みか。
    pub fn is_requested(&self) -> bool {
        self.requested.get()
    }

    /// 終了シグナル（`run()` の防御的 notify と wintf 内テストが使う）。
    pub(crate) fn signal(&self) -> &Event {
        &self.signal
    }
}

impl Default for AppExit {
    fn default() -> Self {
        Self::new()
    }
}

/// `block_on` の「loop 先行 quit で panic」を回避する終了規律（設計 ShutdownPolicy）。
///
/// 状態を持たない名前空間。終了の指示の受け口（[`AppExit`]・`WinApp` 所有）から
/// `run()` の `block_on` が待つ shutdown future を組み立てる接点と、tail race を避ける
/// 終了時 notify 規律を提供する。
///
/// # 終了規律
/// `run()` は `block_on(shutdown_future(exit))` でループを駆動し、[`AppExit::request_exit`]
/// （利用側の明示の指示、または既定ポリシーで `WinApp` が registry 空遷移 hook に仕込んだ指示）
/// で shutdown future を完了させ、ループを **先行 quit させず** future 完了で正常復帰させる
/// （`PostQuitMessage` 先撃ちによる "received unexpected quit message" panic を構造的に回避）。
///
/// # tail race
/// shutdown future の `listen()` arm と notify の間でタスク完了直後の wake を取りこぼす
/// 競合（先進坑が観測）に対し、[`notify_shutdown`] を終了時に補助的に撃つ規律を踏襲する。
pub(crate) struct ShutdownPolicy;

impl ShutdownPolicy {
    /// 終了の指示が出るまで完了しない shutdown future を返す（指示済みなら即完了）。
    ///
    /// 順序は「待ち受けを立てる（`listen()` を arm）→ 指示済みなら即完了 → 通知を待つ」。
    /// - 通知はリスナ不在だと失われる（`event-listener` の仕様）ので、ループ開始前に出た指示は
    ///   「指示済み」の 1 ビットで拾う（要件 1.5）。
    /// - arm を確認の **前** に置くので、確認の直後・await 到達前に届いた指示も arm 済みの
    ///   リスナが捕捉する（`AsyncTickTask` と同じ取りこぼし防止規律）。
    ///
    /// `run()` はこの future を [`MessageLoopDriver::block_on`] へ渡す。`block_on` は完了済みの
    /// future でも正常に戻るので、ループ開始前の指示は「ループ開始後ただちに終わる」になる。
    pub(crate) fn shutdown_future(exit: AppExit) -> impl Future<Output = ()> {
        async move {
            let listener = exit.signal().listen();
            if exit.is_requested() {
                return;
            }
            listener.await;
        }
    }

    /// 終了シグナルを全リスナ起床で notify する（tail race 補填の防御的 notify）。
    ///
    /// `event.notify(usize::MAX)` を撃ち、待機中の shutdown future を起床させる。終了時に
    /// 防御的へ複数回撃っても冪等（`event_listener` は arm 済みリスナのみ起床し、未 arm の
    /// notify は次の arm へ持ち越されない＝余分な副作用なし）。[`AppExit::request_exit`] の
    /// 1 回目（正常経路）と、run 側の終了時 tail race 補填の両方が本 helper を撃つ。
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

    // ── AppExit（areka-P0-app-lifetime-separation 1.1） ─────────────────

    /// 待ちが終わらない壊れ方を「止まったまま」でなく赤で返すための見張り。
    ///
    /// `timeout` 経過までに drop されなければ、呼び出しスレッドへ `WM_QUIT` を投げる。
    /// `block_on` は future 完了前の quit で panic する（ライブラリ仕様）ので、待ちが終わらない
    /// 壊れ方はテストの失敗になる。緑の経路では drop が先に来て、何も投げずに見張りを畳む。
    struct LoopWatchdog {
        _cancel: std::sync::mpsc::Sender<()>,
    }

    impl LoopWatchdog {
        fn arm(timeout: std::time::Duration) -> Self {
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            use windows::Win32::System::Threading::GetCurrentThreadId;
            use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

            // SAFETY: 呼び出しスレッドの ID を読むだけ。
            let tid = unsafe { GetCurrentThreadId() };
            let (cancel, cancelled) = std::sync::mpsc::channel::<()>();
            std::thread::spawn(move || {
                // drop（送り手の破棄）が先なら Disconnected で即座に畳む。
                if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) =
                    cancelled.recv_timeout(timeout)
                {
                    // SAFETY: スレッド宛ての WM_QUIT を投げるだけ（引数は定数）。
                    let _ = unsafe { PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0)) };
                }
            });
            Self { _cancel: cancel }
        }
    }

    const WATCHDOG: std::time::Duration = std::time::Duration::from_secs(5);

    /// 1 度だけ `Pending` を返して自分を起こし直す future（UI タスクに順番を譲らせる）。
    struct YieldNow(bool);

    impl Future for YieldNow {
        type Output = ();
        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<()> {
            if self.0 {
                return std::task::Poll::Ready(());
            }
            self.0 = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }

    /// 要件 5.2・2.6: UI タスクから出した終了の指示で、ループの待ちが終わる。
    ///
    /// 本番の tick の中・smoke の `spawn_local` と同じく、待ちが立った **後** に指示が届く経路。
    /// 投入済みのタスクは `block_on` の future より先に回るので、タスクは「待ちが立った」の印を
    /// 見るまで順番を譲ってから指示する（印は待ちを arm するのと同じ poll の中で立つ）。
    /// 指示が待ちを完了させない壊れ方なら、見張りの `WM_QUIT` で `block_on` が panic する。
    #[test]
    fn exit_requested_from_a_ui_task_ends_the_loop() {
        let _watchdog = LoopWatchdog::arm(WATCHDOG);
        let exit = AppExit::new();
        let waiting = Rc::new(Cell::new(false));

        let from_task = exit.clone();
        let seen = Rc::clone(&waiting);
        let _task = crate::executor::spawn_local(async move {
            while !seen.get() {
                YieldNow(false).await;
            }
            from_task.request_exit();
        });

        let armed = Rc::clone(&waiting);
        let shutdown = ShutdownPolicy::shutdown_future(exit.clone());
        MessageLoopDriver::block_on(async move {
            armed.set(true);
            shutdown.await;
        });

        assert!(
            exit.is_requested(),
            "UI タスクの指示が受け口に残っているはず"
        );
    }

    /// 要件 1.5: ループの開始より前に出した終了の指示を取りこぼさず、ループはただちに終わる。
    ///
    /// 通知はリスナ不在だと失われる（`event-listener` の仕様）。「指示済み」の確認を待ちの順から
    /// 外すと、この指示は失われて待ちが終わらない。
    #[test]
    fn exit_requested_before_the_loop_starts_completes_immediately() {
        let _watchdog = LoopWatchdog::arm(WATCHDOG);
        let exit = AppExit::new();
        exit.request_exit();

        MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(exit.clone()));

        assert!(exit.is_requested());
    }

    /// 要件 1.4: 2 回目の指示は失敗せず流され、指示済みは保たれ、ループの待ちは終わる。
    #[test]
    fn second_request_is_ignored_and_the_future_still_completes() {
        let _watchdog = LoopWatchdog::arm(WATCHDOG);
        let exit = AppExit::new();
        exit.request_exit();
        exit.request_exit();
        assert!(exit.is_requested(), "2 回目の指示で指示済みが崩れないはず");

        MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(exit.clone()));

        assert!(exit.is_requested());
    }
}
