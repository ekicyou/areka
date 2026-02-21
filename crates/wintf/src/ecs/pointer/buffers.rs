//! ポインターバッファ管理
//!
//! WndProcスレッドからECSへのデータ転送を管理する thread_local バッファと
//! ヘルパー関数を提供する。

use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use super::types::{
    ButtonBuffer, CursorVelocity, DoubleClick, PhysicalPoint, PointerBuffer, PointerButton,
    PointerState, PositionSample, WheelBuffer,
};

// ============================================================================
// thread_local! バッファ
// ============================================================================

thread_local! {
    /// Entity ごとの PointerBuffer
    pub(crate) static POINTER_BUFFERS: RefCell<HashMap<Entity, PointerBuffer>> = RefCell::new(HashMap::new());

    /// Entity × Button ごとの ButtonBuffer
    pub(crate) static BUTTON_BUFFERS: RefCell<HashMap<(Entity, PointerButton), ButtonBuffer>> = RefCell::new(HashMap::new());

    /// Entity ごとの WheelBuffer
    pub(crate) static WHEEL_BUFFERS: RefCell<HashMap<Entity, WheelBuffer>> = RefCell::new(HashMap::new());

    /// Entity ごとの DoubleClick（tick 内で最初に検出されたもの）
    pub(crate) static DOUBLE_CLICK_BUFFERS: RefCell<HashMap<Entity, DoubleClick>> = RefCell::new(HashMap::new());

    /// Entity ごとの修飾キー状態（最新値）
    pub(crate) static MODIFIER_STATE: RefCell<HashMap<Entity, (bool, bool)>> = RefCell::new(HashMap::new());
}

// ============================================================================
// バッファ操作ヘルパー（handlers.rs から使用）
// ============================================================================

/// PointerBufferにサンプルを追加
#[inline]
pub(crate) fn push_pointer_sample(entity: Entity, x: f32, y: f32, timestamp: Instant) {
    tracing::trace!(
        entity = ?entity,
        x, y,
        thread_id = ?std::thread::current().id(),
        "[push_pointer_sample] Sample added"
    );
    POINTER_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry(entity).or_insert_with(PointerBuffer::new);
        buffer.push(PositionSample { x, y, timestamp });
    });
}

/// 後方互換性エイリアス
#[allow(dead_code)]
#[inline]
pub(crate) fn push_mouse_sample(entity: Entity, x: f32, y: f32, timestamp: Instant) {
    push_pointer_sample(entity, x, y, timestamp);
}

/// ButtonBufferにボタン押下を記録
#[inline]
pub(crate) fn record_button_down(entity: Entity, button: PointerButton) {
    BUTTON_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry((entity, button)).or_default();
        buffer.record_down();
        // デバッグ用に info レベルで出力
        tracing::info!(
            entity = ?entity,
            button = ?button,
            "[ButtonBuffer] record_button_down"
        );
    });
}

/// ButtonBufferにボタン解放を記録
#[inline]
pub(crate) fn record_button_up(entity: Entity, button: PointerButton) {
    BUTTON_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry((entity, button)).or_default();
        buffer.record_up();
        // デバッグ用に info レベルで出力
        tracing::info!(
            entity = ?entity,
            button = ?button,
            "[ButtonBuffer] record_button_up"
        );
    });
}

/// WheelBufferに垂直ホイール回転を累積
#[inline]
pub(crate) fn add_wheel_vertical(entity: Entity, delta: i16) {
    WHEEL_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry(entity).or_default();
        buffer.add_vertical(delta);
    });
}

/// WheelBufferに水平ホイール回転を累積
#[inline]
pub(crate) fn add_wheel_horizontal(entity: Entity, delta: i16) {
    WHEEL_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry(entity).or_default();
        buffer.add_horizontal(delta);
    });
}

/// 修飾キー状態を設定
#[inline]
pub(crate) fn set_modifier_state(entity: Entity, shift: bool, ctrl: bool) {
    MODIFIER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.insert(entity, (shift, ctrl));
    });
}

// ============================================================================
// WndProcスレッドからWorldへのデータ転送
// ============================================================================

/// WndProcスレッドのthread_localバッファからWorldのPointerStateに直接データを転送
///
/// この関数は`try_tick_world()`の冒頭（Inputスケジュール実行前）で呼ばれ、
/// WndProcスレッド（メインスレッド）で収集したポインター情報を、
/// マルチスレッドで実行されるシステムがアクセスできるように転送する。
pub(crate) fn transfer_buffers_to_world(world: &mut World) {
    // POINTER_BUFFERSからPointerStateへ位置情報を転送
    POINTER_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();

        for (entity, buffer) in buffers.iter_mut() {
            // 最新位置を取得
            if let Some(sample) = buffer.latest() {
                // 速度計算
                let (vx, vy) = buffer.calculate_velocity();

                // Worldから該当エンティティのPointerStateを取得または作成
                if let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity) {
                    // 既存のPointerStateを更新
                    pointer_state.client_point =
                        PhysicalPoint::new(sample.x as i32, sample.y as i32);
                    pointer_state.local_point = pointer_state.client_point;
                    pointer_state.velocity = CursorVelocity::new(vx, vy);

                    tracing::trace!(
                        entity = ?entity,
                        x = sample.x,
                        y = sample.y,
                        "[transfer_buffers_to_world] PointerState updated"
                    );
                }
            }

            // バッファをクリア
            buffer.clear();
        }
    });

    // BUTTON_BUFFERSからPointerStateへボタン状態を転送
    // down_receivedがtrueの場合のみ、ボタンが押されたとしてtrue設定
    // up_receivedがtrueの場合のみ、ボタンが離されたとしてfalse設定
    // どちらでもない場合は既存の状態を維持（エッジ検出）
    BUTTON_BUFFERS.with(|buffers| {
        let buffers = buffers.borrow();

        for ((entity, button), buf) in buffers.iter() {
            if buf.down_received {
                // ボタンが押された瞬間
                if let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity) {
                    match button {
                        PointerButton::Left => pointer_state.left_down = true,
                        PointerButton::Right => pointer_state.right_down = true,
                        PointerButton::Middle => pointer_state.middle_down = true,
                        PointerButton::XButton1 => pointer_state.xbutton1_down = true,
                        PointerButton::XButton2 => pointer_state.xbutton2_down = true,
                    }

                    tracing::trace!(
                        entity = ?entity,
                        button = ?button,
                        "[transfer_buffers_to_world] Button pressed"
                    );
                }
            } else if buf.up_received {
                // ボタンが離された瞬間
                if let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity) {
                    match button {
                        PointerButton::Left => pointer_state.left_down = false,
                        PointerButton::Right => pointer_state.right_down = false,
                        PointerButton::Middle => pointer_state.middle_down = false,
                        PointerButton::XButton1 => pointer_state.xbutton1_down = false,
                        PointerButton::XButton2 => pointer_state.xbutton2_down = false,
                    }

                    tracing::trace!(
                        entity = ?entity,
                        button = ?button,
                        "[transfer_buffers_to_world] Button released"
                    );
                }
            }
        }
    });

    // BUTTON_BUFFERSをリセット（転送完了後）
    BUTTON_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        for buf in buffers.values_mut() {
            buf.reset();
        }
    });

    // MODIFIER_STATEからPointerStateへ修飾キー状態を転送
    MODIFIER_STATE.with(|state| {
        let state = state.borrow();

        for (entity, (shift_down, ctrl_down)) in state.iter() {
            if let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity) {
                pointer_state.shift_down = *shift_down;
                pointer_state.ctrl_down = *ctrl_down;
            }
        }
    });
}
