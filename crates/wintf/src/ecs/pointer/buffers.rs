//! ポインターバッファ管理
//!
//! WndProcスレッドからECSへのデータ転送を管理する thread_local バッファと
//! ヘルパー関数を提供する。

use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

use super::types::{ButtonBuffer, PhysicalPoint, PointerButton, PointerState, WheelBuffer};

// ============================================================================
// thread_local! バッファ
// ============================================================================

thread_local! {
    /// Entity ごとの最新のポインタ位置（クライアント座標）
    pub(crate) static POINTER_BUFFERS: RefCell<HashMap<Entity, PhysicalPoint>> = RefCell::new(HashMap::new());

    /// Entity × Button ごとの ButtonBuffer
    pub(crate) static BUTTON_BUFFERS: RefCell<HashMap<(Entity, PointerButton), ButtonBuffer>> = RefCell::new(HashMap::new());

    /// Entity ごとの WheelBuffer
    pub(crate) static WHEEL_BUFFERS: RefCell<HashMap<Entity, WheelBuffer>> = RefCell::new(HashMap::new());

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

/// 最新のポインタ位置を置く（同じ entity の前の位置は上書き）
#[inline]
pub(crate) fn push_pointer_sample(entity: Entity, point: PhysicalPoint) {
    wake_pointer();
    tracing::trace!(
        entity = ?entity,
        x = point.x,
        y = point.y,
        thread_id = ?std::thread::current().id(),
        "[push_pointer_sample] Sample added"
    );
    POINTER_BUFFERS.with(|buffers| {
        buffers.borrow_mut().insert(entity, point);
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

        for (entity, point) in buffers.iter() {
            // Worldから該当エンティティのPointerStateを取得（無ければ捨てる）
            if let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity) {
                pointer_state.client_point = *point;
                pointer_state.local_point = *point;

                tracing::trace!(
                    entity = ?entity,
                    x = point.x,
                    y = point.y,
                    "[transfer_buffers_to_world] PointerState updated"
                );
            }
        }
        // 1 tick 分だけ渡す
        buffers.clear();
    });

    // BUTTON_BUFFERSからPointerStateへボタン状態を転送
    // down_receivedがtrueの場合のみ、ボタンが押されたとしてtrue設定
    // up_receivedがtrueの場合のみ、ボタンが離されたとしてfalse設定
    // どちらでもない場合は既存の状態を維持（エッジ検出）
    BUTTON_BUFFERS.with(|buffers| {
        let buffers = buffers.borrow();

        for ((entity, button), buf) in buffers.iter() {
            // 解放の旗は押下優先の分岐とは独立に立てる。同じ tick に押下と解放が
            // 入っても（下の `if/else if` が押下を選んでも）解放を落とさない。
            if buf.up_received
                && let Some(mut pointer_state) = world.get_mut::<PointerState>(*entity)
            {
                match button {
                    PointerButton::Left => pointer_state.released.left = true,
                    PointerButton::Right => pointer_state.released.right = true,
                    PointerButton::Middle => pointer_state.released.middle = true,
                    PointerButton::XButton1 => pointer_state.released.xbutton1 = true,
                    PointerButton::XButton2 => pointer_state.released.xbutton2 = true,
                }
            }

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

    // WHEEL_BUFFERS（窓ごと）から、その窓の中でポインタの状態を持つ entity（高々 1 つ）へ移す。
    // 持ち主が居なければ捨てる（要件 6.10）。どちらでも溜めは毎回空にする（1 tick 分だけ渡す）。
    let wheels: Vec<(Entity, WheelBuffer)> =
        WHEEL_BUFFERS.with(|buffers| buffers.borrow_mut().drain().collect());
    if !wheels.is_empty() {
        let holders: Vec<Entity> = world
            .query_filtered::<Entity, With<PointerState>>()
            .iter(world)
            .collect();
        for (window, wheel) in wheels {
            if wheel.vertical == 0 && wheel.horizontal == 0 {
                continue;
            }
            let owner = holders
                .iter()
                .copied()
                .find(|e| crate::ecs::find_owner_window(world, *e) == Some(window));
            match owner.and_then(|e| world.get_mut::<PointerState>(e)) {
                Some(mut pointer_state) => {
                    pointer_state.wheel.vertical = wheel.vertical;
                    pointer_state.wheel.horizontal = wheel.horizontal;
                }
                None => tracing::trace!(
                    window = ?window,
                    vertical = wheel.vertical,
                    horizontal = wheel.horizontal,
                    "[transfer_buffers_to_world] wheel dropped: no pointer holder in window"
                ),
            }
        }
    }

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
        MODIFIER_STATE.with(|b| b.borrow_mut().clear());
    }

    #[test]
    fn test_push_pointer_sample_keeps_only_latest() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn_empty().id();

        push_pointer_sample(e, PhysicalPoint::new(10, 20));
        push_pointer_sample(e, PhysicalPoint::new(11, 21));

        POINTER_BUFFERS.with(|buffers| {
            let buffers = buffers.borrow();
            assert_eq!(
                buffers.get(&e),
                Some(&PhysicalPoint::new(11, 21)),
                "最新の 1 つだけ"
            );
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
    fn test_transfer_buffers_to_world_updates_position() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        push_pointer_sample(e, PhysicalPoint::new(100, 200));
        push_pointer_sample(e, PhysicalPoint::new(150, 200));

        transfer_buffers_to_world(&mut world);

        let state = world.get::<PointerState>(e).unwrap();
        // 最新の位置が client/local へ反映
        assert_eq!(state.client_point, PhysicalPoint::new(150, 200));
        assert_eq!(state.local_point, PhysicalPoint::new(150, 200));

        // 転送後にポインターバッファは空になる
        POINTER_BUFFERS.with(|buffers| {
            assert!(buffers.borrow().get(&e).is_none(), "転送後は空");
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
    fn test_transfer_buffers_to_world_sets_released_flag_even_with_down() {
        // 同じ tick に押下と解放が入ると、押下優先の `if/else if` では解放が捨てられる。
        // 解放の旗は独立した文で立てるため、押下状態と同時に立つ。
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        record_button_down(e, PointerButton::Right);
        record_button_up(e, PointerButton::Right);
        transfer_buffers_to_world(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        assert!(s.right_down, "押下優先の既存分岐は不変");
        assert!(s.released.right, "同 tick でも解放の旗は落ちない");
        assert!(s.released.any(), "any() が真");
        assert!(!s.released.left, "他のボタンの旗は立たない");
    }

    #[test]
    fn test_transfer_buffers_to_world_released_flag_maps_all_buttons() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        record_button_up(e, PointerButton::Left);
        record_button_up(e, PointerButton::Right);
        record_button_up(e, PointerButton::Middle);
        record_button_up(e, PointerButton::XButton1);
        record_button_up(e, PointerButton::XButton2);

        transfer_buffers_to_world(&mut world);

        let r = world.get::<PointerState>(e).unwrap().released;
        assert!(r.left && r.right && r.middle);
        assert!(r.xbutton1 && r.xbutton2, "XButton も個別に写像");
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
    fn test_transfer_buffers_to_world_skips_entity_without_pointer_state() {
        reset_all_buffers();
        let mut world = World::new();
        // PointerState を持たないエンティティ
        let e = world.spawn_empty().id();

        push_pointer_sample(e, PhysicalPoint::new(5, 5));
        record_button_down(e, PointerButton::Left);
        set_modifier_state(e, true, false);

        // PointerState がなくてもパニックしない（get_mut が None → スキップ）
        transfer_buffers_to_world(&mut world);

        assert!(
            world.get::<PointerState>(e).is_none(),
            "PointerState は生成されない（転送先がなければ無視）"
        );
    }

    /// ホイールの移しのテスト用に、窓とその子（ポインタの状態を持つ）を作る。
    fn spawn_window_with_pointer_child(world: &mut World) -> (Entity, Entity) {
        let window = world.spawn(crate::ecs::window::Window::default()).id();
        let child = world.spawn((PointerState::default(), ChildOf(window))).id();
        (window, child)
    }

    #[test]
    fn test_transfer_wheel_moves_to_pointer_child_and_empties_buffer() {
        reset_all_buffers();
        let mut world = World::new();
        let (window, child) = spawn_window_with_pointer_child(&mut world);

        add_wheel_vertical(window, 120);
        add_wheel_vertical(window, 120);
        add_wheel_horizontal(window, -60);
        transfer_buffers_to_world(&mut world);

        let w = world.get::<PointerState>(child).unwrap().wheel;
        assert_eq!(w.vertical, 240, "縦の量が子へ移る");
        assert_eq!(w.horizontal, -60, "横の量も子へ移る");
        WHEEL_BUFFERS.with(|b| {
            let b = b.borrow();
            assert!(
                b.get(&window)
                    .is_none_or(|w| w.vertical == 0 && w.horizontal == 0),
                "移した後は溜めが空"
            );
        });
    }

    #[test]
    fn test_transfer_wheel_second_transfer_does_not_add_again() {
        reset_all_buffers();
        let mut world = World::new();
        let (window, child) = spawn_window_with_pointer_child(&mut world);

        add_wheel_vertical(window, 120);
        transfer_buffers_to_world(&mut world);
        assert_eq!(
            world.get::<PointerState>(child).unwrap().wheel.vertical,
            120
        );
        // FrameFinalize の代わりに 0 へ戻す
        world.get_mut::<PointerState>(child).unwrap().wheel = Default::default();
        transfer_buffers_to_world(&mut world);

        let w = world.get::<PointerState>(child).unwrap().wheel;
        assert_eq!(w.vertical, 0, "2 回目の移しで量が増えない");
    }

    #[test]
    fn test_transfer_wheel_without_owner_is_dropped() {
        reset_all_buffers();
        let mut world = World::new();
        // ポインタの状態を持つ entity が居ない窓
        let window = world.spawn(crate::ecs::window::Window::default()).id();

        add_wheel_vertical(window, 120);
        transfer_buffers_to_world(&mut world);

        WHEEL_BUFFERS.with(|b| {
            let b = b.borrow();
            assert!(
                b.get(&window)
                    .is_none_or(|w| w.vertical == 0 && w.horizontal == 0),
                "持ち主の居ない窓の溜めは捨てられる"
            );
        });
    }

    #[test]
    fn test_transfer_wheel_does_not_move_to_other_window_receiver() {
        reset_all_buffers();
        let mut world = World::new();
        let (window_a, child_a) = spawn_window_with_pointer_child(&mut world);
        let (_window_b, child_b) = spawn_window_with_pointer_child(&mut world);

        add_wheel_vertical(window_a, -120);
        transfer_buffers_to_world(&mut world);

        assert_eq!(
            world.get::<PointerState>(child_a).unwrap().wheel.vertical,
            -120
        );
        assert_eq!(
            world.get::<PointerState>(child_b).unwrap().wheel,
            Default::default(),
            "別の窓の受け手へは移らない"
        );
    }
}
