//! host-32 先進坑: 親（x64）と helper（i686）が両方使う WM_COPYDATA の送り・受けの決まり。
//!
//! 親（`main.rs`）と helper（`helper.rs`）の双方から `#[path = "ipc.rs"] mod ipc;` で
//! 取り込まれる。決まりを一箇所に置くことで、x64⇄i686 の跨ビットネス規約のズレを防ぐ。
//!
//! - `dwData` の下 32 ビット＝メッセージ種別タグ（`MsgTag`）。
//! - `cbData`＝中身の長さ。中身は生のバイト列で、固定ヘッダを持たない。ポインタ・HANDLE・
//!   struct は載せない（跨ビットネスで有効なのは生のバイト列だけ）。
//! - HWND は下 32 ビット（`hwnd_to_u32`）の `to_le_bytes` で運ぶ。
//! - 送りは `SendMessageTimeoutW`（`SMTO_ABORTIFHUNG` ＋上限）で、ハングしない。

// helper だけが使う DEFAULT_TIMEOUT が残る間は外せない。
#![allow(dead_code)]

use core::cell::Cell;
use core::time::Duration;
use std::time::Instant;

use windows::Win32::Foundation::{ERROR_TIMEOUT, GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{
    KillTimer, SMTO_ABORTIFHUNG, SendMessageTimeoutW, SetTimer, WM_COPYDATA,
};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ============================================================================
// 1. メッセージ種別タグ（MsgTag）
// ============================================================================

/// `WM_COPYDATA` の `COPYDATASTRUCT.dwData` に載せるメッセージ種別タグ。
///
/// `dwData` は `ULONG_PTR`（x64=64bit / i686=32bit）だが、タグは **下 32 ビットだけ**を使う。
/// 判別子 2 は欠番（値は変えない）。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgTag {
    /// helper → 親: メッセージ窓 HWND ハンドシェイク（中身 = `hwnd_to_u32(..).to_le_bytes()`）。
    Hello = 1,
    /// 親 → helper: SHIORI/3.0 リクエスト（中身 = 生 SHIORI3 バイト列）。
    Request = 3,
    /// helper → 親: SHIORI/3.0 応答（中身 = 生 SHIORI3 バイト列）。
    Response = 4,
    /// 親 → helper: `pasta.dll` アンロード指示。
    Unload = 5,
}

impl MsgTag {
    /// `dwData` の下 32 ビットへ載せる判別子。
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    /// `dwData` の下 32 ビットから `MsgTag` を戻す。未知の値は `Err(raw)`。
    pub fn try_from_u32(raw: u32) -> Result<Self, u32> {
        match raw {
            x if x == MsgTag::Hello.as_u32() => Ok(MsgTag::Hello),
            x if x == MsgTag::Request.as_u32() => Ok(MsgTag::Request),
            x if x == MsgTag::Response.as_u32() => Ok(MsgTag::Response),
            x if x == MsgTag::Unload.as_u32() => Ok(MsgTag::Unload),
            other => Err(other),
        }
    }
}

// ============================================================================
// 2. HWND の下 32 ビット
// ============================================================================

/// HWND の下 32 ビット（USER ハンドルは 32 ビットで足りる）。HELLO の中身はこれの `to_le_bytes`。
#[inline]
pub fn hwnd_to_u32(hwnd: HWND) -> u32 {
    hwnd.0 as usize as u32
}

/// 下 32 ビットの値をこのプロセスのポインタ幅の `HWND` へ戻す。x64 では上を 0 で埋める。
#[inline]
pub fn hwnd_from_u32(value: u32) -> HWND {
    HWND(value as usize as *mut core::ffi::c_void)
}

// ============================================================================
// 3. 送り（SendMessageTimeoutW）と受け（COPYDATASTRUCT の取り出し）
// ============================================================================

/// 送りの失敗の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// `GetLastError() == ERROR_TIMEOUT`（上限に達した・`SMTO_ABORTIFHUNG` の打ち切り）。
    Timeout,
    /// ほかの Win32 エラー（窓が無い 1400 など）。値は `GetLastError` のまま。
    SendFailed(u32),
}

