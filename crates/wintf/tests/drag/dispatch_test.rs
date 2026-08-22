//! ドラッグイベントディスパッチ統合テスト（W6b-T）
//!
//! `dispatch_drag_events`（ECS スレッド側のディスパッチ）と `cleanup_drag_state` の
//! World 観測可能な挙動を特性化する。これらはデバイス非依存（bevy World 操作）であり、
//! 実マウス/HWND を必要としない経路のみを対象とする。
//!
//! NOTE: Started 経路は最終的に `update_dragging`（thread_local DRAG_STATE 操作）を呼ぶが、
//! 本テストが検証するのは World 側の効果（DraggingState 挿入・WindowDragging マーカー・
//! DragStartEvent メッセージ・DragEndEvent メッセージ・Arrangement.offset 同期）であり、
//! thread_local 状態機械そのものは src/ecs/drag/state.rs の in-source テストで検証済み。

use bevy_ecs::message::Messages;
use bevy_ecs::prelude::*;
use std::time::Instant;
use wintf::ecs::drag::{
    DragAccumulatorResource, DragConstraint, DragEndEvent, DragEvent, DragStartEvent,
    DragTransition, DraggingState, WindowDragContextResource, WindowDragging, cleanup_drag_state,
    dispatch_drag_events,
};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::window::{Window, WindowPos};
use wintf::ecs::{PhysicalPoint, Point, SizeI};

/// dispatch_drag_events が必要とするリソースを登録した World を生成する。
/// production の `EcsWorld::new`（src/ecs/world/mod.rs:59-67）と同じ登録を行う。
fn make_world() -> World {
    let mut world = World::new();
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<Messages<DragStartEvent>>();
    world.init_resource::<Messages<DragEvent>>();
    world.init_resource::<Messages<DragEndEvent>>();
    world
}

/// Messages から書き込まれたメッセージを drain して取得する。
fn drain_messages<E: bevy_ecs::message::Message + Clone>(world: &mut World) -> Vec<E> {
    let mut reader = world.resource_mut::<Messages<E>>();
    // update() で current → previous へ移し、MessageReader で読む簡便化として drain を使う
    reader.drain().collect()
}

// =============================================================================
// dispatch_drag_events: Started 遷移
// =============================================================================

/// Started 遷移で、ドラッグ対象に DraggingState が挿入され、Window 祖先に WindowDragging が付与され、
/// DragStartEvent が 1 件書き込まれる。
#[test]
fn dispatch_started_inserts_dragging_state_and_marker_and_event() {
    let mut world = make_world();

    // Window エンティティ（WindowHandle なし → WindowPos.position を initial_window_pos に使うフォールバック経路）
    let window = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
        ))
        .id();
    // ドラッグ対象（Window の子）
    let target = world.spawn(ChildOf(window)).id();

    // 累積器に Started 遷移をセット（wndproc が行う操作の代替）
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Started {
            entity: target,
            start_pos: PhysicalPoint::new(110, 210),
            timestamp: Instant::now(),
        });

    dispatch_drag_events(&mut world);

    // DraggingState が target に挿入されている
    let ds = world
        .get::<DraggingState>(target)
        .expect("DraggingState が挿入されるべき");
    assert_eq!((ds.drag_start_pos.x, ds.drag_start_pos.y), (110, 210));
    // WindowHandle なしフォールバックでは initial_inset = WindowPos.position
    assert_eq!(ds.initial_inset, (100.0, 200.0));

    // WindowDragging マーカーが Window エンティティに付与されている
    assert!(
        world.get::<WindowDragging>(window).is_some(),
        "Window 祖先に WindowDragging が付与されるべき"
    );

    // DragStartEvent が 1 件
    let events = drain_messages::<DragStartEvent>(&mut world);
    assert_eq!(events.len(), 1, "DragStartEvent は 1 件書き込まれるべき");
    assert_eq!(events[0].target, target);
    assert_eq!((events[0].position.x, events[0].position.y), (110, 210));
    assert!(events[0].is_primary);
}

