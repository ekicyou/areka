//! 窓の手続きで形の問い合わせに答え、カーソルの来ない場面で出し直す。
//!
//! 判断の分岐（[`trigger_of`]・[`plan_set_cursor`]・[`push_allowed`]・[`dpi_or_default`]）は
//! Win32 から切り離した純関数に置き、`on_set_cursor`・`push_cursor` は Win32 の値を読んで
//! それらを呼び、結果どおりに `SetCursor`・`TrackMouseEvent` を呼ぶだけの薄い層にする。

use super::{CursorAsk, CursorQuery, CursorReply, CursorTrigger};
use crate::ecs::layout::hit_test::{PhysicalPoint, hit_test_in_window};
use crate::ecs::window::WindowHandle;
use crate::ecs::world::EcsWorld;
use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::{debug, trace};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Controls::HOVER_DEFAULT;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, TME_HOVER, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VIRTUAL_KEY,
    VK_LBUTTON, VK_MBUTTON, VK_RBUTTON, VK_XBUTTON1, VK_XBUTTON2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, HCURSOR, HTCLIENT, SetCursor, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_XBUTTONDBLCLK, WM_XBUTTONDOWN, WM_XBUTTONUP, WindowFromPoint,
};

/// `WM_SETCURSOR` で何をするか（[`plan_set_cursor`] の結果）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetCursorPlan {
    /// `SetCursor` する形（`None` は触らない）。
    pub set: Option<HCURSOR>,
    /// 窓の手続きの戻り値（`None` は既定の処理へ）。
    pub result: Option<LRESULT>,
    /// 止まった時計を掛け直すか。
    pub rearm_hover: bool,
}

/// `WM_SETCURSOR` の lParam の上位語（引き金のメッセージ）を引き金へ読む。
/// 移動・メニューの 0・知らない値は移動。
pub fn trigger_of(msg: u32) -> CursorTrigger {
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONDBLCLK | WM_XBUTTONDOWN
        | WM_XBUTTONDBLCLK => CursorTrigger::Press { right: false },
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => CursorTrigger::Press { right: true },
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => CursorTrigger::Release,
        _ => CursorTrigger::Move,
    }
}

/// 当たりの結果が `HTCLIENT` のときだけ答えに従う。`Keep` は出さずに 1 を返して既定の処理
/// （窓のクラスの矢印）を止める。止まった時計の掛け直しは「出す」かつ移動のときだけ。
pub fn plan_set_cursor(
    hit_test: u32,
    reply: Option<CursorReply>,
    trigger: CursorTrigger,
) -> SetCursorPlan {
    let pass = SetCursorPlan {
        set: None,
        result: None,
        rearm_hover: false,
    };
    if hit_test != HTCLIENT {
        return pass;
    }
    match reply {
        Some(CursorReply::Show(h)) => SetCursorPlan {
            set: Some(h),
            result: Some(LRESULT(1)),
            rearm_hover: trigger == CursorTrigger::Move,
        },
        Some(CursorReply::Keep) => SetCursorPlan {
            result: Some(LRESULT(1)),
            ..pass
        },
        None => pass,
    }
}

/// 世界を借りられないときの答え: 窓の最後に出した形が在ればそれを出して 1、無ければ既定の処理へ。
pub fn plan_unborrowed(hit_test: u32, last: Option<HCURSOR>) -> SetCursorPlan {
    SetCursorPlan {
        rearm_hover: false,
        ..plan_set_cursor(
            hit_test,
            last.map(CursorReply::Show),
            CursorTrigger::Refresh,
        )
    }
}

/// 出し直してよいか（カーソルの下の窓がその窓のときだけ）。
pub fn push_allowed(under_cursor: HWND, window: HWND) -> bool {
    under_cursor == window
}

/// 窓の DPI（0 は読めなかった＝96 として寸法を選ぶ）。
pub fn dpi_or_default(dpi: u32) -> u32 {
    if dpi == 0 { 96 } else { dpi }
}

thread_local! {
    /// 窓ごとの最後に出した形（世界を借りられないときの答え）。
    static LAST_SHOWN: RefCell<HashMap<isize, HCURSOR>> = RefCell::new(HashMap::new());
}

fn remember_shown(hwnd: HWND, cursor: HCURSOR) {
    LAST_SHOWN.with(|m| m.borrow_mut().insert(hwnd.0 as isize, cursor));
}

/// 窓の最後に出した形。
pub fn last_shown(hwnd: HWND) -> Option<HCURSOR> {
    LAST_SHOWN.with(|m| m.borrow().get(&(hwnd.0 as isize)).copied())
}

/// 最後に出した形の控えを全部消す（シェル・ゴーストの入れ替えで前のカーソルを捨てる前に呼ぶ）。
pub fn clear_last_shown() {
    LAST_SHOWN.with(|m| m.borrow_mut().clear());
}

fn show(hwnd: HWND, cursor: HCURSOR) {
    // SAFETY: SetCursor は共有の形も持ち主の生きている形も受ける。所有権に触らない。
    unsafe { SetCursor(Some(cursor)) };
    remember_shown(hwnd, cursor);
}