/// 送出のデフォルトタイムアウト。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// WM_COPYDATA を 1 通送る。`dwData`＝`tag` の下 32 ビット・`cbData`＝`payload` の長さ・
/// `wParam`＝`self_hwnd`。`SMTO_ABORTIFHUNG` ＋ `timeout` で、相手が詰まってもハングしない。
///
/// 送りが 0 を返したら直後に 1 回だけ `GetLastError` を読み、`ERROR_TIMEOUT` なら
/// `Err(Timeout)`、ほかは `Err(SendFailed(code))` を返す。応答は待たない。
pub fn send_copydata(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), IpcError> {
    // lpData は payload を指すだけで、ポインタ自体はワイヤに載らない（OS が cbData バイトを写す）。
    let cds = COPYDATASTRUCT {
        dwData: tag.as_u32() as usize,
        cbData: payload.len() as u32,
        lpData: payload.as_ptr() as *mut core::ffi::c_void,
    };

    let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;

    let mut result: usize = 0;
    // SAFETY: &cds と payload は本呼び出しの間生きている。target が無効なら OS がエラーを返す。
    let ret: LRESULT = unsafe {
        SendMessageTimeoutW(
            target,
            WM_COPYDATA,
            WPARAM(self_hwnd.0 as usize),
            LPARAM(&cds as *const COPYDATASTRUCT as isize),
            SMTO_ABORTIFHUNG,
            timeout_ms,
            Some(&mut result as *mut usize),
        )
    };

    if ret.0 != 0 {
        return Ok(());
    }
    // 送りの直後に 1 回だけ読む（間に別の Win32 呼び出しを挟まない）。
    // SAFETY: 引数なしの Win32 呼び出し。
    let code = unsafe { GetLastError() };
    if code == ERROR_TIMEOUT {
        Err(IpcError::Timeout)
    } else {
        Err(IpcError::SendFailed(code.0))
    }
}

/// 受けた WM_COPYDATA の LPARAM から（タグ＝`dwData` の下 32 ビット, 中身の写し）を取り出す。
/// `lparam` が null なら `None`。`cbData` が 0 なら中身は空。
///
/// # Safety
/// `lparam` は WM_COPYDATA の LPARAM（OS が `COPYDATASTRUCT` を指すことを約束する）か null。
pub unsafe fn copydata_payload(lparam: LPARAM) -> Option<(u32, Vec<u8>)> {
    let cds = lparam.0 as *const COPYDATASTRUCT;
    if cds.is_null() {
        return None;
    }
    // SAFETY: 呼び元の約束により cds は有効な COPYDATASTRUCT を指す。
    let cds = unsafe { &*cds };
    let tag_raw = (cds.dwData as u64 & 0xFFFF_FFFF) as u32;
    let len = cds.cbData as usize;
    let payload = if len == 0 || cds.lpData.is_null() {
        Vec::new()
    } else {
        // SAFETY: lpData は cbData バイト有効（OS が写した受け側の写し）。
        unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, len).to_vec() }
    };
    Some((tag_raw, payload))
}

// ============================================================================
// 4. 待ち（pump_until）
// ============================================================================

/// 待ちのタイマーの ID（窓ごとの ID なので、窓の持ち主のほかのタイマーと重ならない値）。
const PUMP_TIMER_ID: usize = 0x5049;
/// ループを起こす間隔。
const PUMP_TICK_MS: u32 = 50;