/// Started 遷移で DragConfig.move_window と DragConstraint が WindowDragContextResource に転送される。
#[test]
fn dispatch_started_writes_drag_context_resource() {
    use wintf::ecs::drag::DragConfig;

    let mut world = make_world();
    let window = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 0, y: 0 }),
                size: Some(SizeI {
                    width: 640,
                    height: 480,
                }),
                ..Default::default()
            },
        ))
        .id();
    let constraint = DragConstraint {
        min_x: Some(-50),
        max_x: Some(1000),
        min_y: None,
        max_y: None,
    };
    let target = world
        .spawn((
            ChildOf(window),
            DragConfig {
                move_window: true,
                ..Default::default()
            },
            constraint,
        ))
        .id();

    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Started {
            entity: target,
            start_pos: PhysicalPoint::new(5, 5),
            timestamp: Instant::now(),
        });

    dispatch_drag_events(&mut world);

    let ctx = world
        .resource::<WindowDragContextResource>()
        .get()
        .expect("context が読めるべき");
    assert!(
        ctx.move_window,
        "DragConfig.move_window=true が転送されるべき"
    );
    let c = ctx.constraint.expect("DragConstraint が転送されるべき");
    assert_eq!(c.min_x, Some(-50));
    assert_eq!(c.max_x, Some(1000));
}

// =============================================================================
// dispatch_drag_events: Ended 遷移
// =============================================================================

/// Ended 遷移で DraggingState が削除され、WindowDragging が除去され、DragEndEvent が書き込まれ、
/// Arrangement.offset が WindowPos.position に同期される。
#[test]
fn dispatch_ended_removes_state_marker_and_syncs_offset() {
    let mut world = make_world();

    let window = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 300, y: 400 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            Arrangement {
                offset: Offset { x: 0.0, y: 0.0 },
                ..Default::default()
            },
            WindowDragging, // ドラッグ中マーカー（Ended で除去されるはず）
        ))
        .id();
    let target = world
        .spawn((
            ChildOf(window),
            DraggingState {
                drag_start_pos: PhysicalPoint::new(0, 0),
                initial_inset: (0.0, 0.0),
            },
        ))
        .id();

    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: target,
            end_pos: PhysicalPoint::new(350, 450),
            cancelled: false,
        });

    dispatch_drag_events(&mut world);

    // DraggingState が削除されている
    assert!(
        world.get::<DraggingState>(target).is_none(),
        "DraggingState は Ended で削除されるべき"
    );
    // WindowDragging が除去されている
    assert!(
        world.get::<WindowDragging>(window).is_none(),
        "WindowDragging は Ended で除去されるべき"
    );
    // Arrangement.offset が WindowPos.position(300,400) に同期されている
    let arr = world.get::<Arrangement>(window).unwrap();
    assert_eq!(
        arr.offset,
        Offset { x: 300.0, y: 400.0 },
        "Arrangement.offset は WindowPos.position に同期されるべき"
    );

    // DragEndEvent が 1 件、cancelled=false
    let events = drain_messages::<DragEndEvent>(&mut world);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].target, target);
    assert_eq!((events[0].position.x, events[0].position.y), (350, 450));
    assert!(!events[0].cancelled);
}

/// Ended(cancelled=true) のとき DragEndEvent.cancelled が true で伝播する。
#[test]
fn dispatch_ended_cancelled_propagates_flag() {
    let mut world = make_world();
    let window = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 10, y: 20 }),
                size: Some(SizeI {
                    width: 100,
                    height: 100,
                }),
                ..Default::default()
            },
            Arrangement::default(),
        ))
        .id();
    let target = world.spawn(ChildOf(window)).id();

    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: target,
            end_pos: PhysicalPoint::new(11, 22),
            cancelled: true,
        });

    dispatch_drag_events(&mut world);

    let events = drain_messages::<DragEndEvent>(&mut world);
    assert_eq!(events.len(), 1);
    assert!(
        events[0].cancelled,
        "cancelled=true が DragEndEvent に伝播するべき"
    );

    // WindowDragContextResource はクリアされている
    let ctx = world
        .resource::<WindowDragContextResource>()
        .get()
        .expect("context が読めるべき");
    assert!(ctx.hwnd.is_none());
    assert!(!ctx.move_window);
}

// =============================================================================
// dispatch_drag_events: DragEvent（累積デルタ非ゼロ）
// =============================================================================

