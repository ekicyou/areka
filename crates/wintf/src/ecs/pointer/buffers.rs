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
// バッファ操作ヘルパー（窓の手続きが使う）
// ============================================================================

/// 入力を投入したことを起床の旗へ伝える（`POINTER`）。
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
        tracing::debug!(
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
        tracing::debug!(
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
    // 1 tick 分だけ渡す（取り出して空にする）
    let points: Vec<(Entity, PhysicalPoint)> =
        POINTER_BUFFERS.with(|buffers| buffers.borrow_mut().drain().collect());
    for (entity, point) in points {
        if let Some(mut pointer_state) = world.get_mut::<PointerState>(entity) {
            pointer_state.client_point = point;

            tracing::trace!(
                entity = ?entity,
                x = point.x,
                y = point.y,
                "[transfer_buffers_to_world] PointerState updated"
            );
        }
    }

    // BUTTON_BUFFERS から PointerState へ、この tick に押された・離された旗を移す。
    // `*_down` は「この tick に押された」の旗で、配り（dispatch_pointer_events）の後に下りる。
    let button_bufs: Vec<((Entity, PointerButton), ButtonBuffer)> =
        BUTTON_BUFFERS.with(|buffers| buffers.borrow_mut().drain().collect());
    for ((entity, button), buf) in button_bufs.iter() {
        // 解放の旗は押下と独立に立てる。同じ tick に押下と解放が入っても解放を落とさない。
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
        }
    }

    // WHEEL_BUFFERS（窓ごと）から、その窓の中でポインタの状態を持つ entity（高々 1 つ）へ移す。
    // 持ち主が居なければ捨てる。どちらでも溜めは毎回空にする（1 tick 分だけ渡す）。
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

    // 届かない tick は前の値のまま（PointerState の欄が持つ）
    let modifiers: Vec<(Entity, (bool, bool))> =
        MODIFIER_STATE.with(|state| state.borrow_mut().drain().collect());
    for (entity, (shift_down, ctrl_down)) in modifiers {
        if let Some(mut pointer_state) = world.get_mut::<PointerState>(entity) {
            pointer_state.shift_down = shift_down;
            pointer_state.ctrl_down = ctrl_down;
        }
    }

    // ローカル座標は最後に全部書き直す。位置が届かず窓の手続きが直に入れた状態も含む。
    let holders: Vec<(Entity, PhysicalPoint)> = world
        .query::<(Entity, &PointerState)>()
        .iter(world)
        .map(|(e, s)| (e, s.client_point))
        .collect();
    for (entity, client) in holders {
        let local = local_point_of(world, entity, client);
        if let Some(mut pointer_state) = world.get_mut::<PointerState>(entity) {
            pointer_state.local_point = local;
        }
    }
}

