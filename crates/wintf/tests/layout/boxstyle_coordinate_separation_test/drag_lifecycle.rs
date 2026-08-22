//! Task 7.3: ドラッグ終了同期テスト
//! Task 7.4: WindowDragging ライフサイクルテスト

use bevy_ecs::prelude::*;
use std::time::Instant;
use wintf::ecs::drag::{
    DragAccumulatorResource, DragConfig, DragEndEvent, DragEvent, DragStartEvent, DragTransition,
    WindowDragContextResource, WindowDragging,
};
use wintf::ecs::pointer::PhysicalPoint;
use wintf::ecs::window::{DPI, Window, WindowPos};
use wintf::ecs::world::FrameCount;
use wintf::ecs::{Arrangement, GlobalArrangement, LayoutScale, Offset, Size, dispatch_drag_events};
use wintf::ecs::{Point, SizeI};

// =============================================================================
// Task 7.3: ドラッグ終了同期テスト
// =============================================================================

/// ドラッグ終了後に Arrangement.offset が直接更新されることを検証
/// （Changed<WindowPos> は発火させず、冗長な SetWindowPos を回避する）
#[test]
fn test_drag_end_syncs_window_pos_changed() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<bevy_ecs::message::Messages<DragStartEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEndEvent>>();

    // Window entity（WindowDragging マーカー付き = ドラッグ中を模擬）
    // ドラッグ中に WindowPos.position は bypass で (500, 600) に更新されたが、
    // Arrangement.offset はまだ旧値 (300, 400) のまま
    let window_entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 500, y: 600 }),
                size: Some(SizeI {
                    width: 800,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement {
                offset: Offset { x: 300.0, y: 400.0 },
                scale: LayoutScale { x: 1.0, y: 1.0 },
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
            GlobalArrangement::default(),
            WindowDragging, // ドラッグ中状態
        ))
        .id();

    // ドラッグ対象エンティティ（Window の子）
    let drag_entity = world
        .spawn((
            DragConfig::default(),
            bevy_ecs::hierarchy::ChildOf(window_entity),
        ))
        .id();

    // Changed<Arrangement> 検知用
    #[derive(Resource, Default)]
    struct ArrangementChangedCount(u32);

    fn detect_arrangement_change(
        query: Query<Entity, (With<Window>, Changed<Arrangement>)>,
        mut count: ResMut<ArrangementChangedCount>,
    ) {
        for _e in query.iter() {
            count.0 += 1;
        }
    }

    world.insert_resource(ArrangementChangedCount::default());

    let mut schedule = Schedule::default();
    schedule.add_systems(detect_arrangement_change);

    // 初回: Added で発火
    schedule.run(&mut world);
    let initial = world.resource::<ArrangementChangedCount>().0;

    // Ended 遷移を設定
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: drag_entity,
            end_pos: PhysicalPoint::new(550, 650),
            cancelled: false,
        });

    // dispatch_drag_events を実行（ドラッグ終了処理）
    dispatch_drag_events(&mut world);

    // Arrangement.offset が WindowPos.position から直接更新されていることを検証
    let arr = world.get::<Arrangement>(window_entity).unwrap();
    assert_eq!(
        arr.offset,
        Offset { x: 500.0, y: 600.0 },
        "DragEnd後にArrangement.offsetがWindowPos.positionと同期していること"
    );

    // Changed<Arrangement> 検知を実行
    schedule.run(&mut world);
    let after_end = world.resource::<ArrangementChangedCount>().0;

    assert!(
        after_end > initial,
        "ドラッグ終了後に Changed<Arrangement> が発火すること"
    );
}

/// ドラッグ終了後に WindowDragContextResource がクリアされることを検証
#[test]
fn test_drag_end_clears_context_resource() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<bevy_ecs::message::Messages<DragStartEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEndEvent>>();

    let window_entity = world
        .spawn((
            Window::default(),
            WindowPos {
                position: Some(Point { x: 100, y: 200 }),
                ..Default::default()
            },
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement::default(),
            WindowDragging,
        ))
        .id();

    let drag_entity = world
        .spawn((
            DragConfig::default(),
            bevy_ecs::hierarchy::ChildOf(window_entity),
        ))
        .id();

    // コンテキストをセットしておく
    world
        .resource::<WindowDragContextResource>()
        .set(wintf::ecs::drag::WindowDragContext {
            hwnd: None,
            initial_window_pos: Some(Point { x: 100, y: 200 }.into()),
            move_window: true,
            constraint: None,
        });

    // Ended を設定
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: drag_entity,
            end_pos: PhysicalPoint::new(200, 300),
            cancelled: false,
        });

    dispatch_drag_events(&mut world);

    // コンテキストがクリアされたことを確認
    let ctx = world.resource::<WindowDragContextResource>().get();
    if let Some(ctx) = ctx {
        assert!(
            ctx.hwnd.is_none() && ctx.initial_window_pos.is_none(),
            "ドラッグ終了後にコンテキストがクリアされるべき"
        );
    }
    // get() が None を返すか、中身が空であればOK
}