/// current_dragging_entity があり累積デルタが非ゼロのとき DragEvent が書き込まれ、
/// start_position が DraggingState.drag_start_pos から取得される。
#[test]
fn dispatch_emits_drag_event_when_delta_nonzero() {
    let mut world = make_world();
    let target = world
        .spawn(DraggingState {
            drag_start_pos: PhysicalPoint::new(100, 100),
            initial_inset: (0.0, 0.0),
        })
        .id();

    let acc = world.resource::<DragAccumulatorResource>();
    // Started でドラッグ対象を確定（current_dragging_entity = target）
    acc.set_transition(DragTransition::Started {
        entity: target,
        start_pos: PhysicalPoint::new(100, 100),
        timestamp: Instant::now(),
    });
    // 位置更新 + デルタ累積
    acc.update_position(PhysicalPoint::new(140, 160));
    acc.accumulate_delta(PhysicalPoint::new(40, 60));

    dispatch_drag_events(&mut world);

    // DragEvent が書き込まれる（target は Window 祖先を持たないが、Started 処理は
    // window_entity=None でも DraggingState を挿入し、DragEvent 経路は別途デルタで発火する）
    let events = drain_messages::<DragEvent>(&mut world);
    assert_eq!(
        events.len(),
        1,
        "デルタ非ゼロで DragEvent が 1 件発火するべき"
    );
    assert_eq!(events[0].target, target);
    assert_eq!((events[0].position.x, events[0].position.y), (140, 160));
    assert_eq!(
        (events[0].start_position.x, events[0].start_position.y),
        (100, 100),
        "start_position は DraggingState.drag_start_pos から取得されるべき"
    );
}

/// 累積デルタがゼロのときは DragEvent を発火しない。
#[test]
fn dispatch_no_drag_event_when_delta_zero() {
    let mut world = make_world();
    let target = world
        .spawn(DraggingState {
            drag_start_pos: PhysicalPoint::new(0, 0),
            initial_inset: (0.0, 0.0),
        })
        .id();

    let acc = world.resource::<DragAccumulatorResource>();
    acc.set_transition(DragTransition::Started {
        entity: target,
        start_pos: PhysicalPoint::new(0, 0),
        timestamp: Instant::now(),
    });
    acc.update_position(PhysicalPoint::new(0, 0));
    // accumulate_delta は呼ばない → delta = (0,0)

    dispatch_drag_events(&mut world);

    let events = drain_messages::<DragEvent>(&mut world);
    assert!(
        events.is_empty(),
        "デルタゼロでは DragEvent を発火しないべき"
    );
}

/// flush 結果がないと判断される条件はないが、遷移もデルタもない（current_dragging_entity=None）
/// 場合はメッセージが一切発火しない（パニックなし）。
#[test]
fn dispatch_noop_when_no_transition_and_no_entity() {
    let mut world = make_world();

    // 何も set せずに呼ぶ → flush は Some だが transition=None, entity=None
    dispatch_drag_events(&mut world);

    assert!(drain_messages::<DragStartEvent>(&mut world).is_empty());
    assert!(drain_messages::<DragEvent>(&mut world).is_empty());
    assert!(drain_messages::<DragEndEvent>(&mut world).is_empty());
}

// =============================================================================
// cleanup_drag_state システム
// =============================================================================

/// cleanup_drag_state は DragEndEvent を受けて対象の DraggingState を削除する。
#[test]
fn cleanup_removes_dragging_state_on_end_event() {
    let mut world = make_world();
    let target = world
        .spawn(DraggingState {
            drag_start_pos: PhysicalPoint::new(0, 0),
            initial_inset: (0.0, 0.0),
        })
        .id();

    // DragEndEvent を書き込む
    world
        .resource_mut::<Messages<DragEndEvent>>()
        .write(DragEndEvent {
            target,
            position: PhysicalPoint::new(1, 1),
            cancelled: false,
            is_primary: true,
            timestamp: Instant::now(),
        });

    let mut schedule = Schedule::default();
    schedule.add_systems(cleanup_drag_state);
    schedule.run(&mut world);

    assert!(
        world.get::<DraggingState>(target).is_none(),
        "DragEndEvent 受信で DraggingState が削除されるべき"
    );
}

/// DragEndEvent がなければ cleanup_drag_state は DraggingState を削除しない（noop）。
#[test]
fn cleanup_noop_without_end_event() {
    let mut world = make_world();
    let target = world
        .spawn(DraggingState {
            drag_start_pos: PhysicalPoint::new(0, 0),
            initial_inset: (0.0, 0.0),
        })
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(cleanup_drag_state);
    schedule.run(&mut world);

    assert!(
        world.get::<DraggingState>(target).is_some(),
        "イベントなしでは DraggingState は残るべき"
    );
}
