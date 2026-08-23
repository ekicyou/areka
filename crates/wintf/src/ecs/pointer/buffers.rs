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

/// 入力を投入したことを起床の旗へ伝える（設計 C16 の `POINTER`）。
///
/// 呼ぶのは WndProc スレッドである。旗は原子的な OR で錠を取らず、何度立てても
/// 同じ（冪等）なので、投入のたびに素直に呼んでよい。
#[inline]
fn wake_pointer() {
    crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::POINTER);
}

/// PointerBufferにサンプルを追加
#[inline]
pub(crate) fn push_pointer_sample(entity: Entity, x: f32, y: f32, timestamp: Instant) {
    wake_pointer();
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

/// ButtonBufferにボタン押下を記録
#[inline]
pub(crate) fn record_button_down(entity: Entity, button: PointerButton) {
    wake_pointer();
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
    wake_pointer();
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
    wake_pointer();
    WHEEL_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry(entity).or_default();
        buffer.add_vertical(delta);
    });
}

/// WheelBufferに水平ホイール回転を累積
#[inline]
pub(crate) fn add_wheel_horizontal(entity: Entity, delta: i16) {
    wake_pointer();
    WHEEL_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        let buffer = buffers.entry(entity).or_default();
        buffer.add_horizontal(delta);
    });
}