/// クライアント座標を、entity の左上からの座標にする（物理ピクセル）。
/// local = client + (持ち主の窓の GlobalArrangement.bounds の左上 − entity の GlobalArrangement.bounds の左上)
/// （差を四捨五入して i32）。entity が窓そのものなら client のまま。
/// どちらかの GlobalArrangement が無い（配置の前）なら client を返し debug! を 1 行出す。
fn local_point_of(world: &World, entity: Entity, client: PhysicalPoint) -> PhysicalPoint {
    use crate::ecs::GlobalArrangement;
    let window = crate::ecs::find_owner_window(world, entity);
    if window == Some(entity) {
        return client;
    }
    let window_bounds = window.and_then(|w| world.get::<GlobalArrangement>(w));
    match (window_bounds, world.get::<GlobalArrangement>(entity)) {
        (Some(w), Some(e)) => PhysicalPoint::new(
            client.x + (w.bounds.left - e.bounds.left).round() as i32,
            client.y + (w.bounds.top - e.bounds.top).round() as i32,
        ),
        _ => {
            tracing::debug!(
                entity = ?entity,
                window = ?window,
                "[local_point_of] no arrangement; local_point = client_point"
            );
            client
        }
    }
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
        assert_eq!(state.client_point, PhysicalPoint::new(150, 200));
        assert_eq!(state.local_point, PhysicalPoint::new(150, 200));

        POINTER_BUFFERS.with(|buffers| {
            assert!(buffers.borrow().get(&e).is_none(), "転送後は空");
        });
    }

    #[test]
    fn test_transfer_buffers_to_world_empties_all_three_buffers() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        push_pointer_sample(e, PhysicalPoint::new(1, 2));
        record_button_down(e, PointerButton::Left);
        set_modifier_state(e, true, false);
        transfer_buffers_to_world(&mut world);

        assert!(POINTER_BUFFERS.with(|b| b.borrow().is_empty()), "位置は空");
        assert!(BUTTON_BUFFERS.with(|b| b.borrow().is_empty()), "ボタンは空");
        assert!(
            MODIFIER_STATE.with(|b| b.borrow().is_empty()),
            "修飾キーは空"
        );
    }

    #[test]
    fn test_transfer_buffers_to_world_press_is_true_only_in_its_tick() {
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        // 押下を移すと、その tick だけ真
        record_button_down(e, PointerButton::Left);
        transfer_buffers_to_world(&mut world);
        assert!(
            world.get::<PointerState>(e).unwrap().left_down,
            "押した tick は真"
        );

        // 配りの後は偽で、知らせの無い次の tick も偽のまま
        crate::ecs::pointer::dispatch_pointer_events(&mut world);
        assert!(
            !world.get::<PointerState>(e).unwrap().left_down,
            "配りの後は偽"
        );
        transfer_buffers_to_world(&mut world);
        assert!(
            !world.get::<PointerState>(e).unwrap().left_down,
            "次の tick も偽"
        );
    }

    #[test]
    fn test_transfer_buffers_to_world_sets_released_flag_even_with_down() {
        // 同じ tick に押下と解放が入っても、解放の旗は押下の旗と同時に立つ。
        reset_all_buffers();
        let mut world = World::new();
        let e = world.spawn(PointerState::default()).id();

        record_button_down(e, PointerButton::Right);
        record_button_up(e, PointerButton::Right);
        transfer_buffers_to_world(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        assert!(s.right_down, "押下の旗も立つ");
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

    /// 左上 (left, top)・大きさ 50×50 の `GlobalArrangement` を作る。
    fn arrangement_at(left: f32, top: f32) -> crate::ecs::GlobalArrangement {
        crate::ecs::GlobalArrangement {
            transform: windows_numerics::Matrix3x2::translation(left, top),
            bounds: crate::ecs::Rect {
                left,
                top,
                right: left + 50.0,
                bottom: top + 50.0,
            },
        }
    }

    #[test]
    fn test_transfer_local_point_is_from_entity_top_left() {
        reset_all_buffers();
        let mut world = World::new();
        let window = world
            .spawn((
                crate::ecs::window::Window::default(),
                arrangement_at(100.0, 200.0),
            ))
            .id();
        let child = world
            .spawn((
                PointerState::default(),
                ChildOf(window),
                arrangement_at(150.0, 250.0),
            ))
            .id();

        // 窓の子に移すと、子の左上からの座標になる
        push_pointer_sample(child, PhysicalPoint::new(60, 70));
        transfer_buffers_to_world(&mut world);
        let s = world.get::<PointerState>(child).unwrap();
        assert_eq!(s.client_point, PhysicalPoint::new(60, 70));
        assert_eq!(s.local_point, PhysicalPoint::new(10, 20), "子の左上から");

        // 位置を積まずに直に入れた状態（ダブルクリックの道）も書き直される
        *world.get_mut::<PointerState>(child).unwrap() = PointerState {
            client_point: PhysicalPoint::new(60, 70),
            local_point: PhysicalPoint::new(60, 70),
            ..Default::default()
        };
        transfer_buffers_to_world(&mut world);
        assert_eq!(
            world.get::<PointerState>(child).unwrap().local_point,
            PhysicalPoint::new(10, 20),
            "直に入れた状態も書き直される"
        );

        // 窓そのものはクライアント座標のまま
        world.entity_mut(child).remove::<PointerState>();
        world.entity_mut(window).insert(PointerState::default());
        push_pointer_sample(window, PhysicalPoint::new(60, 70));
        transfer_buffers_to_world(&mut world);
        let s = world.get::<PointerState>(window).unwrap();
        assert_eq!(s.local_point, s.client_point, "窓そのものは client と同じ");
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
