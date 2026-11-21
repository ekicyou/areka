//! ウィンドウ手続きブリッジ。
//!
//! ライブラリ（`wintf-winmsg-executor`）の `Window<S>` が受け取る wndproc クロージャの状態
//! `S = WndState` を定め、クロージャを ECS 層の配送純関数 `crate::ecs::dispatch_window_message`
//! （配送表の単一の真実の源）へ橋渡しする。`None` を返すとライブラリ既定手続き
//! （`DefWindowProcW`）へ委ねられる。
//!
//! World/Entity は生成時に確定した `WndState` をクロージャが直接 capture する
//! （`GWLP_USERDATA` もグローバル状態も使わない）。
//!
//! # 安全スキップ
//! `world.upgrade()`→`None`（破棄中）と World の `try_borrow` 失敗（再入）はいずれも `None` で飛ばす。
//!
//! 例外として `try_borrow` 失敗（再入）のときも、ドラッグを終える 5 種（ESC の押下〔`WM_KEYDOWN`〕・
//! `WM_CANCELMODE`・非活性化の `WM_ACTIVATE`・`WM_CAPTURECHANGED`・`WM_LBUTTONUP`）だけは捨てずに、
//! World を使わないドラッグの扱い（`crate::ecs::drag::handle_message_while_world_busy`）へ渡してから
//! `None` を返す。画面更新の途中の非活性化などでドラッグの状態が休まず残るのを防ぐため。
//! World の破棄中は例外なく飛ばす。
//!
//! `try_borrow` 失敗（再入）のときに届いた `WM_CLOSE` は、閉じる要求が捨てられたことを
//! `warn!(wm_close_world_busy)` に残す（ライブラリは `WM_CLOSE` を既定の手続きへ渡さず
//! `LRESULT(0)` を返すので、窓は壊れずに要求だけが消える）。

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Weak;

use crate::executor::util::WindowMessage;
use bevy_ecs::prelude::Entity;
use tracing::warn;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::UI::WindowsAndMessaging::{WM_CLOSE, WM_ENDSESSION};

use crate::ecs::dispatch_window_message;
use crate::ecs::drag::handle_message_while_world_busy;
use crate::ecs::world::EcsWorld;

/// wndproc クロージャが生成時に capture する共有状態。
///
/// `world`／`entity` はウィンドウ生成時に確定し、以後不変。`Weak` で掴むのは registry↔World の
/// 自己循環リークを避けるため。
pub(crate) struct WndState {
    /// 共有 World への弱参照（strong 所有者は `WinApp`）。
    pub world: Weak<RefCell<EcsWorld>>,
    /// このウィンドウに対応する ECS Entity。
    pub entity: Entity,
}

/// `Window::new_ex` 互換の wndproc クロージャを構築する。
///
/// ライブラリの `new_ex`（wndproc 再入可）へ渡す。再入を阻止する `new_checked_ex` を使わない
/// 理由は `window_factory` の `create_window` にある。`try_borrow` 失敗時は `WM_ENDSESSION`
/// （wParam 真）を `warn!(os_session_end_world_busy)` に残してから飛ばす。
pub(crate) fn make_wndproc() -> impl Fn(Pin<&WndState>, WindowMessage) -> Option<LRESULT> {
    move |state: Pin<&WndState>, msg: WindowMessage| -> Option<LRESULT> {
        let entity = state.entity;

        let world = state.world.upgrade()?;

        // 借用ガードは dispatch に渡さない（dispatch は `Rc` を受け取り内部で借用する）。
        // ここでは借用可能かだけを試して即座に解放する。
        if world.try_borrow().is_err() {
            // OS のセッションの終了（wParam 真）だけは、受け手を呼べないことを黙って流さない。
            if msg.msg == WM_ENDSESSION && msg.wparam.0 != 0 {
                warn!(
                    event = "os_session_end_world_busy",
                    entity = ?entity,
                    "[WM_ENDSESSION] World が借用中でセッションの終了の受け手を呼べない → \
                     後始末をせずに終わるので起動中の印が残り、次の起動は前回落ちた扱い（Ref6/7 付き）になる"
                );
            }
            // 閉じる要求はここで捨てられる（ライブラリは WM_CLOSE を既定の手続きへ渡さない）ので黙って流さない。
            if msg.msg == WM_CLOSE {
                warn!(
                    event = "wm_close_world_busy",
                    entity = ?entity,
                    "[WM_CLOSE] World が借用中で閉じる要求を扱えない → 要求を捨てた（閉じ直せば閉じる）"
                );
            }
            // 戻り値（扱ったか）は使わず、どの場合も既定の手続きへ委ねる。
            handle_message_while_world_busy(entity, &msg);
            return None;
        }

        dispatch_window_message(&world, entity, &msg)
    }
}

