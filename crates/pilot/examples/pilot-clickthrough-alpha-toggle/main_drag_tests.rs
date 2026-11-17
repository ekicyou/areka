//! ドラッグの終わり（指摘 1）のテスト。`main.rs` の末尾から `drag_tests` として読む。
//!
//! 見えない窓（`ShowWindow` しない・DComp を組まない）を作り、ドラッグ中の印を立てて
//! 終わりの合図を窓の手続きへ直接送る。

use super::*;
use event_listener::Listener;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

/// ドラッグ中の窓へ `msg` を送り、印が下りて起こしが鳴ったことを確かめる。
fn assert_drag_ends_on(msg: u32) {
    let state_changed = Arc::new(Event::new());
    let dragging = Arc::new(AtomicBool::new(true));
    let window = make_window(Rc::new(AppState::new(
        state_changed.clone(),
        dragging.clone(),
    )));
    let listener = state_changed.listen();

    // SAFETY: Win32 境界。自分のスレッドが作った有効な窓へ同期で送る。
    unsafe {
        SendMessageW(window.hwnd(), msg, Some(WPARAM(0)), Some(LPARAM(0)));
    }

    assert!(
        !dragging.load(Ordering::Acquire),
        "ドラッグ中の印が下りていない"
    );
    assert!(
        listener.wait_timeout(Duration::ZERO).is_some(),
        "先に張った起こしが鳴っていない"
    );
}

#[test]
fn capture_lost_ends_drag() {
    assert_drag_ends_on(WM_CAPTURECHANGED);
}

#[test]
fn button_up_ends_drag() {
    assert_drag_ends_on(WM_LBUTTONUP);
}
