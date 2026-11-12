//! 実走のポンプを期限つきで回す道具（`toy_ui_pump_test`・`ui_span_test` が `mod ui_pump_support;` で読む）。
//!
//! 呼んだスレッドで `MessageLoop::run` を回す。`spawn_ui` は同じスレッドで先に呼んでおくこと
//! （drain タスクは呼んだスレッドの executor 窓へ積まれる）。窓は作らない（executor の
//! message-only 窓だけ）ので、実の画面も机も要らない。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_NULL};

use wintf_winmsg_executor::{FilterResult, MessageLoop};

/// 心拍（別スレッドからの `PostThreadMessageW(WM_NULL)`）の間隔。無入力でも `GetMessageW` を
/// 起こし、filter が `done` と期限を見直せるようにする（期限つきの要）。
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(25);

/// 呼んだスレッドでポンプを回し、`done` が立つか `deadline` が過ぎたら止めて戻る。
///
/// filter は全メッセージを `Forward` する（wake は drain タスクの再開に要る）。`done` を見て
/// 止めるのは filter の中＝タスクの poll の外。
pub fn run_pump_until(done: &AtomicBool, deadline: Duration) {
    // SAFETY: GetCurrentThreadId は引数なしで今のスレッドの ID を返すだけ。
    let pump_thread_id = unsafe { GetCurrentThreadId() };

    let hb_stop = Arc::new(AtomicBool::new(false));
    let heartbeat = {
        let hb_stop = Arc::clone(&hb_stop);
        std::thread::Builder::new()
            .name("ui-pump-heartbeat".to_owned())
            .spawn(move || {
                while !hb_stop.load(Ordering::Acquire) {
                    // SAFETY: WM_NULL は無害な起こし用のスレッドメッセージ。宛先はポンプを回して
                    // いる最中のスレッド。キューがまだ無い一瞬の送り損ねは無害なので捨てる。
                    unsafe {
                        let _ = PostThreadMessageW(pump_thread_id, WM_NULL, WPARAM(0), LPARAM(0));
                    }
                    std::thread::sleep(HEARTBEAT_INTERVAL);
                }
            })
            .expect("heartbeat thread must start")
    };

    let deadline = Instant::now() + deadline;
    MessageLoop::run(|msg_loop, _msg| {
        if done.load(Ordering::Acquire) || Instant::now() >= deadline {
            msg_loop.quit();
        }
        FilterResult::Forward
    });

    hb_stop.store(true, Ordering::Release);
    let _ = heartbeat.join();
}
