//! ドラッグイベントディスパッチ統合テスト。
//!
//! `dispatch_drag_events`（ECS スレッド側のディスパッチ）の
//! World 観測可能な挙動を特性化する。これらはデバイス非依存（bevy World 操作）であり、
//! 実マウス/HWND を必要としない経路のみを対象とする。
//!
//! NOTE: Started 経路は最終的に `update_dragging`（thread_local DRAG_STATE 操作）を呼ぶが、
//! 本テストが検証するのは World 側の効果（DraggingState 挿入・WindowDragging マーカー・
//! 受け手に届く DragStartEvent・DragEvent・DragEndEvent・Arrangement.offset 同期）であり、
//! thread_local 状態機械そのものは src/ecs/drag/state/ の in-source テストで検証済み。

use bevy_ecs::prelude::*;
use std::time::Instant;
use wintf::ecs::drag::{
    DragAccumulatorResource, DragConstraint, DragEndEvent, DragEvent, DragStartEvent,
    DragTransition, DraggingState, OnDrag, OnDragEnd, OnDragStart, WindowDragContextResource,
    WindowDragging, dispatch_drag_events,
};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::{Window, WindowPos};
use wintf::ecs::{PhysicalPoint, Point, SizeI};

/// dispatch_drag_events が必要とするリソースを登録した World を生成する。
/// production の `EcsWorld::new` と同じ登録を行い、受け手に届いた知らせの溜め（[`Seen`]）も置く。
fn make_world() -> World {
    let mut world = World::new();
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<Seen>();
    world
}

/// [`watch`] した entity の受け手に届いた知らせ（泡の段で 1 件ずつ）。
#[derive(Resource, Default)]
struct Seen {
    starts: Vec<DragStartEvent>,
    drags: Vec<DragEvent>,
    ends: Vec<DragEndEvent>,
}

fn seen_start(world: &mut World, _: Entity, _: Entity, ev: &Phase<DragStartEvent>) -> bool {
    if let Phase::Bubble(e) = ev {
        world.resource_mut::<Seen>().starts.push(e.clone());
    }
    false
}

fn seen_drag(world: &mut World, _: Entity, _: Entity, ev: &Phase<DragEvent>) -> bool {
    if let Phase::Bubble(e) = ev {
        world.resource_mut::<Seen>().drags.push(e.clone());
    }
    false
}

fn seen_end(world: &mut World, _: Entity, _: Entity, ev: &Phase<DragEndEvent>) -> bool {
    if let Phase::Bubble(e) = ev {
        world.resource_mut::<Seen>().ends.push(e.clone());
    }
    false
}

/// `entity` に、届いた知らせを [`Seen`] へ控える受け手を付ける（1 つの経路に 1 か所だけ）。
fn watch(world: &mut World, entity: Entity) {
    world.entity_mut(entity).insert((
        OnDragStart(seen_start),
        OnDrag(seen_drag),
        OnDragEnd(seen_end),
    ));
}

// =============================================================================
// dispatch_drag_events: Started 遷移
// =============================================================================

/// Started 遷移で、ドラッグ対象に DraggingState が挿入され、Window 祖先に WindowDragging が付与され、
/// DragStartEvent が受け手に 1 件届く。
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
    watch(&mut world, window);
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

    let ds = world
        .get::<DraggingState>(target)
        .expect("DraggingState が挿入されるべき");
    assert_eq!((ds.drag_start_pos.x, ds.drag_start_pos.y), (110, 210));
    // WindowHandle なしフォールバックでは initial_window_pos = WindowPos.position
    assert_eq!(ds.initial_window_pos, Some((100.0, 200.0)));
    // wndproc スレッドへ渡すコンテキストにも同じ位置が書かれる
    let ctx = world
        .resource::<WindowDragContextResource>()
        .get()
        .expect("context が読めるべき");
    assert_eq!(ctx.initial_window_pos, Some(Point { x: 100, y: 200 }));

    assert!(
        world.get::<WindowDragging>(window).is_some(),
        "Window 祖先に WindowDragging が付与されるべき"
    );

    let events = &world.resource::<Seen>().starts;
    assert_eq!(events.len(), 1, "DragStartEvent は 1 件届くべき");
    assert_eq!(events[0].target, target);
    assert_eq!((events[0].position.x, events[0].position.y), (110, 210));
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