/// 今のカーソルの位置（スクリーン座標）。
fn cursor_pos() -> Option<POINT> {
    let mut pt = POINT::default();
    // SAFETY: 有効な POINT への書き込み。
    match unsafe { GetCursorPos(&mut pt) } {
        Ok(()) => Some(pt),
        Err(e) => {
            trace!(error = %e, "GetCursorPos failed");
            None
        }
    }
}

fn any_button_down() -> bool {
    const BUTTONS: [VIRTUAL_KEY; 5] =
        [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2];
    // SAFETY: 引数は仮想キーの値だけ。最上位ビットが「今押されている」。
    BUTTONS
        .iter()
        .any(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } < 0)
}

/// Win32 から問い合わせの材料を読む。位置が読めなければ `None`（問い合わせない）。
fn read_ask(
    world: &World,
    window: Entity,
    hwnd: HWND,
    screen: POINT,
    trigger: CursorTrigger,
) -> Option<CursorAsk> {
    let mut pt = screen;
    // SAFETY: HWND と有効な POINT。
    if !unsafe { ScreenToClient(hwnd, &mut pt) }.as_bool() {
        trace!(?hwnd, "ScreenToClient failed");
        return None;
    }
    let owner = hit_test_in_window(world, window, PhysicalPoint::new(pt.x as f32, pt.y as f32));
    // SAFETY: HWND を渡すだけ（無効な窓は 0 が返る）。
    let raw_dpi = unsafe { GetDpiForWindow(hwnd) };
    if raw_dpi == 0 {
        debug!(?hwnd, "GetDpiForWindow returned 0, using 96");
    }
    Some(CursorAsk {
        trigger,
        client: (pt.x, pt.y),
        see_through: owner.is_none_or(|e| e == window),
        buttons_down: any_button_down(),
        dpi: dpi_or_default(raw_dpi),
    })
}

/// `WM_SETCURSOR` に答える（十字の診断の後ろで呼ぶ）。`None` は既定の処理へ。
pub fn on_set_cursor(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    lparam: LPARAM,
) -> Option<LRESULT> {
    let hit_test = (lparam.0 & 0xFFFF) as u32;
    let trigger = trigger_of(((lparam.0 >> 16) & 0xFFFF) as u32);
    if hit_test != HTCLIENT {
        return None;
    }
    let plan = match world.try_borrow() {
        Ok(w) => {
            let w = w.world();
            let reply = w.get_resource::<CursorQuery>().and_then(|q| {
                let ask = read_ask(w, entity, hwnd, cursor_pos()?, trigger)?;
                (q.0)(w, entity, &ask)
            });
            plan_set_cursor(hit_test, reply, trigger)
        }
        Err(_) => {
            trace!(
                ?hwnd,
                "WM_SETCURSOR: world busy, answering with last shown cursor"
            );
            plan_unborrowed(hit_test, last_shown(hwnd))
        }
    };
    if let Some(cursor) = plan.set {
        show(hwnd, cursor);
    }
    if plan.rearm_hover {
        let mut tme = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_HOVER | TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: HOVER_DEFAULT,
        };
        // SAFETY: 有効な TRACKMOUSEEVENT。
        if let Err(e) = unsafe { TrackMouseEvent(&mut tme) } {
            debug!(?hwnd, error = %e, "TrackMouseEvent(TME_HOVER) failed");
        }
    }
    plan.result
}

/// `WM_MOUSEHOVER`（止まった）を受けて出し直す。
pub fn on_mouse_hover(world: &Rc<RefCell<EcsWorld>>, entity: Entity) -> Option<LRESULT> {
    push_cursor_rc(world, entity, CursorTrigger::Hover);
    Some(LRESULT(0))
}

/// 世界の中から出し直す。カーソルがその窓の上に居て、答えが `Show` のときだけ `SetCursor` する。
pub fn push_cursor(world: &World, window: Entity, trigger: CursorTrigger) {
    let Some(query) = world.get_resource::<CursorQuery>() else {
        return;
    };
    let Some(hwnd) = world.get::<WindowHandle>(window).map(|h| h.hwnd) else {
        trace!(?window, "push_cursor: no WindowHandle");
        return;
    };
    let Some(screen) = cursor_pos() else {
        return;
    };
    // SAFETY: 値渡しの POINT だけ。
    let under = unsafe { WindowFromPoint(screen) };
    if !push_allowed(under, hwnd) {
        return;
    }
    let Some(ask) = read_ask(world, window, hwnd, screen, trigger) else {
        return;
    };
    if let Some(CursorReply::Show(cursor)) = (query.0)(world, window, &ask) {
        show(hwnd, cursor);
    }
}

/// 窓の手続きから出し直す（世界の借用を試す）。
pub fn push_cursor_rc(world: &Rc<RefCell<EcsWorld>>, window: Entity, trigger: CursorTrigger) {
    match world.try_borrow() {
        Ok(w) => push_cursor(w.world(), window, trigger),
        // 解放の形は次の移動まで残る恐れがあるので trace より上で残す。
        Err(_) if trigger == CursorTrigger::Release => {
            debug!(
                ?window,
                "push_cursor_rc: world busy, release cursor not refreshed"
            )
        }
        Err(_) => trace!(?window, ?trigger, "push_cursor_rc: world busy"),
    }
}

#[cfg(test)]
#[path = "answer_tests.rs"]
mod answer_tests;
