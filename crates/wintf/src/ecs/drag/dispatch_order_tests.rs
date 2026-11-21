//! 1 回の配りにドラッグの終わりと次の始まりが重なる並びのテストと、
//! ドラッグを始めた窓の位置が取れないときのテスト。
//!
//! 世界は `EcsWorld::new()` の本番の登録どおりに組み、`Input` の段を回す。
//! ドラッグの状態はスレッド局所なので、各テストの頭で「何もない」へ戻す。

use crate::ecs::drag::seen_test_support::{take, watch};
use crate::ecs::drag::{
    DragAccumulatorResource, DragConfig, DragEvent, DragStartEvent, DragState, DragTransition,
    DraggingState, WindowDragContext, WindowDragContextResource, update_drag_state,
    update_dragging,
};
use crate::ecs::pointer::PhysicalPoint;
use crate::ecs::window::{Window, WindowPos};
use crate::ecs::world::{EcsWorld, Input};
use crate::ecs::{Point, SizeI};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::time::Instant;

fn force_idle() {
    update_drag_state(|state| *state = DragState::Idle);
}

/// 位置 `(x, y)` の窓（`WindowHandle` は無い・配られた知らせを控える）と、その子を 1 つ作り、子を返す。
fn spawn_window_child(world: &mut World, x: i32, y: i32) -> Entity {
    let window = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x, y }),
                size: Some(SizeI {
                    width: 200,
                    height: 100,
                }),
                ..Default::default()
            },
        ))
        .id();
    watch(world, window);
    world.spawn(ChildOf(window)).id()
}

fn started(entity: Entity, x: i32, y: i32) -> DragTransition {
    DragTransition::Started {
        entity,
        start_pos: PhysicalPoint::new(x, y),
        timestamp: Instant::now(),
    }
}

fn ended(entity: Entity, x: i32, y: i32) -> DragTransition {
    DragTransition::Ended {
        entity,
        end_pos: PhysicalPoint::new(x, y),
        cancelled: false,
    }
}

fn push(world: &World, transition: DragTransition) {
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(transition);
}

/// 同じ子を離してすぐ掴み直すと、終わりと始まりが 1 回の配りに重なっても `DraggingState` が残る。
#[test]
fn end_and_restart_in_one_dispatch_keeps_dragging_state() {
    force_idle();
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let a = spawn_window_child(world, 100, 200);

    push(world, started(a, 110, 210));
    world.run_schedule(Input);
    assert!(world.get::<DraggingState>(a).is_some(), "始まりで入る");

    push(world, ended(a, 120, 220));
    push(world, started(a, 130, 230));
    world.run_schedule(Input);

    assert!(
        world.get::<DraggingState>(a).is_some(),
        "次のドラッグの DraggingState は残る"
    );
}

/// 違う窓の子へ掴み直すと、前のドラッグの始まりの腕は状態を進めず、新しい窓の位置で進む。
#[test]
fn regrab_on_other_window_advances_with_new_window_pos() {
    force_idle();
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let a = spawn_window_child(world, 100, 200);
    let b = spawn_window_child(world, 500, 600);

    // 窓の手続きは配りの前に B の閾値まで進めている。
    update_drag_state(|state| {
        *state = DragState::JustStarted {
            entity: b,
            start_pos: PhysicalPoint::new(510, 610),
            current_pos: PhysicalPoint::new(520, 620),
            start_time: Instant::now(),
        }
    });
    push(world, started(a, 110, 210));
    push(world, ended(a, 120, 220));
    push(world, started(b, 510, 610));
    world.run_schedule(Input);

    let state = crate::ecs::drag::snapshot_drag_state();
    assert!(
        matches!(
            state,
            DragState::Dragging {
                entity,
                initial_window_pos: Point { x: 500, y: 600 },
                ..
            } if entity == b
        ),
        "B の窓の位置で Dragging へ進む: {state:?}"
    );
    force_idle();
}

/// 前のドラッグで積んだ移動量は、終わりと次の始まりが重なっても次のドラッグへ配られない。
#[test]
fn delta_of_previous_drag_is_not_dispatched_to_next() {
    force_idle();
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let a = spawn_window_child(world, 100, 200);
    let b = spawn_window_child(world, 500, 600);

    push(world, started(a, 110, 210));
    let acc = world.resource::<DragAccumulatorResource>().clone();
    acc.accumulate_delta(PhysicalPoint::new(7, 9));
    acc.update_position(PhysicalPoint::new(117, 219));
    push(world, ended(a, 117, 219));
    push(world, started(b, 510, 610));
    world.run_schedule(Input);

    let events = take::<DragEvent>(world);
    assert!(
        events.is_empty(),
        "前の移動量で DragEvent を配らない: {events:?}"
    );
    force_idle();
}

/// 位置の無い窓の子で始まりを配ると、記録を 1 行残し、控えの始めの位置は「無い」・動かすかは設定のまま。
#[test]
fn start_on_window_without_position_warns_and_leaves_origin_none() {
    force_idle();
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let window = world
        .spawn((
            Window::default(),
            WindowPos {
                position: None,
                ..Default::default()
            },
        ))
        .id();
    watch(world, window);
    let child = world
        .spawn((
            ChildOf(window),
            DragConfig {
                move_window: true,
                ..Default::default()
            },
        ))
        .id();

    push(world, started(child, 10, 20));
    let ((), events) = log_capture_kit::capture(|| world.run_schedule(Input));

    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "WARN がちょうど 1 行でない: {events:?}");
    let ctx = world
        .resource::<WindowDragContextResource>()
        .get()
        .expect("控えが書かれる");
    assert_eq!(ctx.initial_window_pos, None, "始めの位置は無い");
    assert!(ctx.move_window, "動かすかは DragConfig のまま真");
    let starts = take::<DragStartEvent>(world);
    assert_eq!(starts.len(), 1, "始まりの知らせは 1 件: {starts:?}");
    let ds = world.get::<DraggingState>(child).expect("印は今どおり入る");
    assert_eq!(
        ds.initial_window_pos, None,
        "始めの位置が無いのに (0,0) を入れない"
    );
    force_idle();
}

/// 始めの位置の無い控えで状態を進めると、控えが動かすと言っても窓を動かさない。
#[test]
fn advance_with_context_without_origin_does_not_move_window() {
    force_idle();
    let entity = World::new().spawn_empty().id();
    update_drag_state(|state| {
        *state = DragState::JustStarted {
            entity,
            start_pos: PhysicalPoint::new(10, 20),
            current_pos: PhysicalPoint::new(20, 30),
            start_time: Instant::now(),
        }
    });
    let ctx = WindowDragContextResource::new();
    ctx.set(WindowDragContext {
        hwnd: None,
        initial_window_pos: None,
        move_window: true,
        constraint: None,
    });

    update_dragging(PhysicalPoint::new(20, 30), Some(&ctx));

    let state = crate::ecs::drag::snapshot_drag_state();
    assert!(
        matches!(
            state,
            DragState::Dragging {
                move_window: false,
                ..
            }
        ),
        "始めの位置が無いので動かさない: {state:?}"
    );
    force_idle();
}