/// Ended 遷移で DraggingState が削除され、WindowDragging が除去され、DragEndEvent が受け手に届き、
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
        ))
        .id();
    watch(&mut world, window);
    let target = world.spawn(ChildOf(window)).id();

    // 開始を配ってドラッグ中の記録と印を入れる（Ended で外れるはず）
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Started {
            entity: target,
            start_pos: PhysicalPoint::new(310, 410),
            timestamp: Instant::now(),
        });
    dispatch_drag_events(&mut world);
    assert!(world.get::<DraggingState>(target).is_some());
    assert!(world.get::<WindowDragging>(window).is_some());

    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: target,
            end_pos: PhysicalPoint::new(350, 450),
            cancelled: false,
        });

    dispatch_drag_events(&mut world);

    assert!(
        world.get::<DraggingState>(target).is_none(),
        "DraggingState は Ended で削除されるべき"
    );
    assert!(
        world.get::<WindowDragging>(window).is_none(),
        "WindowDragging は Ended で除去されるべき"
    );
    let arr = world.get::<Arrangement>(window).unwrap();
    assert_eq!(
        arr.offset,
        Offset { x: 300.0, y: 400.0 },
        "Arrangement.offset は WindowPos.position に同期されるべき"
    );

    let events = &world.resource::<Seen>().ends;
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
    watch(&mut world, window);
    let target = world.spawn(ChildOf(window)).id();

    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Started {
            entity: target,
            start_pos: PhysicalPoint::new(11, 21),
            timestamp: Instant::now(),
        });
    dispatch_drag_events(&mut world);

    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: target,
            end_pos: PhysicalPoint::new(11, 22),
            cancelled: true,
        });

    dispatch_drag_events(&mut world);

    let events = &world.resource::<Seen>().ends;
    assert_eq!(events.len(), 1);
    assert!(
        events[0].cancelled,
        "cancelled=true が DragEndEvent に伝播するべき"
    );

    let ctx = world
        .resource::<WindowDragContextResource>()
        .get()
        .expect("context が読めるべき");
    assert!(ctx.hwnd.is_none());
    assert!(!ctx.move_window);
}

// =============================================================================
// dispatch_drag_events: 速いドラッグ（開始と終了が 1 回の配りに同居）
// =============================================================================