// =============================================================================
// Task 7.4: WindowDragging ライフサイクルテスト
// =============================================================================

/// ドラッグ開始で WindowDragging がWindow entityに挿入されることを検証
#[test]
fn test_window_dragging_inserted_on_drag_start() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<bevy_ecs::message::Messages<DragStartEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEndEvent>>();

    let window_entity = world
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
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    let drag_entity = world
        .spawn((
            DragConfig {
                move_window: true,
                ..Default::default()
            },
            bevy_ecs::hierarchy::ChildOf(window_entity),
        ))
        .id();

    // ドラッグ開始前: WindowDragging なし
    assert!(
        world.get::<WindowDragging>(window_entity).is_none(),
        "ドラッグ開始前は WindowDragging がないこと"
    );

    // Started 遷移を設定
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Started {
            entity: drag_entity,
            start_pos: PhysicalPoint::new(150, 250),
            timestamp: Instant::now(),
        });

    dispatch_drag_events(&mut world);

    // ドラッグ開始後: WindowDragging が挿入されていること
    assert!(
        world.get::<WindowDragging>(window_entity).is_some(),
        "ドラッグ開始後は WindowDragging がWindow entityに存在すること"
    );
}

/// ドラッグ終了で WindowDragging がWindow entityから除去されることを検証
#[test]
fn test_window_dragging_removed_on_drag_end() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<bevy_ecs::message::Messages<DragStartEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEndEvent>>();

    // Window entity（ドラッグ中状態）
    let window_entity = world
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
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement::default(),
            WindowDragging, // 事前に挿入
        ))
        .id();

    let drag_entity = world
        .spawn((
            DragConfig::default(),
            bevy_ecs::hierarchy::ChildOf(window_entity),
        ))
        .id();

    // ドラッグ中: WindowDragging あり
    assert!(
        world.get::<WindowDragging>(window_entity).is_some(),
        "ドラッグ中は WindowDragging が存在すること"
    );

    // Ended 遷移を設定
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: drag_entity,
            end_pos: PhysicalPoint::new(350, 450),
            cancelled: false,
        });

    dispatch_drag_events(&mut world);

    // ドラッグ終了後: WindowDragging が除去されていること
    assert!(
        world.get::<WindowDragging>(window_entity).is_none(),
        "ドラッグ終了後は WindowDragging が除去されること"
    );
}

/// ドラッグ全ライフサイクル（Started → Ended）を通じた WindowDragging の挿入/除去
#[test]
fn test_window_dragging_full_lifecycle() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    world.insert_resource(DragAccumulatorResource::new());
    world.insert_resource(WindowDragContextResource::new());
    world.init_resource::<bevy_ecs::message::Messages<DragStartEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEvent>>();
    world.init_resource::<bevy_ecs::message::Messages<DragEndEvent>>();

    let window_entity = world
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
            DPI::default(),
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    let drag_entity = world
        .spawn((
            DragConfig {
                move_window: true,
                ..Default::default()
            },
            bevy_ecs::hierarchy::ChildOf(window_entity),
        ))
        .id();

    // Step 1: 開始前 → なし
    assert!(world.get::<WindowDragging>(window_entity).is_none());

    // Step 2: Started → あり
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Started {
            entity: drag_entity,
            start_pos: PhysicalPoint::new(150, 250),
            timestamp: Instant::now(),
        });
    dispatch_drag_events(&mut world);
    assert!(
        world.get::<WindowDragging>(window_entity).is_some(),
        "Started後: WindowDragging が存在すること"
    );

    // Step 3: Ended → なし
    world
        .resource::<DragAccumulatorResource>()
        .set_transition(DragTransition::Ended {
            entity: drag_entity,
            end_pos: PhysicalPoint::new(200, 300),
            cancelled: false,
        });
    dispatch_drag_events(&mut world);
    assert!(
        world.get::<WindowDragging>(window_entity).is_none(),
        "Ended後: WindowDragging が除去されること"
    );
}
