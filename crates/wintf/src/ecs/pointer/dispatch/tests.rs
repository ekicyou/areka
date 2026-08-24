use super::*;

#[test]
fn test_phase_tunnel() {
    let phase = Phase::Tunnel(42);
    assert!(phase.is_tunnel());
    assert!(!phase.is_bubble());
    assert_eq!(*phase.value(), 42);
}

#[test]
fn test_phase_bubble() {
    let phase = Phase::Bubble("test");
    assert!(!phase.is_tunnel());
    assert!(phase.is_bubble());
    assert_eq!(*phase.value(), "test");
}

#[test]
fn test_phase_clone() {
    let phase = Phase::Tunnel(100);
    let cloned = phase.clone();
    assert_eq!(*cloned.value(), 100);
}

#[test]
fn test_handler_component_size() {
    // ハンドラコンポーネントは fn ポインタのサイズのみ
    use std::mem::size_of;
    assert_eq!(
        size_of::<OnPointerPressed>(),
        size_of::<PointerEventHandler>()
    );
    assert_eq!(
        size_of::<OnPointerMoved>(),
        size_of::<PointerEventHandler>()
    );
}

#[test]
fn test_build_bubble_path_single_entity() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();

    let path = build_bubble_path(&world, entity);
    assert_eq!(path.len(), 1);
    assert_eq!(path[0], entity);
}

#[test]
fn test_build_bubble_path_hierarchy() {
    let mut world = World::new();
    let root = world.spawn_empty().id();
    let child = world.spawn(ChildOf(root)).id();
    let grandchild = world.spawn(ChildOf(child)).id();

    let path = build_bubble_path(&world, grandchild);
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], grandchild);
    assert_eq!(path[1], child);
    assert_eq!(path[2], root);
}

#[test]
fn test_dispatch_with_no_handlers() {
    let mut world = World::new();
    let entity = world.spawn(PointerState::default()).id();

    // ハンドラなしでもパニックしない
    dispatch_pointer_events(&mut world);

    // エンティティがまだ存在することを確認
    assert!(world.get_entity(entity).is_ok());
}

#[test]
fn test_dispatch_with_handler() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn test_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        ev: &Phase<PointerState>,
    ) -> bool {
        if ev.is_bubble() {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        false // 伝播続行
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let mut world = World::new();
    let entity = world
        .spawn((PointerState::default(), OnPointerMoved(test_handler)))
        .id();

    dispatch_pointer_events(&mut world);

    // Bubble フェーズでハンドラが呼ばれたことを確認
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    assert!(world.get_entity(entity).is_ok());
}

#[test]
fn test_dispatch_stop_propagation() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static TUNNEL_COUNT: AtomicU32 = AtomicU32::new(0);
    static BUBBLE_COUNT: AtomicU32 = AtomicU32::new(0);

    fn stopping_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        ev: &Phase<PointerState>,
    ) -> bool {
        if ev.is_tunnel() {
            TUNNEL_COUNT.fetch_add(1, Ordering::SeqCst);
            false // Tunnel では停止しない
        } else {
            BUBBLE_COUNT.fetch_add(1, Ordering::SeqCst);
            true // Bubble で停止
        }
    }

    fn never_called_in_bubble_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        ev: &Phase<PointerState>,
    ) -> bool {
        if ev.is_bubble() {
            BUBBLE_COUNT.fetch_add(100, Ordering::SeqCst); // 呼ばれたら大きな値を加算
        }
        false
    }

    TUNNEL_COUNT.store(0, Ordering::SeqCst);
    BUBBLE_COUNT.store(0, Ordering::SeqCst);

    let mut world = World::new();
    let root = world
        .spawn(OnPointerMoved(never_called_in_bubble_handler))
        .id();
    let child = world
        .spawn((
            ChildOf(root),
            PointerState::default(),
            OnPointerMoved(stopping_handler),
        ))
        .id();

    dispatch_pointer_events(&mut world);

    // Tunnel: root(無) → child(stopping_handler) → 1回
    // Bubble: child(stopping_handler) → 停止 → root は呼ばれない
    assert_eq!(TUNNEL_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(BUBBLE_COUNT.load(Ordering::SeqCst), 1);
    assert!(world.get_entity(child).is_ok());
}

#[test]
fn test_phase_value_on_bubble() {
    // value() は Tunnel/Bubble いずれでもデータを返す（既存は Tunnel のみ）
    let phase = Phase::Bubble(7);
    assert_eq!(*phase.value(), 7);
    assert!(phase.is_bubble());
    assert!(!phase.is_tunnel());
}

