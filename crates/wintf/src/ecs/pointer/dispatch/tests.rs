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
