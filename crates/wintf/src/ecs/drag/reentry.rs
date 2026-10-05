//! World を使わないドラッグの扱い（再入）
//!
//! 画面更新が World を可変で借りている間に、OS が同期で送ってくるメッセージは
//! 窓のメッセージの入口で World を借りられない。そのうちドラッグを終える 5 種
//! （ESC の押下・`WM_CANCELMODE`・非活性化・捕捉の喪失・左ボタンを離す）だけを、
//! World を使わずにドラッグの状態（`thread_local!` の `DRAG_STATE`）へ直に届ける。
//!
//! 終了の種は状態を休ませる関数が積む（`state` の `close_drag`）。ここでは積まない
//! （積むと二重になる）。World を借りない・画面更新を呼ばない・ボタンを離した記録や
//! 沈降の観測の目印や当たり判定を行わない——それらは World が要るふつうの道の仕事で、
//! 再入では飛ばす（だから扱った 1 行を記録に残す）。

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    WA_INACTIVE, WM_ACTIVATE, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KEYDOWN, WM_LBUTTONUP,
};

use crate::ecs::drag::{
    DragClose, cancel_dragging, cancel_dragging_on_capture_lost, end_dragging_on_release,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::world::tick_wake;
use crate::executor::util::WindowMessage;

/// World を借りられないときに入口が呼ぶ。5 種のどれかなら扱って `true`、違えば何もせず `false`
/// （記録も出さない）。
///
/// 5 種のときは、状態を休ませ（終了の種は休ませる関数が積む）、配送表の入口と同じ式で
/// 起床の旗を立て、扱いの記録を 1 行出す。World には触らない。
pub(crate) fn handle_message_while_world_busy(window_entity: Entity, msg: &WindowMessage) -> bool {
    let (kind, close) = match msg.msg {
        WM_KEYDOWN if msg.wparam.0 == VK_ESCAPE.0 as usize => {
            ("WM_KEYDOWN(ESC)", cancel_dragging())
        }
        WM_CANCELMODE => ("WM_CANCELMODE", cancel_dragging()),
        // wParam の下位が WA_INACTIVE のときだけ（活性化は 5 種に入らない）
        WM_ACTIVATE if (msg.wparam.0 & 0xFFFF) as u32 == WA_INACTIVE => {
            ("WM_ACTIVATE(WA_INACTIVE)", cancel_dragging())
        }
        WM_CAPTURECHANGED => ("WM_CAPTURECHANGED", cancel_dragging_on_capture_lost()),
        // 画面の座標は状態を借りる前に作る（DRAG_STATE を借りたまま OS を呼ばない）
        WM_LBUTTONUP => (
            "WM_LBUTTONUP",
            end_dragging_on_release(msg.hwnd, release_screen_pos(window_entity, msg)),
        ),
        _ => return false,
    };

    // 配送表の入口（window_proc/mod.rs）と同じ式で立てる
    tick_wake::mark(tick_wake::wake_bits_for_message(msg.msg));

    let window = format!("{window_entity:?}");
    match close {
        DragClose::Closed { entity, .. } => {
            let action = if msg.msg == WM_LBUTTONUP {
                "ended"
            } else {
                "cancelled"
            };
            // ドラッグ以外の処理（ボタンを離した記録・沈降の観測の目印など）を飛ばした、まれな出来事
            tracing::warn!(
                event = "drag_reentry_handled",
                msg = kind,
                entity = format!("{entity:?}").as_str(),
                window = window.as_str(),
                action,
                "[drag] World を借りられない間のドラッグの終わりを扱った"
            );
        }
        // 休んでいる・違う窓の離し。自分の ReleaseCapture が送る捕捉の喪失が左クリックの
        // たびにここを通るので debug!
        DragClose::OtherWindow | DragClose::NotActive => {
            tracing::debug!(
                event = "drag_reentry_handled",
                msg = kind,
                entity = "",
                window = window.as_str(),
                action = "none",
                "[drag] World を借りられない間のメッセージ（ドラッグの扱いなし）"
            );
        }
    }
    true
}

/// 離しの窓の中の座標（lParam）を画面の座標へ直す。直せなければ `warn!` を出して `None`
/// （休ませる関数が状態の持つ最後の画面の座標を使う）。
fn release_screen_pos(window_entity: Entity, msg: &WindowMessage) -> Option<PhysicalPoint> {
    let mut point = POINT {
        x: (msg.lparam.0 & 0xFFFF) as i16 as i32,
        y: ((msg.lparam.0 >> 16) & 0xFFFF) as i16 as i32,
    };
    // SAFETY: Win32 境界。ClientToScreen は point を書き換えるだけの読み取りの座標写像。
    if unsafe { ClientToScreen(msg.hwnd, &mut point) }.as_bool() {
        return Some(PhysicalPoint::new(point.x, point.y));
    }
    tracing::warn!(
        event = "drag_reentry_pos_unreadable",
        window = ?window_entity,
        hwnd = format!("0x{:X}", msg.hwnd.0 as usize),
        "[drag] 離した位置を画面の座標へ直せない。最後の画面の座標で終える"
    );
    None
}

#[cfg(test)]
#[path = "reentry_tests.rs"]
mod tests;