#[test]
fn test_dispatch_tunnel_order_is_root_to_sender() {
    // Tunnel フェーズは root → sender の順で呼ばれることを記録順で検証する
    use std::sync::Mutex;
    static ORDER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    fn root_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        ev: &Phase<PointerState>,
    ) -> bool {
        if ev.is_tunnel() {
            ORDER.lock().unwrap().push(0); // root
        }
        false
    }
    fn child_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        ev: &Phase<PointerState>,
    ) -> bool {
        if ev.is_tunnel() {
            ORDER.lock().unwrap().push(1); // child(sender)
        }
        false
    }

    ORDER.lock().unwrap().clear();

    let mut world = World::new();
    let root = world.spawn(OnPointerMoved(root_handler)).id();
    let _child = world
        .spawn((
            ChildOf(root),
            PointerState::default(),
            OnPointerMoved(child_handler),
        ))
        .id();

    dispatch_pointer_events(&mut world);

    // Tunnel は root(0) → child(1) の順
    assert_eq!(
        *ORDER.lock().unwrap(),
        vec![0, 1],
        "Tunnel は root→sender 順"
    );
}

#[test]
fn test_dispatch_pressed_gating_requires_main_button() {
    // OnPointerPressed は left/right/middle のいずれかが down のときのみ発火する。
    // XButton1/XButton2 だけでは発火しない（dispatch/mod.rs:231 のゲート条件）。
    use std::sync::atomic::{AtomicU32, Ordering};
    static PRESSED_COUNT: AtomicU32 = AtomicU32::new(0);

    fn pressed_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        ev: &Phase<PointerState>,
    ) -> bool {
        if ev.is_bubble() {
            PRESSED_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        false
    }

    // ケース1: XButton1 のみ down → 発火しない
    PRESSED_COUNT.store(0, Ordering::SeqCst);
    {
        let mut world = World::new();
        world.spawn((
            PointerState {
                xbutton1_down: true,
                ..Default::default()
            },
            OnPointerPressed(pressed_handler),
        ));
        dispatch_pointer_events(&mut world);
        assert_eq!(
            PRESSED_COUNT.load(Ordering::SeqCst),
            0,
            "XButton のみでは Pressed 不発火"
        );
    }

    // ケース2: Left down → 発火
    PRESSED_COUNT.store(0, Ordering::SeqCst);
    {
        let mut world = World::new();
        world.spawn((
            PointerState {
                left_down: true,
                ..Default::default()
            },
            OnPointerPressed(pressed_handler),
        ));
        dispatch_pointer_events(&mut world);
        assert_eq!(
            PRESSED_COUNT.load(Ordering::SeqCst),
            1,
            "Left down で Pressed 発火"
        );
    }
}

#[test]
fn test_dispatch_clears_button_state_after_dispatch() {
    // dispatch 後にボタン状態と double_click がクリアされる（次フレーム再発火防止、
    // dispatch/mod.rs:243-252）。位置・修飾キーはクリアされない。
    let mut world = World::new();
    let e = world
        .spawn(PointerState {
            left_down: true,
            right_down: true,
            middle_down: true,
            xbutton1_down: true,
            xbutton2_down: true,
            double_click: super::super::DoubleClick::Left,
            shift_down: true, // 修飾キーは保持されることを確認
            ..Default::default()
        })
        .id();

    dispatch_pointer_events(&mut world);

    let s = world.get::<PointerState>(e).unwrap();
    assert!(
        !s.left_down && !s.right_down && !s.middle_down,
        "主ボタンクリア"
    );
    assert!(!s.xbutton1_down && !s.xbutton2_down, "XButton クリア");
    assert_eq!(
        s.double_click,
        super::super::DoubleClick::None,
        "double_click クリア"
    );
    assert!(s.shift_down, "修飾キーは dispatch でクリアされない");
}

#[test]
fn test_dispatch_event_for_handler_guards_deleted_entity() {
    // path 内のエンティティが存在しない場合、dispatch_event_for_handler は
    // パニックせず静かに終了する（存在チェック、dispatch/mod.rs:165/181）。
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALL: AtomicU32 = AtomicU32::new(0);

    fn counting_handler(
        _world: &mut World,
        _sender: Entity,
        _entity: Entity,
        _ev: &Phase<PointerState>,
    ) -> bool {
        CALL.fetch_add(1, Ordering::SeqCst);
        false
    }

    CALL.store(0, Ordering::SeqCst);

    let mut world = World::new();
    let real = world
        .spawn((PointerState::default(), OnPointerMoved(counting_handler)))
        .id();
    // despawn して dangling になったエンティティ ID を path 先頭に混ぜる
    let ghost = world.spawn_empty().id();
    world.despawn(ghost);

    let event = PointerState::default();
    // path = [real, ghost] : Tunnel は rev で ghost→real。先頭の ghost で存在チェックに
    // 引っかかり即 return するため、real のハンドラには到達しない。
    dispatch_event_for_handler::<PointerState, OnPointerMoved>(
        &mut world,
        real,
        &[real, ghost],
        &event,
        |h| h.0,
    );

    // ghost に到達した時点で return するため、real のハンドラは呼ばれない。
    // パニックしないことが主眼（存在チェックの特性化）。
    assert_eq!(
        CALL.load(Ordering::SeqCst),
        0,
        "削除済みエンティティで静かに終了（real ハンドラ未到達）"
    );
    assert!(world.get_entity(real).is_ok());
}
