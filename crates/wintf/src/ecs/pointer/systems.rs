//! ポインターシステム
//!
//! ECSスケジュールで実行されるポインター関連システム関数。
//! バッファ内容の反映、一時状態のクリア、デバッグ監視を提供する。

use bevy_ecs::prelude::*;
use std::time::Instant;

use super::buffers::{
    BUTTON_BUFFERS, DOUBLE_CLICK_BUFFERS, MODIFIER_STATE, POINTER_BUFFERS, WHEEL_BUFFERS,
};
use super::types::{
    CursorVelocity, DoubleClick, PhysicalPoint, PointerButton, PointerLeave, PointerState,
    WheelDelta,
};

// ============================================================================
// システム
// ============================================================================

/// ポインターバッファ処理システム
///
/// Inputスケジュールで実行され、バッファ内容をPointerStateコンポーネントに反映する。
pub fn process_pointer_buffers(mut query: Query<(Entity, &mut PointerState)>) {
    tracing::trace!("[process_pointer_buffers] Called");

    // ButtonBufferの内容をPointerStateに反映（エンティティIDで照合）
    // Note: BUTTON_BUFFERSのリセットはdispatch_pointer_eventsで行われる
    BUTTON_BUFFERS.with(|buffers| {
        let buffers = buffers.borrow();

        for (entity, mut pointer) in query.iter_mut() {
            // 各ボタンの処理（DOWN優先ルール）
            for button in [
                PointerButton::Left,
                PointerButton::Right,
                PointerButton::Middle,
                PointerButton::XButton1,
                PointerButton::XButton2,
            ] {
                if let Some(buf) = buffers.get(&(entity, button)) {
                    let is_down = if buf.down_received {
                        true
                    } else if buf.up_received {
                        false
                    } else {
                        // イベントなし - 現在の状態を維持
                        match button {
                            PointerButton::Left => pointer.left_down,
                            PointerButton::Right => pointer.right_down,
                            PointerButton::Middle => pointer.middle_down,
                            PointerButton::XButton1 => pointer.xbutton1_down,
                            PointerButton::XButton2 => pointer.xbutton2_down,
                        }
                    };

                    match button {
                        PointerButton::Left => pointer.left_down = is_down,
                        PointerButton::Right => pointer.right_down = is_down,
                        PointerButton::Middle => pointer.middle_down = is_down,
                        PointerButton::XButton1 => pointer.xbutton1_down = is_down,
                        PointerButton::XButton2 => pointer.xbutton2_down = is_down,
                    }

                    // ログ出力（ボタン状態が変化した場合）
                    if buf.down_received || buf.up_received {
                        tracing::trace!(
                            entity = ?entity,
                            button = ?button,
                            is_down,
                            "[process_pointer_buffers] Button state updated"
                        );
                    }
                }
            }
        }
    });

    for (entity, mut pointer) in query.iter_mut() {
        tracing::trace!(
            entity = ?entity,
            thread_id = ?std::thread::current().id(),
            "[process_pointer_buffers] Checking POINTER_BUFFERS"
        );

        // PointerBuffer から位置と速度を取得
        POINTER_BUFFERS.with(|buffers| {
            let mut buffers = buffers.borrow_mut();
            if let Some(buffer) = buffers.get_mut(&entity) {
                tracing::trace!(
                    entity = ?entity,
                    "[process_pointer_buffers] Buffer found"
                );

                // 速度計算
                let (vx, vy) = buffer.calculate_velocity();
                pointer.velocity = CursorVelocity::new(vx, vy);

                // 最新位置取得
                if let Some(sample) = buffer.latest() {
                    let old_x = pointer.client_point.x;
                    let old_y = pointer.client_point.y;
                    pointer.client_point = PhysicalPoint::new(sample.x as i32, sample.y as i32);
                    // Note: local_point は hit_test 結果から設定（Phase 1ではclient_pointと同じ）
                    pointer.local_point = pointer.client_point;

                    tracing::trace!(
                        entity = ?entity,
                        old_x, old_y,
                        new_x = pointer.client_point.x,
                        new_y = pointer.client_point.y,
                        "[process_pointer_buffers] Position updated"
                    );
                }

                // バッファクリア
                buffer.clear();
            } else {
                tracing::trace!(
                    entity = ?entity,
                    "[process_pointer_buffers] No buffer found"
                );
            }
        });

        // WheelBuffer からホイール情報を取得
        WHEEL_BUFFERS.with(|buffers| {
            let mut buffers = buffers.borrow_mut();
            if let Some(buf) = buffers.get_mut(&entity) {
                pointer.wheel = WheelDelta {
                    vertical: buf.vertical,
                    horizontal: buf.horizontal,
                };
                buf.reset();
            }
        });

        // DoubleClick を取得
        DOUBLE_CLICK_BUFFERS.with(|buffers| {
            let mut buffers = buffers.borrow_mut();
            if let Some(dc) = buffers.remove(&entity) {
                pointer.double_click = dc;
            }
        });

        // 修飾キー状態を取得
        MODIFIER_STATE.with(|state| {
            let state = state.borrow();
            if let Some(&(shift, ctrl)) = state.get(&entity) {
                pointer.shift_down = shift;
                pointer.ctrl_down = ctrl;
            }
        });

        pointer.timestamp = Instant::now();
    }
}

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use process_pointer_buffers instead")]
pub fn process_mouse_buffers(query: Query<(Entity, &mut PointerState)>) {
    process_pointer_buffers(query);
}