/// 修飾キー状態を設定
#[inline]
pub(crate) fn set_modifier_state(entity: Entity, shift: bool, ctrl: bool) {
    wake_pointer();
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
                    // 整数変換: `f32 as i32` は Rust では飽和的（NaN→0、範囲外→i32::MIN/MAX、
                    // UB/パニックなし）。本番でこの sample.x/y は WM_MOUSEMOVE lparam の
                    // `(lparam & 0xFFFF) as i16 as i32 as f32`（window_proc/mouse_move.rs）由来で
                    // i16 範囲 [-32768, 32767] に収まるため、i32 への切り戻しは無損失（切り捨て/
                    // オーバーフローなし）。外部から非有限値や範囲外値が混入しても飽和で吸収され
                    // パニック経路にはならない。
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

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// thread_local バッファを既知のクリーン状態にする。
    ///
    /// thread_local バッファはテスト実行スレッドで共有されるため、各テストは
    /// 自分が使う Entity キーのエントリのみを参照する設計だが、`transfer_buffers_to_world`
    /// は全エントリを走査する。隔離のため、transfer 系テストの冒頭で全マップを空にする。
    fn reset_all_buffers() {
        POINTER_BUFFERS.with(|b| b.borrow_mut().clear());
        BUTTON_BUFFERS.with(|b| b.borrow_mut().clear());
        WHEEL_BUFFERS.with(|b| b.borrow_mut().clear());
        DOUBLE_CLICK_BUFFERS.with(|b| b.borrow_mut().clear());
        MODIFIER_STATE.with(|b| b.borrow_mut().clear());
    }

    #[test]
    fn test_push_pointer_sample_accumulates_in_thread_local() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn_empty().id();

        push_pointer_sample(e, 10.0, 20.0, Instant::now());
        push_pointer_sample(e, 11.0, 21.0, Instant::now());

        POINTER_BUFFERS.with(|buffers| {
            let buffers = buffers.borrow();
            let buf = buffers.get(&e).expect("buffer created on first push");
            assert_eq!(buf.len(), 2, "2 samples accumulated");
            let latest = buf.latest().unwrap();
            assert_eq!(latest.x, 11.0);
            assert_eq!(latest.y, 21.0);
        });
    }

    #[test]
    fn test_record_button_down_up_sets_independent_flags() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn_empty().id();

        record_button_down(e, PointerButton::Left);
        record_button_up(e, PointerButton::Right);

        BUTTON_BUFFERS.with(|buffers| {
            let buffers = buffers.borrow();
            let left = buffers.get(&(e, PointerButton::Left)).unwrap();
            assert!(left.down_received, "Left は down のみ");
            assert!(!left.up_received);

            let right = buffers.get(&(e, PointerButton::Right)).unwrap();
            assert!(!right.down_received);
            assert!(right.up_received, "Right は up のみ");
        });
    }

    #[test]
    fn test_add_wheel_helpers_accumulate_separately() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn_empty().id();

        add_wheel_vertical(e, 120);
        add_wheel_vertical(e, 120);
        add_wheel_horizontal(e, -60);

        WHEEL_BUFFERS.with(|buffers| {
            let buffers = buffers.borrow();
            let buf = buffers.get(&e).unwrap();
            assert_eq!(buf.vertical, 240, "垂直は累積");
            assert_eq!(buf.horizontal, -60, "水平は独立して累積");
        });
    }

    #[test]
    fn test_set_modifier_state_overwrites_latest() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn_empty().id();

        set_modifier_state(e, true, false);
        set_modifier_state(e, false, true);

        MODIFIER_STATE.with(|state| {
            let state = state.borrow();
            // 最新値で上書き（累積ではない）
            assert_eq!(state.get(&e), Some(&(false, true)));
        });
    }

    #[test]
    fn test_transfer_buffers_to_world_updates_position_and_velocity() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        // 2 サンプル（速度計算に必要）。dt を十分に取って非ゼロ速度にする。
        let t0 = Instant::now();
        push_pointer_sample(e, 100.0, 200.0, t0);
        push_pointer_sample(e, 150.0, 200.0, t0 + std::time::Duration::from_millis(100));

        transfer_buffers_to_world(&mut world);

        let state = world.get::<PointerState>(e).unwrap();
        // 最新サンプルが client/local へ反映（f32→i32 切り捨て）
        assert_eq!(state.client_point, PhysicalPoint::new(150, 200));
        assert_eq!(state.local_point, PhysicalPoint::new(150, 200));
        // x 方向に 50px / 0.1s = 500px/s、y は不変で 0
        assert!(state.velocity.x > 0.0, "x 速度は正");
        assert_eq!(state.velocity.y, 0.0, "y 速度は 0");

        // 転送後にポインターバッファはクリアされる
        POINTER_BUFFERS.with(|buffers| {
            let buffers = buffers.borrow();
            assert!(buffers.get(&e).unwrap().is_empty(), "転送後はクリア");
        });
    }

    #[test]
    fn test_transfer_buffers_to_world_button_edge_detection_and_reset() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        // down のみ受信 → left_down = true
        record_button_down(e, PointerButton::Left);
        transfer_buffers_to_world(&mut world);
        assert!(
            world.get::<PointerState>(e).unwrap().left_down,
            "down 受信で押下状態 true"
        );

        // transfer 後 ButtonBuffer は reset される（down_received/up_received=false）。
        // フラグなしの transfer では既存の押下状態が維持される（エッジ検出）。
        transfer_buffers_to_world(&mut world);
        assert!(
            world.get::<PointerState>(e).unwrap().left_down,
            "イベントなしでは押下状態を維持"
        );

        // up 受信 → left_down = false
        record_button_up(e, PointerButton::Left);
        transfer_buffers_to_world(&mut world);
        assert!(
            !world.get::<PointerState>(e).unwrap().left_down,
            "up 受信で押下状態 false"
        );
    }

    #[test]
    fn test_transfer_buffers_to_world_maps_all_buttons() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        record_button_down(e, PointerButton::Left);
        record_button_down(e, PointerButton::Right);
        record_button_down(e, PointerButton::Middle);
        record_button_down(e, PointerButton::XButton1);
        record_button_down(e, PointerButton::XButton2);

        transfer_buffers_to_world(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        assert!(s.left_down && s.right_down && s.middle_down);
        assert!(s.xbutton1_down && s.xbutton2_down, "XButton も個別に写像");
    }

    #[test]
    fn test_transfer_buffers_to_world_applies_modifier_state() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        set_modifier_state(e, true, true);
        transfer_buffers_to_world(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        assert!(s.shift_down, "Shift 状態が転送される");
        assert!(s.ctrl_down, "Ctrl 状態が転送される");
    }

    #[test]
    fn test_transfer_buffers_to_world_i16_extreme_coords_are_exact() {
        // 本番の座標は WM_MOUSEMOVE lparam 由来で i16 範囲（[-32768, 32767]）。
        // その極値を transfer_buffers_to_world に通すと、f32→i32 切り戻しが無損失で
        // 正確な値になり（切り捨て/オーバーフローなし）、パニックもしないことを固定する。
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        let xmin = i16::MIN as f32; // -32768.0
        let ymax = i16::MAX as f32; // 32767.0
        push_pointer_sample(e, xmin, ymax, Instant::now());

        transfer_buffers_to_world(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        assert_eq!(
            s.client_point,
            PhysicalPoint::new(i16::MIN as i32, i16::MAX as i32),
            "i16 極値座標は無損失で i32 へ反映される"
        );
        assert_eq!(s.local_point, s.client_point, "local = client");
    }

    #[test]
    fn test_transfer_buffers_to_world_nonfinite_coords_saturate_without_panic() {
        // 防御的特性化: 本番経路では非有限/範囲外 f32 は供給されないが、万一混入しても
        // Rust の `f32 as i32` は飽和（NaN→0・+inf→i32::MAX・-inf→i32::MIN・範囲超過→飽和）
        // であり、パニック経路にはならないことを固定する（DoS パニックなしの安全鎖）。
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        // 2 サンプル（速度計算分岐も通す）。最新サンプルが NaN / +inf。
        let t0 = Instant::now();
        push_pointer_sample(e, 0.0, 0.0, t0);
        push_pointer_sample(
            e,
            f32::NAN,
            f32::INFINITY,
            t0 + std::time::Duration::from_millis(10),
        );

        // パニックしないことが主眼
        transfer_buffers_to_world(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        // NaN as i32 == 0、+inf as i32 == i32::MAX（Rust の飽和キャスト仕様）
        assert_eq!(s.client_point.x, 0, "NaN は 0 に飽和");
        assert_eq!(s.client_point.y, i32::MAX, "+inf は i32::MAX に飽和");
    }

    #[test]
    fn test_transfer_buffers_to_world_skips_entity_without_pointer_state() {
        reset_all_buffers();
        let mut world = World::new();
        // PointerState を持たないエンティティ
        let e = world.spawn_empty().id();

        push_pointer_sample(e, 5.0, 5.0, Instant::now());
        record_button_down(e, PointerButton::Left);
        set_modifier_state(e, true, false);

        // PointerState がなくてもパニックしない（get_mut が None → スキップ）
        transfer_buffers_to_world(&mut world);

        assert!(
            world.get::<PointerState>(e).is_none(),
            "PointerState は生成されない（転送先がなければ無視）"
        );
    }
}
