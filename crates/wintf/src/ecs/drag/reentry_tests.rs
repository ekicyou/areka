//! `reentry.rs` の判断の分かれ目のテスト（5 種の見分け・種類ごとの扱い・位置を読めないとき・
//! 記録の欄と level・起床の旗）。
//!
//! 入口を通る道（借用中の入口から届くか・配る段で終了の知らせが 1 件か・画面の座標が
//! 借用なしと同じか）は `runtime/wndproc_bridge_drag_{cancel,release}_tests.rs` が見る。
//! ここは World を作らず、状態を直に置いて関数だけを呼ぶ。
//!
//! `DRAG_STATE` は `thread_local!` なので、各テストは休む状態へ戻してから始める。
//! 捕捉の窓は空の `hwnd`（`SetCapture`／`ReleaseCapture` は実質何もしない）。

use std::time::Instant;

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_SPACE};
use windows::Win32::UI::WindowsAndMessaging::{
    WA_ACTIVE, WM_ACTIVATE, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KEYDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE,
};

use super::handle_message_while_world_busy;
use crate::ecs::drag::{DragState, DragStateSnapshot, snapshot_drag_state, update_drag_state};
use crate::ecs::drag::{start_dragging, start_preparing};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::world::tick_wake;
use crate::executor::util::WindowMessage;

const PRESS: PhysicalPoint = PhysicalPoint { x: 10, y: 20 };
const MOVED: PhysicalPoint = PhysicalPoint { x: 40, y: 70 };

fn target() -> Entity {
    Entity::from_raw_u32(7).expect("valid entity index")
}

fn window() -> Entity {
    Entity::from_raw_u32(3).expect("valid entity index")
}

/// 休む状態へ戻す（捕捉の守りは借用を返してから落とす）。
fn force_idle() {
    let _guard = update_drag_state(|state| match std::mem::replace(state, DragState::Idle) {
        DragState::Preparing { capture_guard, .. }
        | DragState::JustStarted { capture_guard, .. }
        | DragState::Dragging { capture_guard, .. } => Some(capture_guard),
        _ => None,
    });
}

/// 押した（閾値前）。
fn prepare() {
    force_idle();
    start_preparing(target(), PRESS, HWND::default());
}

/// 押して閾値を越えた（開始済み）。
fn start() {
    prepare();
    start_dragging(MOVED);
}

fn message(msg: u32, wparam: usize, lparam: isize) -> WindowMessage {
    WindowMessage {
        hwnd: HWND::default(),
        msg,
        wparam: WPARAM(wparam),
        lparam: LPARAM(lparam),
    }
}

