//! キーボード・システムキャンセル・アクティベーションメッセージハンドラ
//!
//! WM_KEYDOWN, WM_CANCELMODE, WM_ACTIVATE の処理を担当する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_KEYDOWN: キー押下
#[inline]
pub(super) fn WM_KEYDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

    // ESCキーでドラッグキャンセル
    if wparam.0 == VK_ESCAPE.0 as usize {
        let state_snapshot = crate::ecs::drag::snapshot_drag_state();

        if let crate::ecs::drag::DragStateSnapshot::Dragging {
            entity, start_pos, ..
        }
        | crate::ecs::drag::DragStateSnapshot::Preparing {
            entity, start_pos, ..
        }
        | crate::ecs::drag::DragStateSnapshot::JustStarted {
            entity, start_pos, ..
        } = state_snapshot
        {
            // DragAccumulatorResourceにEnded遷移を記録
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

        // cancel_dragging → DragState::JustEnded へ遷移
        // 旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
        crate::ecs::drag::cancel_dragging();

        tracing::debug!("[WM_KEYDOWN] ESC key pressed, drag cancelled");
    }

    None // DefWindowProcWに委譲
}

/// WM_CANCELMODE: システムキャンセル
#[inline]
pub(super) fn WM_CANCELMODE(
    world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    let state_snapshot = crate::ecs::drag::snapshot_drag_state();

    if let crate::ecs::drag::DragStateSnapshot::Dragging {
        entity, start_pos, ..
    }
    | crate::ecs::drag::DragStateSnapshot::Preparing {
        entity, start_pos, ..
    }
    | crate::ecs::drag::DragStateSnapshot::JustStarted {
        entity, start_pos, ..
    } = state_snapshot
    {
        // DragAccumulatorResourceにEnded遷移を記録
        if let Ok(world_borrow) = world.try_borrow() {
            if let Some(accumulator) = world_borrow
                .world()
                .get_resource::<crate::ecs::drag::DragAccumulatorResource>()
            {
                accumulator.set_transition(crate::ecs::drag::DragTransition::Ended {
                    entity,
                    end_pos: start_pos,
                    cancelled: true,
                });
            }
        }
    }

    // ドラッグキャンセル
    // cancel_dragging → DragState::JustEnded へ遷移
    // 旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
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

    // ドラッグ中なら状態を確認してキャンセル
    let state_snapshot = crate::ecs::drag::snapshot_drag_state();
    match state_snapshot {
        crate::ecs::drag::DragStateSnapshot::Dragging {
            entity, start_pos, ..
        } => {
            tracing::info!(
                entity = ?entity,
                "[WM_ACTIVATE] Window deactivated during drag, cancelling"
            );

            // DragAccumulatorResourceにEnded(cancelled)遷移を記録
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

            crate::ecs::drag::cancel_dragging();
        }
        crate::ecs::drag::DragStateSnapshot::Preparing { .. } => {
            tracing::debug!("[WM_ACTIVATE] Window deactivated during drag prepare, resetting");
            // Preparing 状態をキャンセル（CaptureGuard が Drop され ReleaseCapture が自動呼び出し）
            crate::ecs::drag::cancel_dragging();
        }
        _ => {}
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
    world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // CaptureGuard を mark_released してからドラッグキャンセル
    // mark_released を先に呼ぶことで、cancel_dragging → JustEnded 遷移時の
    // CaptureGuard Drop で ReleaseCapture が呼ばれないようにする
    let was_dragging = crate::ecs::drag::update_drag_state(|state| {
        match state {
            crate::ecs::drag::DragState::Preparing { capture_guard, .. }
            | crate::ecs::drag::DragState::JustStarted { capture_guard, .. }
            | crate::ecs::drag::DragState::Dragging { capture_guard, .. } => {
                capture_guard.mark_released();
                true
            }
            _ => false, // Idle / JustEnded: 何もしない
        }
    });

    if !was_dragging {
        return None;
    }

    // DragAccumulatorResource に Ended(cancelled) 遷移を記録
    let state_snapshot = crate::ecs::drag::snapshot_drag_state();
    if let crate::ecs::drag::DragStateSnapshot::Dragging {
        entity, start_pos, ..
    }
    | crate::ecs::drag::DragStateSnapshot::Preparing {
        entity, start_pos, ..
    }
    | crate::ecs::drag::DragStateSnapshot::JustStarted {
        entity, start_pos, ..
    } = state_snapshot
    {
        if let Ok(world_borrow) = world.try_borrow() {
            if let Some(accumulator) = world_borrow
                .world()
                .get_resource::<crate::ecs::drag::DragAccumulatorResource>()
            {
                accumulator.set_transition(crate::ecs::drag::DragTransition::Ended {
                    entity,
                    end_pos: start_pos,
                    cancelled: true,
                });
            }
        }
    }

    crate::ecs::drag::cancel_dragging();

    tracing::debug!("[WM_CAPTURECHANGED] Capture lost externally, drag cancelled");

    None // DefWindowProcWに委譲
}
