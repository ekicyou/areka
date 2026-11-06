//! メッセージループ層。自作 `PeekMessageW` ポンプを撤去し、ライブラリ
//! （`wintf-winmsg-executor`）の `block_on` / `MessageLoop::run` へ委譲する。
//!
//! 本層は終了の受け口（[`AppExit`]）と、`WinApp::run` がライブラリの [`block_on`](crate::executor::block_on) へ渡す
//! shutdown future（[`ShutdownPolicy`]）を提供する。OS メッセージはライブラリのループが
//! ウィンドウ手続きへ配る。
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
//! 場合は [`block_on`](crate::executor::block_on) によるネストが正規経路となる。

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

use event_listener::Event;
use tracing::{debug, info};

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
        // 待っている shutdown future を全部起こす（未 arm の notify は持ち越されない）。
        self.signal.notify(usize::MAX);
    }

    /// 指示済みか。
    pub fn is_requested(&self) -> bool {
        self.requested.get()
    }

    /// 終了シグナル（[`ShutdownPolicy::shutdown_future`] の待ちと wintf 内テストが使う）。
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
/// `run()` の `block_on` が待つ shutdown future を組み立てる。
///
/// # 終了規律
/// `run()` は `block_on(shutdown_future(exit))` でループを駆動し、[`AppExit::request_exit`]
/// （利用側の明示の指示、または既定ポリシーで `WinApp` が registry 空遷移 hook に仕込んだ指示）
/// で shutdown future を完了させ、ループを **先行 quit させず** future 完了で正常復帰させる
/// （`PostQuitMessage` 先撃ちによる "received unexpected quit message" panic を構造的に回避）。
pub(crate) struct ShutdownPolicy;

impl ShutdownPolicy {
    /// 終了の指示が出るまで完了しない shutdown future を返す（指示済みなら即完了）。
    ///
    /// 順序は「待ち受けを立てる（`listen()` を arm）→ 指示済みなら即完了 → 通知を待つ」。
    /// - 通知はリスナ不在だと失われる（`event-listener` の仕様）ので、ループ開始前に出た指示は
    ///   「指示済み」の 1 ビットで拾う（要件 1.5）。
    /// - arm を確認の **前** に置くので、確認の直後・await 到達前に届いた指示も arm 済みの
    ///   リスナが捕捉する（tick のタスク `run_async_tick` と同じ取りこぼし防止規律）。
    ///
    /// `run()` はこの future を [`block_on`](crate::executor::block_on) へ渡す。`block_on` は完了済みの
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::block_on;

    // ── ShutdownPolicy (task 4.2) ─────────────────────────────────

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
        block_on(async move {
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

        block_on(ShutdownPolicy::shutdown_future(exit.clone()));

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

        block_on(ShutdownPolicy::shutdown_future(exit.clone()));

        assert!(exit.is_requested());
    }
}
