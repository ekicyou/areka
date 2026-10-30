//! `answer` のテスト（引き金の対応表・`HTCLIENT` の門・答えごとの `SetCursor` と戻り値・
//! 掛け直しの条件・出し直しの条件・DPI 0・最後に出した形の控え・問い合わせの口が無い世界）。

use super::*;
use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, HTTRANSPARENT, WM_MOUSEMOVE};

fn cursor(n: usize) -> HCURSOR {
    HCURSOR(n as *mut core::ffi::c_void)
}

fn hwnd(n: usize) -> HWND {
    HWND(n as *mut core::ffi::c_void)
}

#[test]
fn trigger_table() {
    let cases = [
        (WM_MOUSEMOVE, CursorTrigger::Move),
        (0, CursorTrigger::Move),
        (0x7FFF, CursorTrigger::Move),
        (WM_LBUTTONDOWN, CursorTrigger::Press { right: false }),
        (WM_MBUTTONDOWN, CursorTrigger::Press { right: false }),
        (WM_XBUTTONDOWN, CursorTrigger::Press { right: false }),
        (WM_LBUTTONDBLCLK, CursorTrigger::Press { right: false }),
        (WM_MBUTTONDBLCLK, CursorTrigger::Press { right: false }),
        (WM_XBUTTONDBLCLK, CursorTrigger::Press { right: false }),
        (WM_RBUTTONDOWN, CursorTrigger::Press { right: true }),
        (WM_RBUTTONDBLCLK, CursorTrigger::Press { right: true }),
        (WM_LBUTTONUP, CursorTrigger::Release),
        (WM_RBUTTONUP, CursorTrigger::Release),
        (WM_MBUTTONUP, CursorTrigger::Release),
        (WM_XBUTTONUP, CursorTrigger::Release),
    ];
    for (msg, want) in cases {
        assert_eq!(trigger_of(msg), want, "msg=0x{msg:X}");
    }
}

#[test]
fn not_htclient_is_not_answered() {
    for hit in [HTCAPTION, HTTRANSPARENT as u32, 0] {
        let plan = plan_set_cursor(hit, Some(CursorReply::Show(cursor(7))), CursorTrigger::Move);
        assert_eq!(
            plan,
            SetCursorPlan {
                set: None,
                result: None,
                rearm_hover: false
            },
            "hit={hit}"
        );
    }
}

#[test]
fn show_sets_and_returns_1() {
    let plan = plan_set_cursor(
        HTCLIENT,
        Some(CursorReply::Show(cursor(7))),
        CursorTrigger::Release,
    );
    assert_eq!(plan.set, Some(cursor(7)));
    assert_eq!(plan.result, Some(LRESULT(1)));
}

#[test]
fn keep_does_not_set_but_stops_default() {
    let plan = plan_set_cursor(HTCLIENT, Some(CursorReply::Keep), CursorTrigger::Move);
    assert_eq!(plan.set, None);
    assert_eq!(plan.result, Some(LRESULT(1)));
    assert!(!plan.rearm_hover);
}

#[test]
fn not_mine_goes_to_default() {
    let plan = plan_set_cursor(HTCLIENT, None, CursorTrigger::Move);
    assert_eq!(plan.set, None);
    assert_eq!(plan.result, None);
    assert!(!plan.rearm_hover);
}

#[test]
fn hover_rearmed_only_on_show_with_move() {
    let show = Some(CursorReply::Show(cursor(7)));
    assert!(plan_set_cursor(HTCLIENT, show, CursorTrigger::Move).rearm_hover);
    for t in [
        CursorTrigger::Press { right: false },
        CursorTrigger::Press { right: true },
        CursorTrigger::Release,
        CursorTrigger::Hover,
        CursorTrigger::Wheel,
        CursorTrigger::Refresh,
    ] {
        assert!(!plan_set_cursor(HTCLIENT, show, t).rearm_hover, "{t:?}");
    }
}

#[test]
fn push_only_when_cursor_is_over_that_window() {
    assert!(push_allowed(hwnd(10), hwnd(10)));
    assert!(!push_allowed(hwnd(11), hwnd(10)));
    assert!(!push_allowed(HWND::default(), hwnd(10)));
}

#[test]
fn dpi_zero_is_96() {
    assert_eq!(dpi_or_default(0), 96);
    assert_eq!(dpi_or_default(144), 144);
}

#[test]
fn unborrowed_answers_with_last_shown() {
    clear_last_shown();
    let w = hwnd(20);
    assert_eq!(plan_unborrowed(HTCLIENT, last_shown(w)).result, None);

    remember_shown(w, cursor(5));
    let plan = plan_unborrowed(HTCLIENT, last_shown(w));
    assert_eq!(plan.set, Some(cursor(5)));
    assert_eq!(plan.result, Some(LRESULT(1)));
    assert!(!plan.rearm_hover);
    // 別の窓の控えは使わない・HTCLIENT でなければ答えない。
    assert_eq!(last_shown(hwnd(21)), None);
    assert_eq!(plan_unborrowed(HTCAPTION, last_shown(w)).result, None);
}

#[test]
fn cleared_last_shown_is_absent() {
    remember_shown(hwnd(30), cursor(1));
    remember_shown(hwnd(31), cursor(2));
    clear_last_shown();
    assert_eq!(last_shown(hwnd(30)), None);
    assert_eq!(last_shown(hwnd(31)), None);
}

#[test]
fn push_is_noop_without_query() {
    clear_last_shown();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let window = world
        .borrow_mut()
        .world_mut()
        .spawn(WindowHandle {
            hwnd: hwnd(40),
            instance: Default::default(),
        })
        .id();
    push_cursor(world.borrow().world(), window, CursorTrigger::Refresh);
    push_cursor_rc(&world, window, CursorTrigger::Release);
    assert_eq!(on_mouse_hover(&world, window), Some(LRESULT(0)));
    assert_eq!(last_shown(hwnd(40)), None);
}