/// 一時的ポインター状態クリアシステム（FrameFinalize）
///
/// CommitComposition 後に実行され、1フレームのみ有効な状態をリセットする。
pub fn clear_transient_pointer_state(
    mut query: Query<&mut PointerState>,
    mut commands: Commands,
    leave_query: Query<Entity, With<PointerLeave>>,
) {
    // double_click, wheel をリセット（1フレームのみ有効）
    for mut pointer in query.iter_mut() {
        pointer.double_click = DoubleClick::None;
        pointer.wheel = WheelDelta::default();
    }

    // PointerLeave マーカー除去
    for entity in leave_query.iter() {
        commands.entity(entity).remove::<PointerLeave>();
    }
}

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use clear_transient_pointer_state instead")]
pub fn clear_transient_mouse_state(
    query: Query<&mut PointerState>,
    commands: Commands,
    leave_query: Query<Entity, With<PointerLeave>>,
) {
    clear_transient_pointer_state(query, commands, leave_query);
}

// ============================================================================
// デバッグ用監視システム
// ============================================================================

/// PointerState の Added/Changed を監視するデバッグシステム
///
/// Inputスケジュールで実行し、PointerStateの変化をログ出力する。
/// デバッグ用途のため、リリースビルドでは使用しないこと。
pub fn debug_pointer_state_changes(
    added_query: Query<(Entity, &PointerState), Added<PointerState>>,
    changed_query: Query<(Entity, &PointerState), Changed<PointerState>>,
) {
    use tracing::debug;

    // 新規追加（Enter）
    for (entity, pointer) in added_query.iter() {
        debug!(
            entity = ?entity,
            client_x = pointer.client_point.x,
            client_y = pointer.client_point.y,
            left = pointer.left_down,
            right = pointer.right_down,
            middle = pointer.middle_down,
            shift = pointer.shift_down,
            ctrl = pointer.ctrl_down,
            "[PointerState Added] Enter detected"
        );
    }

    // 変更（移動・ボタン・ホイール等）
    for (entity, pointer) in changed_query.iter() {
        // Added も Changed に含まれるのでスキップ
        // Note: bevy_ecs では Added は Changed のサブセット
        // ここでは移動・ボタン変化のみログ出力したい場合、
        // 別途フラグ管理が必要だが、デバッグ用なので許容

        // ダブルクリック検出時のみログ
        if pointer.double_click != DoubleClick::None {
            debug!(
                entity = ?entity,
                double_click = ?pointer.double_click,
                "[PointerState Changed] DoubleClick detected"
            );
        }

        // ホイール回転時のみログ
        if pointer.wheel.vertical != 0 || pointer.wheel.horizontal != 0 {
            debug!(
                entity = ?entity,
                vertical = pointer.wheel.vertical,
                horizontal = pointer.wheel.horizontal,
                "[PointerState Changed] Wheel detected"
            );
        }
    }
}

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use debug_pointer_state_changes instead")]
pub fn debug_mouse_state_changes(
    added_query: Query<(Entity, &PointerState), Added<PointerState>>,
    changed_query: Query<(Entity, &PointerState), Changed<PointerState>>,
) {
    debug_pointer_state_changes(added_query, changed_query);
}

/// PointerLeave マーカーを監視するデバッグシステム
///
/// Inputスケジュールで実行し、PointerLeaveの付与をログ出力する。
pub fn debug_pointer_leave(leave_query: Query<Entity, Added<PointerLeave>>) {
    use tracing::debug;

    for entity in leave_query.iter() {
        debug!(
            entity = ?entity,
            "[PointerLeave Added] Leave detected"
        );
    }
}

/// 後方互換性エイリアス
#[deprecated(since = "0.1.0", note = "Use debug_pointer_leave instead")]
pub fn debug_mouse_leave(leave_query: Query<Entity, Added<PointerLeave>>) {
    debug_pointer_leave(leave_query);
}