#[cfg(test)]
#[path = "wndproc_bridge_drag_test_support.rs"]
mod wndproc_bridge_drag_test_support;

#[cfg(test)]
#[path = "wndproc_bridge_drag_cancel_tests.rs"]
mod wndproc_bridge_drag_cancel_tests;

#[cfg(test)]
#[path = "wndproc_bridge_drag_release_tests.rs"]
mod wndproc_bridge_drag_release_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{WM_ERASEBKGND, WM_USER};

    fn make_state(world: &Rc<RefCell<EcsWorld>>) -> WndState {
        let entity = world.borrow_mut().world_mut().spawn(()).id();
        WndState {
            world: Rc::downgrade(world),
            entity,
        }
    }

    /// hwnd 等は配送に無関係。
    fn msg(message: u32) -> WindowMessage {
        WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: message,
            wparam: WPARAM(0),
            lparam: LPARAM(0),
        }
    }

    /// World が drop 済み（`upgrade()`→`None`）のときクロージャは `None` を返し panic しない。
    #[test]
    fn closure_safe_skips_when_world_dropped() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();

        drop(world);

        let boxed = Box::pin(state);
        let ret = wndproc(boxed.as_ref(), msg(WM_ERASEBKGND));
        assert!(
            ret.is_none(),
            "World drop 後は upgrade→None で安全スキップ（None）すべき"
        );
    }

    /// World が借用中（`borrow_mut` 保持）のときクロージャは `None` を返し panic しない。
    #[test]
    fn closure_safe_skips_when_world_borrowed() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();

        let _held = world.borrow_mut();

        let boxed = Box::pin(state);
        let ret = wndproc(boxed.as_ref(), msg(WM_ERASEBKGND));
        assert!(
            ret.is_none(),
            "World 借用中は try_borrow 失敗で安全スキップ（None）すべき"
        );
    }

    /// World の借用中に `WM_ENDSESSION`（wParam 真）が届いたら、受け手を呼べないことを
    /// 帰結付きの `warn!` 1 件で残し、既定の手続きへ委ねる（`None`）。wParam 偽や他のメッセージでは
    /// 記録しない。
    #[test]
    fn closure_warns_session_end_only_while_world_borrowed() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();
        let boxed = Box::pin(state);
        let busy_warns = |m: WindowMessage| {
            let (ret, events) = log_capture_kit::capture(|| wndproc(boxed.as_ref(), m));
            let n = events
                .iter()
                .filter(|e| {
                    e.field_str("event") == Some("os_session_end_world_busy")
                        && e.level == tracing::Level::WARN
                })
                .count();
            (ret, n)
        };

        let _held = world.borrow_mut();
        let session_end = WindowMessage {
            wparam: WPARAM(1),
            ..msg(WM_ENDSESSION)
        };
        assert_eq!(busy_warns(session_end), (None, 1));
        assert_eq!(
            busy_warns(msg(WM_ENDSESSION)),
            (None, 0),
            "wParam 偽で警告した"
        );
        assert_eq!(
            busy_warns(msg(WM_ERASEBKGND)),
            (None, 0),
            "他のメッセージで警告した"
        );
    }

    /// World の借用中に届いた `WM_CLOSE` は捨てられるので `warn!(wm_close_world_busy)` を 1 件残す。
    /// 他のメッセージでは記録しない。
    #[test]
    fn closure_warns_close_while_world_borrowed() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();
        let boxed = Box::pin(state);
        let busy = |m: WindowMessage| {
            let (ret, events) = log_capture_kit::capture(|| wndproc(boxed.as_ref(), m));
            let n = events
                .iter()
                .filter(|e| {
                    e.field_str("event") == Some("wm_close_world_busy")
                        && e.level == tracing::Level::WARN
                })
                .count();
            (ret, n)
        };

        let _held = world.borrow_mut();
        assert_eq!(busy(msg(WM_CLOSE)), (None, 1));
        assert_eq!(
            busy(msg(WM_ERASEBKGND)),
            (None, 0),
            "他のメッセージで警告した"
        );
    }

    /// 生存・未借用 World で代表メッセージは `Some(LRESULT)`、非対応メッセージは `None`。
    #[test]
    fn closure_routes_representative_message() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();

        let boxed = Box::pin(state);

        let handled = wndproc(boxed.as_ref(), msg(WM_ERASEBKGND));
        assert_eq!(
            handled,
            Some(LRESULT(1)),
            "代表メッセージ WM_ERASEBKGND は Some(LRESULT(1)) を返すべき"
        );

        let unhandled = wndproc(boxed.as_ref(), msg(WM_USER));
        assert!(
            unhandled.is_none(),
            "非対応メッセージは None（既定手続き委譲）であるべき"
        );
    }
}
