//! キーボード・システムキャンセル・アクティベーションメッセージハンドラ
//!
//! WM_KEYDOWN, WM_CANCELMODE, WM_ACTIVATE の処理を担当する。

#![allow(non_snake_case)]

use windows::Win32::Foundation::*;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_KEYDOWN: キー押下
#[inline]
pub(super) fn WM_KEYDOWN(
    _hwnd: HWND,
    _message: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

    // ESCキーでドラッグキャンセル
    if wparam.0 == VK_ESCAPE.0 as usize {
        // thread_local DragStateをクローンして取得
        let state_snapshot = crate::ecs::drag::read_drag_state(|state| state.clone());

        if let crate::ecs::drag::DragState::Dragging {
            entity, start_pos, ..
        }
        | crate::ecs::drag::DragState::Preparing {
            entity, start_pos, ..
        }
        | crate::ecs::drag::DragState::JustStarted {
            entity, start_pos, ..
        } = state_snapshot
        {
            // DragAccumulatorResourceにEnded遷移を記録
            if let Some(world) = super::try_get_ecs_world() {
                if let Ok(world_borrow) = world.try_borrow() {
                    if let Some(accumulator) = world_borrow
                        .world()
                        .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                    ) {
                        accumulator.set_transition(crate::ecs::drag::DragTransition::Ended {
                            entity,
                            end_pos: start_pos,
                            cancelled: true,
                        });
                    }
                }
            }
        }

        crate::ecs::drag::cancel_dragging();
        // TODO(wintf-screen-drag-stability): ReleaseCapture をここで呼び出す
        // API: windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture
        // let _ = unsafe { ReleaseCapture() };

        tracing::debug!("[WM_KEYDOWN] ESC key pressed, drag cancelled");
    }

    None // DefWindowProcWに委譲
}

/// WM_CANCELMODE: システムキャンセル
#[inline]
pub(super) fn WM_CANCELMODE(
    _hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // thread_local DragStateをクローンして取得
    let state_snapshot = crate::ecs::drag::read_drag_state(|state| state.clone());

    if let crate::ecs::drag::DragState::Dragging {
        entity, start_pos, ..
    }
    | crate::ecs::drag::DragState::Preparing {
        entity, start_pos, ..
    }
    | crate::ecs::drag::DragState::JustStarted {
        entity, start_pos, ..
    } = state_snapshot
    {
        // DragAccumulatorResourceにEnded遷移を記録
        if let Some(world) = super::try_get_ecs_world() {
            if let Ok(world_borrow) = world.try_borrow() {
                if let Some(accumulator) = world_borrow
                    .world()
                    .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                ) {
                    accumulator.set_transition(crate::ecs::drag::DragTransition::Ended {
                        entity,
                        end_pos: start_pos,
                        cancelled: true,
                    });
                }
            }
        }
    }

    // ドラッグキャンセル
    crate::ecs::drag::cancel_dragging();

    tracing::debug!("[WM_CANCELMODE] System cancel, drag cancelled");

    None // DefWindowProcWに委譲（ReleaseCapture自動実行）
}

/// WM_ACTIVATE: ウィンドウ非アクティブ化時のドラッグキャンセル
///
/// Alt+Tabなどでウィンドウが非アクティブになった場合、ドラッグ中であればキャンセルする。
/// WM_CANCELMODEはモーダルダイアログやメニュー表示時にのみ送られ、
/// Alt+Tabでは送られないため、WM_ACTIVATEで補完する必要がある。
pub(super) fn WM_ACTIVATE(
    _hwnd: HWND,
    _message: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    let activation_state = (wparam.0 & 0xFFFF) as u32;

    // 非アクティブ化時のみ処理 (WA_INACTIVE = 0)
    if activation_state != 0 {
        return None;
    }

    // ドラッグ中なら状態を確認してキャンセル
    let state_snapshot = crate::ecs::drag::read_drag_state(|state| state.clone());
    match state_snapshot {
        crate::ecs::drag::DragState::Dragging {
            entity, start_pos, ..
        } => {
            tracing::info!(
                entity = ?entity,
                "[WM_ACTIVATE] Window deactivated during drag, cancelling"
            );

            // DragAccumulatorResourceにEnded(cancelled)遷移を記録
            if let Some(world) = super::try_get_ecs_world() {
                if let Ok(world_borrow) = world.try_borrow() {
                    if let Some(accumulator) = world_borrow
                        .world()
                        .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                    ) {
                        accumulator.set_transition(crate::ecs::drag::DragTransition::Ended {
                            entity,
                            end_pos: start_pos,
                            cancelled: true,
                        });
                    }
                }
            }

            crate::ecs::drag::cancel_dragging();
        }
        crate::ecs::drag::DragState::Preparing { .. } => {
            tracing::debug!("[WM_ACTIVATE] Window deactivated during drag prepare, resetting");
            crate::ecs::drag::reset_to_idle();
        }
        _ => {}
    }

    None // DefWindowProcWに委譲
}
