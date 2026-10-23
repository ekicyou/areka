//! キーボード・システムキャンセル・アクティベーションメッセージハンドラ
//!
//! WM_KEYDOWN, WM_SYSKEYDOWN, WM_CANCELMODE, WM_ACTIVATE の処理を担当する。
//! キーの押下は窓に差された [`OnKeyDown`] の関数へ渡す（wintf はアプリの近道を知らない）。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::{debug, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};

use super::lifecycle::DESPAWNED_SKIP_TAG;
use crate::ecs::window::{KeyDown, OnKeyDown};
use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_KEYDOWN: キー押下
#[inline]
pub(super) fn WM_KEYDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

    // ESCキーでドラッグキャンセル
    if wparam.0 == VK_ESCAPE.0 as usize {
        // cancel_dragging → DragState::JustEnded へ遷移。開始済みなら終了の種も積む
        // （ここで積むと二重になる）。旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
        crate::ecs::drag::cancel_dragging();

        tracing::debug!("[WM_KEYDOWN] ESC key pressed, drag cancelled");
    }

    // 取り消しの後で利用側へ知らせる（真なら消費、それ以外は DefWindowProcW に委譲）
    deliver_key_down(
        world,
        entity,
        key_down_from(wparam, lparam, read_modifiers()),
    )
}

/// WM_SYSKEYDOWN: Alt との組み合わせや F10 の押下。WM_KEYDOWN と同じ受け渡しを通す
/// （消費しなければ DefWindowProcW に委譲し、Alt+F4 やメニューの起動は今どおり）。
#[inline]
pub(super) fn WM_SYSKEYDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    deliver_key_down(
        world,
        entity,
        key_down_from(wparam, lparam, read_modifiers()),
    )
}

/// 押下を受けた時点の修飾キー（Ctrl, Shift, Alt）。窓の手続きの入口でだけ読む。
fn read_modifiers() -> (bool, bool, bool) {
    // 上位ビットが立っていれば押されている（戻り値が負）
    let down = |vk: u16| unsafe { GetKeyState(vk as i32) } < 0;
    (down(VK_CONTROL.0), down(VK_SHIFT.0), down(VK_MENU.0))
}

/// メッセージの引数と修飾キーから [`KeyDown`] を作る（OS を読まない）。
pub(super) fn key_down_from(
    wparam: WPARAM,
    lparam: LPARAM,
    (ctrl, shift, alt): (bool, bool, bool),
) -> KeyDown {
    KeyDown {
        vk: wparam.0 as u16,
        ctrl,
        shift,
        alt,
        repeat: (lparam.0 >> 30) & 1 != 0,
    }
}

/// キーの押下を、窓に差された [`OnKeyDown`] の関数へ World 借用中に渡す。
///
/// 分かれ道は `deliver_dropped_files` と同じ 4 つ: World が借用中 → `key_down_dropped` の
/// `warn!` で渡さない／entity が破棄済み → [`DESPAWNED_SKIP_TAG`] の `debug!`／部品なし →
/// 渡さない／在れば呼び、真なら消費して `Some(LRESULT(0))`。消費しないときは `None`
/// （DefWindowProcW に委譲）。
pub(super) fn deliver_key_down(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    key: KeyDown,
) -> HandlerResult {
    let Ok(mut w) = world.try_borrow_mut() else {
        warn!(
            event = "key_down_dropped",
            entity = ?entity,
            vk = key.vk,
            reason = "world_busy",
            "[WM_KEYDOWN] World が借用中でキーの受け手を呼べない → この押下は渡さない"
        );
        return None;
    };
    if w.world().get_entity(entity).is_err() {
        debug!(
            entity = ?entity,
            vk = key.vk,
            "{DESPAWNED_SKIP_TAG} WM_KEYDOWN: 対象 entity は既に破棄済み（despawn）→ 押下を正常系として打ち切り"
        );
        return None;
    }
    let cb = w.world().get::<OnKeyDown>(entity).copied()?;
    (cb.0)(w.world_mut(), entity, key).then_some(LRESULT(0))
}

/// WM_CANCELMODE: システムキャンセル
#[inline]
pub(super) fn WM_CANCELMODE(
    _world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // ドラッグキャンセル
    // cancel_dragging → DragState::JustEnded へ遷移。開始済みなら終了の種も積む
    // （ここで積むと二重になる）。旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
    crate::ecs::drag::cancel_dragging();

    tracing::debug!("[WM_CANCELMODE] System cancel, drag cancelled");

    None // DefWindowProcWに委譲
}