/// 受け手が受け取った知らせの並びを記録するリソース。
#[derive(Resource, Default)]
struct ReceivedOrder(Vec<&'static str>);

fn record_start(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragStartEvent>,
) -> bool {
    if let wintf::ecs::pointer::Phase::Bubble(_) = ev {
        world.resource_mut::<ReceivedOrder>().0.push("start");
    }
    false
}

fn record_end(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEndEvent>,
) -> bool {
    if let wintf::ecs::pointer::Phase::Bubble(_) = ev {
        world.resource_mut::<ReceivedOrder>().0.push("end");
    }
    false
}

/// 開始と終了の種を続けて積み、配る処理を 1 回呼ぶと、開始 → 終了の順に両方が届き、
/// 配り終えた時点でドラッグ中の記録（DraggingState）と印（WindowDragging）が外れている。
#[test]
fn dispatch_started_and_ended_in_one_flush_delivers_both_in_order() {
    let mut world = make_world();
    world.init_resource::<ReceivedOrder>();

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
            Arrangement::default(),
        ))
        .id();
    watch(&mut world, window);
    let target = world
        .spawn((
            ChildOf(window),
            OnDragStart(record_start),
            OnDragEnd(record_end),
        ))
        .id();

    // 1 回の配りの間に開始と終了が続けて積まれる（速いドラッグ）
    let acc = world.resource::<DragAccumulatorResource>();
    acc.set_transition(DragTransition::Started {
        entity: target,
        start_pos: PhysicalPoint::new(110, 210),
        timestamp: Instant::now(),
    });
    acc.set_transition(DragTransition::Ended {
        entity: target,
        end_pos: PhysicalPoint::new(160, 260),
        cancelled: false,
    });

    dispatch_drag_events(&mut world);

    let starts = &world.resource::<Seen>().starts;
    assert_eq!(starts.len(), 1, "DragStartEvent は 1 件届くべき");
    assert_eq!(starts[0].target, target);
    let ends = &world.resource::<Seen>().ends;
    assert_eq!(ends.len(), 1, "DragEndEvent は 1 件届くべき");
    assert_eq!(ends[0].target, target);
    assert!(!ends[0].cancelled);

    assert_eq!(
        world.resource::<ReceivedOrder>().0,
        vec!["start", "end"],
        "受け手には開始 → 終了の順に届くべき"
    );

    assert!(
        world.get::<DraggingState>(target).is_none(),
        "配り終えた時点で DraggingState は外れているべき"
    );
    assert!(
        world.get::<WindowDragging>(window).is_none(),
        "配り終えた時点で WindowDragging は外れているべき"
    );
}

// =============================================================================
// dispatch_drag_events: DragEvent（累積デルタ非ゼロ）
// =============================================================================

/// current_dragging_entity があり累積デルタが非ゼロのとき DragEvent が受け手に届き、
/// start_position が DraggingState.drag_start_pos から取得される。
#[test]
fn dispatch_emits_drag_event_when_delta_nonzero() {
    let mut world = make_world();
    let target = world
        .spawn(DraggingState {
            drag_start_pos: PhysicalPoint::new(100, 100),
            initial_window_pos: Some((0.0, 0.0)),
        })
        .id();
    watch(&mut world, target);

    let acc = world.resource::<DragAccumulatorResource>();
    // Started でドラッグ対象を確定（current_dragging_entity = target）
    acc.set_transition(DragTransition::Started {
        entity: target,
        start_pos: PhysicalPoint::new(100, 100),
        timestamp: Instant::now(),
    });
    acc.update_position(PhysicalPoint::new(140, 160));
    acc.accumulate_delta(PhysicalPoint::new(40, 60));

    dispatch_drag_events(&mut world);

    // DragEvent が届く（target は Window 祖先を持たないが、Started 処理は
    // window_entity=None でも DraggingState を挿入し、DragEvent 経路は別途デルタで発火する）
    let events = &world.resource::<Seen>().drags;
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
            initial_window_pos: Some((0.0, 0.0)),
        })
        .id();
    watch(&mut world, target);

    let acc = world.resource::<DragAccumulatorResource>();
    acc.set_transition(DragTransition::Started {
        entity: target,
        start_pos: PhysicalPoint::new(0, 0),
        timestamp: Instant::now(),
    });
    acc.update_position(PhysicalPoint::new(0, 0));
    // accumulate_delta は呼ばない → delta = (0,0)

    dispatch_drag_events(&mut world);

    let events = &world.resource::<Seen>().drags;
    assert!(
        events.is_empty(),
        "デルタゼロでは DragEvent を発火しないべき"
    );
}

/// 遷移もデルタもない（current_dragging_entity=None）場合は、受け手の付いた entity が在っても
/// 何も届かない（パニックなし）。
#[test]
fn dispatch_noop_when_no_transition_and_no_entity() {
    let mut world = make_world();
    let watched = world.spawn_empty().id();
    watch(&mut world, watched);

    // 何も set せずに呼ぶ → flush は Some だが transition=None, entity=None
    dispatch_drag_events(&mut world);

    let seen = world.resource::<Seen>();
    assert!(seen.starts.is_empty() && seen.drags.is_empty() && seen.ends.is_empty());
}
