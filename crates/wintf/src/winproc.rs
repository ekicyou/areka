#![allow(deprecated)]

use crate::win_message_handler::*;
use std::ffi::c_void;
use std::sync::*;
use tracing::{debug, info};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub(crate) trait WinMessageHandlerIntoBoxedPtr {
    fn into_boxed_ptr(self) -> *mut c_void;
}

impl WinMessageHandlerIntoBoxedPtr for Arc<dyn BaseWinMessageHandler> {
    fn into_boxed_ptr(self) -> *mut c_void {
        Box::into_raw(Box::new(self)) as _
    }
}

// NOTE(健全性): 本関数には2つの既知の健全性違反がある（codebase-review-loop P28 として修正提案済み）。
// (a) into_boxed_ptr は Box<Arc<dyn BaseWinMessageHandler>> として格納するが、ここでは
//     *mut Arc<dyn WinMessageHandler>（別トレイトのファットポインタ）として読み出す型混同。
// (b) #[allow(mutable_transmutes)] による &dyn → &mut dyn の transmute（共有参照からの
//     可変参照生成 = 未定義動作領域）。
// テスト未保護の unsafe 経路のため、本ループでは修正せず記録に留める（R5.5 / R2.8）。
//
// 【W1-V 影響範囲分析】現行ランタイムで本関数へ到達する経路は2系統:
//   (1) areka 本体・ECS 系 examples: メッセージ専用ウィンドウ（win_thread_mgr.rs の new()）は
//       lpParam なしで生成され GWLP_USERDATA が null のまま → 本関数は常に None を返し、
//       (a)(b) の UB コードは一切実行されない（unit test で null 経路を固定済み）。
//   (2) レガシー create_window 利用者（examples/dcomp_demo.rs のみ）: 全メッセージ dispatch で
//       (a)(b) が実行される。現状「動作して見える」のは、ファットポインタ（データ + vtable）の
//       ビット列が格納時のまま保存され、誤型の中間参照 &dyn WinMessageHandler ではメソッドを
//       一切呼ばず、最後の transmute で格納時の型 BaseWinMessageHandler へ戻ってから
//       message_handler を呼ぶため（格納時の vtable がそのまま使われる）。
//       ただしハンドラ実装が同一ウィンドウへの同期メッセージ送信（SendMessageW・SetWindowPos・
//       DefWindowProcW の一部メッセージ等）を行うと wndproc が再入し、同一ハンドラへの &mut が
//       同時に2つ生存する（エイリアスされた可変参照 = 即 UB）。修正は P27（経路ごと削除・第一
//       推奨）または P28（型統一 + 内部可変性への置き換え）を参照。
fn get_boxed_ptr<'a>(ptr: *mut c_void) -> Option<&'a mut dyn BaseWinMessageHandler> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let raw: *mut Arc<dyn WinMessageHandler> = ptr as _;
        let handler = &**raw;
        #[allow(mutable_transmutes)]
        let handler = std::mem::transmute::<_, &mut dyn BaseWinMessageHandler>(handler);
        Some(handler)
    }
}

fn from_boxed_ptr(ptr: *mut c_void) -> Option<Arc<dyn BaseWinMessageHandler>> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let raw: *mut Arc<dyn BaseWinMessageHandler> = ptr as _;
        let boxed = Box::from_raw(raw);
        Some(*boxed)
    }
}

pub(crate) extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_NCCREATE => {
                debug!(hwnd = ?hwnd, "WM_NCCREATE");
                let cs = lparam.0 as *const CREATESTRUCTW;
                if cs.is_null() {
                    return LRESULT(0);
                }
                let ptr = (*cs).lpCreateParams;
                if let Some(handler) = get_boxed_ptr(ptr) {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as _);
                    handler.message_handler(hwnd, message, wparam, lparam);
                }
                LRESULT(1)
            }
            WM_NCDESTROY => {
                debug!(hwnd = ?hwnd, "WM_NCDESTROY");
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as _;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = from_boxed_ptr(ptr);
                LRESULT(1)
            }
            crate::win_thread_mgr::WM_LAST_WINDOW_DESTROYED => {
                // 最後のウィンドウが閉じられた通知を受け取ったらアプリケーションを終了
                info!("Received WM_LAST_WINDOW_DESTROYED, posting quit message");
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as _;
                if let Some(handler) = get_boxed_ptr(ptr) {
                    handler.message_handler(hwnd, message, wparam, lparam)
                } else {
                    DefWindowProcW(hwnd, message, wparam, lparam)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意: get_boxed_ptr の非 null 経路（P28 の健全性違反コード）はテストから意図的に
    // 実行しない（既知の UB をテストプロセスで踏まないため）。ここでは健全なペアである
    // into_boxed_ptr / from_boxed_ptr（格納・解放）と、null ガード・wndproc の
    // ハンドラ未登録経路（areka 本体が実行時に通る唯一の経路）のみを固定する。

    struct TestHandler;
    impl BaseWinMessageHandler for TestHandler {
        fn message_handler(
            &mut self,
            _hwnd: HWND,
            _msg: u32,
            _wparam: WPARAM,
            _lparam: LPARAM,
        ) -> LRESULT {
            LRESULT(0)
        }
    }

    #[test]
    fn get_boxed_ptr_with_null_returns_none() {
        assert!(get_boxed_ptr(std::ptr::null_mut()).is_none());
    }

    #[test]
    fn from_boxed_ptr_with_null_returns_none() {
        assert!(from_boxed_ptr(std::ptr::null_mut()).is_none());
    }

    #[test]
    fn into_from_boxed_ptr_roundtrip_preserves_identity_and_refcount() {
        let original: Arc<dyn BaseWinMessageHandler> = Arc::new(TestHandler);
        let stored = Arc::clone(&original);
        assert_eq!(Arc::strong_count(&original), 2);

        // 格納: Box 化は Arc の強参照数を変えない（所有権の移動のみ）
        let ptr = stored.into_boxed_ptr();
        assert!(!ptr.is_null());
        assert_eq!(Arc::strong_count(&original), 2);

        // 解放: 同一の Arc が復元され、強参照数も保存される
        let restored = from_boxed_ptr(ptr).expect("non-null pointer must round-trip");
        assert!(Arc::ptr_eq(&original, &restored));
        assert_eq!(Arc::strong_count(&original), 2);

        drop(restored);
        assert_eq!(Arc::strong_count(&original), 1);
    }

    #[test]
    fn wndproc_nccreate_with_null_createstruct_fails_creation() {
        // CREATESTRUCT が null の WM_NCCREATE は LRESULT(0)（ウィンドウ生成中止）を返す
        let r = wndproc(HWND::default(), WM_NCCREATE, WPARAM(0), LPARAM(0));
        assert_eq!(r, LRESULT(0));
    }

    #[test]
    fn wndproc_ncdestroy_without_stored_handler_returns_1_without_crash() {
        // GWLP_USERDATA 未設定（null）→ from_boxed_ptr(None) で二重解放なし
        let r = wndproc(HWND::default(), WM_NCDESTROY, WPARAM(0), LPARAM(0));
        assert_eq!(r, LRESULT(1));
    }

    #[test]
    fn wndproc_unknown_message_without_handler_falls_back_to_defwindowproc() {
        // areka 本体のメッセージ専用ウィンドウが通る経路: ハンドラ未登録 → DefWindowProcW
        let r = wndproc(HWND::default(), WM_NULL, WPARAM(0), LPARAM(0));
        assert_eq!(r, LRESULT(0));
    }
}