/// `hwnd` の窓に `SetTimer`（50 ms）を掛けて `MessageLoop::run` を回し、`done()` が真になるか
/// `deadline` で抜ける。抜けた後に `KillTimer`。戻り値は抜けたときの `done()`。
///
/// `hwnd` はこのスレッドが作った窓で、ほかの `MessageLoop::run` のフィルタの中から呼ばない。
/// 無入力でも `WM_TIMER` がループを起こすので、起こし役のスレッドは要らない。
pub fn pump_until(hwnd: HWND, deadline: Instant, done: impl Fn() -> bool) -> bool {
    let first = done();
    if first || Instant::now() >= deadline {
        return first;
    }
    // SAFETY: hwnd はこのスレッドが作った窓（呼び元の約束）。TIMERPROC 無しなので WM_TIMER が届く。
    if unsafe { SetTimer(Some(hwnd), PUMP_TIMER_ID, PUMP_TICK_MS, None) } == 0 {
        // SAFETY: 引数なしの Win32 呼び出し。
        let code = unsafe { GetLastError() };
        eprintln!("[ipc] SetTimer に失敗（待たずに戻る）: {}", code.0);
        return first;
    }
    let last = Cell::new(first);
    MessageLoop::run(|msg_loop, _msg| {
        let d = done();
        last.set(d);
        if d || Instant::now() >= deadline {
            msg_loop.quit();
        }
        FilterResult::Forward
    });
    // SAFETY: 上で掛けた同じ窓・同じ ID のタイマーを外す。
    if let Err(e) = unsafe { KillTimer(Some(hwnd), PUMP_TIMER_ID) } {
        eprintln!("[ipc] KillTimer に失敗: {e}");
    }
    last.get()
}