/// WM_ACTIVATE: ウィンドウ非アクティブ化時のドラッグキャンセルと沈降観測の目印付け
///
/// Alt+Tabなどでウィンドウが非アクティブになった場合、ドラッグ中であればキャンセルする。
/// WM_CANCELMODEはモーダルダイアログやメニュー表示時にのみ送られ、
/// Alt+Tabでは送られないため、WM_ACTIVATEで補完する必要がある。
///
/// あわせて、当該窓がゴースト窓ペアの当事者なら**沈降観測の目印**を付ける（要件 4.4／7.5）。
/// 付けるのは目印だけで、重なりの実測も記録もここでは行わない——理由は
/// [`mark_pair_sink_observation`](crate::ecs::window::mark_pair_sink_observation) の
/// 呼出箇所のコメントを参照。**窓の挙動は一切変えない**（読み取りのみ）。
pub(super) fn WM_ACTIVATE(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    let activation_state = (wparam.0 & 0xFFFF) as u32;

    // 非アクティブ化時のみ処理 (WA_INACTIVE = 0)
    if activation_state != 0 {
        return None;
    }

    // 押している状態ならキャンセル（押していなければ何もしない）。
    // JustStarted／Dragging は cancel_dragging が Ended(cancelled) を積んで閉じ、
    // Preparing は積まずに休ませる（CaptureGuard が Drop され ReleaseCapture が自動呼び出し）。
    match crate::ecs::drag::cancel_dragging() {
        crate::ecs::drag::DragClose::Closed {
            entity,
            notified: true,
        } => {
            tracing::info!(
                entity = ?entity,
                "[WM_ACTIVATE] Window deactivated during drag, cancelling"
            );
        }
        crate::ecs::drag::DragClose::Closed {
            notified: false, ..
        } => {
            tracing::debug!("[WM_ACTIVATE] Window deactivated during drag prepare, resetting");
        }
        crate::ecs::drag::DragClose::OtherWindow | crate::ecs::drag::DragClose::NotActive => {}
    }

    // 沈降観測の目印を付ける（既存処理の後・読み取り専用）。
    //
    // ここで重なりを測ってはならない。WM_ACTIVATE(WA_INACTIVE) は活性化トランザクションの
    // **途中**に届き、自分が前面から外れたことは確定していても、**新しい前面窓の浮上は
    // 終わっていないことがある**。その瞬間に走査すると、実装に何の欠陥が無くても
    // 「前面窓より背面に居る」を満たさず、偽の失敗が記録として残る。
    // よってこの枝が行うのは目印を付けることだけであり、実測と記録は次の巡に維持系
    // （apply_zorder_pair_maintenance）が行う（design.md「WM_ACTIVATE 沈降観測」）。
    //
    // 当該窓がペアの当事者でなければ何も起きない。窓の状態には一切書き込まない。
    if let Ok(mut world_borrow) = world.try_borrow_mut() {
        crate::ecs::window::mark_pair_sink_observation(world_borrow.world_mut(), entity);
    }

    None // DefWindowProcWに委譲
}

/// WM_CAPTURECHANGED: 外部要因でマウスキャプチャを失った場合の処理
///
/// 別ウィンドウが `SetCapture` を呼び出すか、OS がキャプチャを解放した場合に送信される。
/// ドラッグ中であれば安全にキャンセルする。
/// CaptureGuard に `mark_released()` を呼び、Drop 時の `ReleaseCapture` をスキップする
/// （キャプチャは OS が既に解放済みのため）。
///
/// 冪等性: DragState が既に Idle / JustEnded の場合は何もしない。
pub(super) fn WM_CAPTURECHANGED(
    _world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // cancel_dragging_on_capture_lost が CaptureGuard を mark_released してからキャンセルする
    // （JustEnded 遷移時の CaptureGuard Drop で ReleaseCapture が呼ばれない）。
    // 開始済みなら終了の種も積む（ここで積むと二重になる）。
    if let crate::ecs::drag::DragClose::Closed { .. } =
        crate::ecs::drag::cancel_dragging_on_capture_lost()
    {
        tracing::debug!("[WM_CAPTURECHANGED] Capture lost externally, drag cancelled");
    }

    None // DefWindowProcWに委譲
}

#[cfg(test)]
#[path = "keyboard_tests.rs"]
mod keyboard_tests;
