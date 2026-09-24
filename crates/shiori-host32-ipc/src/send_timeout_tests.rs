//! 送出の期限切れと送出失敗の分類（`send_copydata_with` の戻り 0 の写し）。
//!
//! 別スレッドの message-only 窓が `WM_COPYDATA` の手続きの中で期限より長く眠る → 短い期限で
//! 送ると [`IpcError::Timeout`]。既に壊した窓へ送ると [`IpcError::SendFailed`]。
//! 戻り 0 を一律 `SendFailed` に写す形へ戻すと前者が赤になる。

use super::*;

use std::sync::Once;
use std::sync::mpsc;
use std::thread::JoinHandle;

use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE,
    WM_DESTROY, WNDCLASSW,
};
use windows::core::w;

/// 受け手の手続きが `WM_COPYDATA` の中で眠る時間。送り手の期限（[`SEND_TIMEOUT`]）より十分長い。
const HANDLER_SLEEP: Duration = Duration::from_millis(300);
/// 送り手の期限。
const SEND_TIMEOUT: Duration = Duration::from_millis(20);
/// 窓のスレッドの終わりを待つ上限。
const JOIN_BOUND: Duration = Duration::from_secs(5);
/// 本テスト専用の窓クラス名。
const CLASS: windows::core::PCWSTR = w!("shiori-host32-ipc-send-timeout-tests");

unsafe extern "system" fn sleepy_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // lparam の中身には触れない（送り手は期限切れで先に戻り、payload はもう無い）。
        WM_COPYDATA => {
            std::thread::sleep(HANDLER_SLEEP);
            LRESULT(1)
        }
        WM_DESTROY => {
            // SAFETY: Win32 境界。自スレッドのループへ終了を積む。
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: Win32 境界。既定の手続きへ委譲する（WM_CLOSE は窓を壊す）。
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 眠る窓を別スレッドに建て、その HWND（数値）と終わりの知らせを返す。
fn spawn_sleepy_window() -> (HWND, mpsc::Receiver<()>, JoinHandle<()>) {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(sleepy_wnd_proc),
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: Win32 境界。プロセス固有名のクラスを 1 度だけ登録する。
        assert_ne!(unsafe { RegisterClassW(&wc) }, 0, "RegisterClassW");
    });

    let (hwnd_tx, hwnd_rx) = mpsc::channel::<usize>();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        // SAFETY: Win32 境界。message-only 窓を作り、WM_QUIT までこのスレッドで回す。
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS,
                w!("send-timeout"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
            .expect("message-only 窓の生成に失敗した");
            hwnd_tx
                .send(hwnd.0 as usize)
                .expect("hwnd の受け手が居ない");
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                DispatchMessageW(&msg);
            }
        }
        let _ = done_tx.send(());
    });
    let raw = hwnd_rx.recv_timeout(JOIN_BOUND).expect("窓が建たなかった");
    (HWND(raw as *mut core::ffi::c_void), done_rx, handle)
}

/// 窓を閉じ、スレッドの終わりを上限つきで待つ。
fn close_and_join(hwnd: HWND, done_rx: mpsc::Receiver<()>, handle: JoinHandle<()>) {
    // SAFETY: Win32 境界。WM_CLOSE → DefWindowProcW が窓を壊し WM_DESTROY で抜ける。
    unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }.expect("PostMessageW");
    done_rx
        .recv_timeout(JOIN_BOUND)
        .expect("窓のスレッドが上限内に終わらない");
    handle.join().expect("窓のスレッドが panic した");
}

#[test]
fn send_to_a_window_that_does_not_return_in_time_is_timeout() {
    let (hwnd, done_rx, handle) = spawn_sleepy_window();
    let result = send_copydata(hwnd, hwnd, MsgTag::Request, b"late", SEND_TIMEOUT);
    close_and_join(hwnd, done_rx, handle);
    assert!(
        matches!(result, Err(IpcError::Timeout)),
        "期限内に返らなかった送出は Timeout: got {result:?}"
    );
}

#[test]
fn send_to_a_destroyed_window_is_send_failed() {
    let (hwnd, done_rx, handle) = spawn_sleepy_window();
    close_and_join(hwnd, done_rx, handle);
    let result = send_copydata(hwnd, hwnd, MsgTag::Request, b"gone", SEND_TIMEOUT);
    assert!(
        matches!(result, Err(IpcError::SendFailed)),
        "存在しない窓への送出は SendFailed: got {result:?}"
    );
}