// ============================================================================
// 単体テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
    use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};

    /// 判別子の値（1・3・4・5）が下 32 ビットに収まり、u32 を往復すること。
    #[test]
    fn msgtag_discriminants_roundtrip() {
        for (tag, raw) in [
            (MsgTag::Hello, 1u32),
            (MsgTag::Request, 3),
            (MsgTag::Response, 4),
            (MsgTag::Unload, 5),
        ] {
            assert_eq!(tag.as_u32(), raw, "{tag:?} の判別子");
            assert_eq!(MsgTag::try_from_u32(raw), Ok(tag), "{tag:?} の往復");
        }
    }

    /// 未知の判別子（欠番の 2 を含む）は `Err(raw)`。
    #[test]
    fn msgtag_unknown_discriminant_is_err() {
        for raw in [0u32, 2, 6, u32::MAX] {
            assert_eq!(MsgTag::try_from_u32(raw), Err(raw));
        }
    }

    /// HWND の下 32 ビットが往復し、HELLO の中身（to_le_bytes）からも戻ること。
    #[test]
    fn hwnd_u32_roundtrips() {
        for sample in [0x0000_0001u32, 0x1234_5678u32, 0xDEAD_BEEFu32, u32::MAX] {
            let hwnd = hwnd_from_u32(sample);
            assert_eq!(hwnd_to_u32(hwnd), sample, "{sample:#010x} の往復");
            let bytes = hwnd_to_u32(hwnd).to_le_bytes();
            assert_eq!(
                u32::from_le_bytes(bytes),
                sample,
                "{sample:#010x} の LE 往復"
            );
        }
    }

    /// 手で組んだ COPYDATASTRUCT からタグが下 32 ビットになり、中身が写ること。
    #[test]
    fn copydata_payload_takes_low_32_bits_and_copies_bytes() {
        let body = b"GET SHIORI/3.0\r\n\r\n";
        // x64 では dwData の上 32 ビットに値を入れる（i686 では切り捨てられて 3 だけになる）。
        let cds = COPYDATASTRUCT {
            dwData: ((0xABCD_u64 << 32) | 3) as usize,
            cbData: body.len() as u32,
            lpData: body.as_ptr() as *mut core::ffi::c_void,
        };
        // SAFETY: cds は本ブロックの間生きている COPYDATASTRUCT。
        let got = unsafe { copydata_payload(LPARAM(&cds as *const COPYDATASTRUCT as isize)) };
        assert_eq!(got, Some((3, body.to_vec())));

        // cbData 0 は空。
        let empty = COPYDATASTRUCT {
            dwData: 5,
            cbData: 0,
            lpData: core::ptr::null_mut(),
        };
        // SAFETY: 同上。
        let got = unsafe { copydata_payload(LPARAM(&empty as *const COPYDATASTRUCT as isize)) };
        assert_eq!(got, Some((5, Vec::new())));

        // null の LPARAM は None。
        // SAFETY: null は約束の範囲（None を返す）。
        assert_eq!(unsafe { copydata_payload(LPARAM(0)) }, None);
    }

    /// 作って壊した窓へ送ると、窓が無いエラー（1400）の送りの失敗になる（時間切れと呼ばない）。
    #[test]
    fn send_to_destroyed_window_is_send_failed_1400() {
        let window = Window::new_checked(
            WindowType::MessageOnly,
            (),
            |_s: Pin<&()>, _m: WindowMessage| -> Option<LRESULT> { None },
        )
        .expect("message-only 窓を作れる");
        let hwnd = window.hwnd();
        drop(window); // DestroyWindow

        let got = send_copydata(
            hwnd,
            HWND::default(),
            MsgTag::Request,
            b"x",
            Duration::from_millis(100),
        );
        assert_eq!(got, Err(IpcError::SendFailed(1400)));
    }

    /// 別スレッドで WM_COPYDATA を受けると 1 秒眠る窓へ 100 ms で送ると時間切れ。
    #[test]
    fn send_to_sleeping_window_is_timeout() {
        let (tx, rx) = mpsc::channel::<u32>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let sleeper = std::thread::spawn(move || {
            let window = Window::new_checked(
                WindowType::MessageOnly,
                (),
                |_s: Pin<&()>, m: WindowMessage| -> Option<LRESULT> {
                    if m.msg == WM_COPYDATA {
                        std::thread::sleep(Duration::from_secs(1));
                        return Some(LRESULT(0));
                    }
                    None
                },
            )
            .expect("message-only 窓を作れる");
            tx.send(hwnd_to_u32(window.hwnd())).unwrap();
            MessageLoop::run(|msg_loop, _msg| {
                if stop_thread.load(Ordering::Relaxed) {
                    msg_loop.quit();
                }
                FilterResult::Forward
            });
            drop(window);
        });
        let target = hwnd_from_u32(rx.recv_timeout(Duration::from_secs(5)).unwrap());

        let started = Instant::now();
        let got = send_copydata(
            target,
            HWND::default(),
            MsgTag::Request,
            b"x",
            Duration::from_millis(100),
        );
        let elapsed = started.elapsed();

        stop.store(true, Ordering::Relaxed);
        // SAFETY: target は sleeper スレッドが持つ窓。WM_NULL でループを起こすだけ。
        unsafe {
            let _ = PostMessageW(Some(target), 0, WPARAM(0), LPARAM(0));
        }
        sleeper.join().unwrap();

        assert_eq!(got, Err(IpcError::Timeout));
        assert!(
            elapsed < Duration::from_millis(900),
            "眠りの 1 秒を待たずに戻る: {elapsed:?}"
        );
    }

    fn message_only_window() -> Window<()> {
        Window::new_checked(
            WindowType::MessageOnly,
            (),
            |_s: Pin<&()>, _m: WindowMessage| -> Option<LRESULT> { None },
        )
        .expect("message-only 窓を作れる")
    }

    /// 条件が偽のままなら、起こし役のスレッド無しで約 150 ms の期限に偽で抜ける。
    #[test]
    fn pump_until_returns_false_at_deadline_without_waker_thread() {
        let window = message_only_window();
        let started = Instant::now();
        let got = pump_until(window.hwnd(), started + Duration::from_millis(150), || {
            false
        });
        let elapsed = started.elapsed();
        assert!(!got);
        assert!(
            elapsed >= Duration::from_millis(150) && elapsed < Duration::from_secs(2),
            "期限の付近で抜ける: {elapsed:?}"
        );
    }

    /// 条件が 3 回目の呼び出しで真になると、期限（5 秒）よりずっと前に真で抜ける。
    #[test]
    fn pump_until_returns_true_soon_after_condition_holds() {
        let window = message_only_window();
        let calls = core::cell::Cell::new(0u32);
        let started = Instant::now();
        let got = pump_until(window.hwnd(), started + Duration::from_secs(5), || {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        });
        let elapsed = started.elapsed();
        assert!(got);
        assert_eq!(calls.get(), 3, "真になった後は呼ばない");
        assert!(
            elapsed < Duration::from_secs(1),
            "期限を待たずに抜ける: {elapsed:?}"
        );
    }
}