/// 取り消し 3 種＋捕捉の喪失（どれも押した位置・取り消しの印つきで休ませる）。
fn cancel_kinds() -> [(&'static str, WindowMessage); 4] {
    [
        ("esc", message(WM_KEYDOWN, VK_ESCAPE.0 as usize, 0)),
        ("cancelmode", message(WM_CANCELMODE, 0, 0)),
        ("deactivate", message(WM_ACTIVATE, 0, 0)), // WA_INACTIVE
        ("capture_lost", message(WM_CAPTURECHANGED, 0, 0)),
    ]
}

/// 5 種でないもの（ESC でない押下・活性化・動き）。
fn not_kinds() -> [(&'static str, WindowMessage); 3] {
    [
        ("space", message(WM_KEYDOWN, VK_SPACE.0 as usize, 0)),
        ("activate", message(WM_ACTIVATE, WA_ACTIVE as usize, 0)),
        ("mousemove", message(WM_MOUSEMOVE, 0, 0)),
    ]
}

fn lines<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// `drag_reentry_handled` がちょうど 1 行で、欄が期待どおり。
fn assert_one_handled_line(
    case: &str,
    events: &[log_capture_kit::CapturedEvent],
    action: &str,
    level: tracing::Level,
    entity: &str,
) {
    let handled = lines(events, "drag_reentry_handled");
    assert_eq!(handled.len(), 1, "{case}: 扱いの記録は 1 行: {handled:?}");
    let line = handled[0];
    assert_eq!(line.level, level, "{case}: level: {line:?}");
    assert_eq!(line.field_str("action"), Some(action), "{case}: {line:?}");
    assert_eq!(line.field_str("entity"), Some(entity), "{case}: {line:?}");
    assert_eq!(
        line.field_str("window"),
        Some(format!("{:?}", window()).as_str()),
        "{case}: {line:?}"
    );
    assert!(
        line.field_str("msg").is_some_and(|m| !m.is_empty()),
        "{case}: msg の欄: {line:?}"
    );
}

#[test]
fn messages_outside_the_five_kinds_are_left_alone() {
    for (case, msg) in not_kinds() {
        start();
        let before = format!("{:?}", snapshot_drag_state());
        let (handled, events) =
            log_capture_kit::capture(|| handle_message_while_world_busy(window(), &msg));
        assert!(!handled, "{case}: 扱わなかった");
        assert_eq!(
            format!("{:?}", snapshot_drag_state()),
            before,
            "{case}: 状態は変わらない"
        );
        assert!(
            lines(&events, "drag_reentry_handled").is_empty(),
            "{case}: 記録 0 行"
        );
    }
    force_idle();
}

#[test]
fn cancel_kinds_after_threshold_rest_cancelled_at_the_press_point() {
    for (case, msg) in cancel_kinds() {
        start();
        let (handled, events) =
            log_capture_kit::capture(|| handle_message_while_world_busy(window(), &msg));
        assert!(handled, "{case}: 扱った");
        match snapshot_drag_state() {
            DragStateSnapshot::JustEnded {
                entity,
                position,
                cancelled,
            } => {
                assert_eq!(entity, target(), "{case}");
                assert_eq!((position.x, position.y), (PRESS.x, PRESS.y), "{case}");
                assert!(cancelled, "{case}: 取り消しの印");
            }
            other => panic!("{case}: 休んでいない: {other:?}"),
        }
        let entity = format!("{:?}", target());
        assert_one_handled_line(case, &events, "cancelled", tracing::Level::WARN, &entity);
    }
    force_idle();
}

#[test]
fn cancel_kinds_before_threshold_also_rest() {
    for (case, msg) in cancel_kinds() {
        prepare();
        assert!(handle_message_while_world_busy(window(), &msg), "{case}");
        assert!(
            matches!(
                snapshot_drag_state(),
                DragStateSnapshot::JustEnded {
                    cancelled: true,
                    ..
                }
            ),
            "{case}: 準備中も休む"
        );
    }
    force_idle();
}

#[test]
fn release_on_an_empty_window_ends_at_the_last_point_and_logs_pos_unreadable() {
    start();
    let msg = message(WM_LBUTTONUP, 0, 0);
    let (handled, events) =
        log_capture_kit::capture(|| handle_message_while_world_busy(window(), &msg));
    assert!(handled);
    match snapshot_drag_state() {
        DragStateSnapshot::JustEnded {
            position,
            cancelled,
            ..
        } => {
            assert_eq!(
                (position.x, position.y),
                (MOVED.x, MOVED.y),
                "最後の画面の座標"
            );
            assert!(!cancelled, "離しは取り消しの印なし");
        }
        other => panic!("休んでいない: {other:?}"),
    }
    let unreadable = lines(&events, "drag_reentry_pos_unreadable");
    assert_eq!(unreadable.len(), 1, "{unreadable:?}");
    assert_eq!(unreadable[0].level, tracing::Level::WARN);
    let entity = format!("{:?}", target());
    assert_one_handled_line("release", &events, "ended", tracing::Level::WARN, &entity);
    force_idle();
}

#[test]
fn the_five_kinds_while_resting_log_none_at_debug_with_an_empty_entity() {
    let release = ("release", message(WM_LBUTTONUP, 0, 0));
    for (case, msg) in cancel_kinds().into_iter().chain([release]) {
        force_idle();
        let (handled, events) =
            log_capture_kit::capture(|| handle_message_while_world_busy(window(), &msg));
        assert!(handled, "{case}: 5 種は休んでいても扱った");
        assert!(
            matches!(snapshot_drag_state(), DragStateSnapshot::Idle),
            "{case}: 休んだまま"
        );
        assert_one_handled_line(case, &events, "none", tracing::Level::DEBUG, "");
    }
}

#[test]
fn handled_messages_raise_the_same_wake_bits_as_the_dispatch_table() {
    let _guard = crate::ecs::world::TICK_WAKE_TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let release = ("release", message(WM_LBUTTONUP, 0, 0));
    for (case, msg) in cancel_kinds().into_iter().chain([release]) {
        start();
        let _ = tick_wake::take(Instant::now());
        assert!(handle_message_while_world_busy(window(), &msg), "{case}");
        let woke = tick_wake::take(Instant::now()).bits;
        let expected = tick_wake::wake_bits_for_message(msg.msg).bits();
        assert_eq!(woke & expected, expected, "{case}: 起床の旗: {woke:#x}");
    }
    force_idle();
}
