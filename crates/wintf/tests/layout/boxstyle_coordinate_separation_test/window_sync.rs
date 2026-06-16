//! Task 7.5: update_arrangements Window offset スキップテスト

use bevy_ecs::prelude::*;
use wintf::ecs::layout::systems::update_arrangements_system;
use wintf::ecs::layout::taffy::{TaffyComputedLayout, TaffyStyle};
use wintf::ecs::window::Window;
use wintf::ecs::{Arrangement, LayoutScale, Offset, Size};

/// Window entity の Arrangement.offset が taffy の layout.location で上書きされないことを検証
#[test]
fn test_update_arrangements_skips_window_offset() {
    let mut world = World::new();

    // Window entity: taffy layout location = (50, 60) だが offset は (100, 200) を維持するはず
    let layout = taffy::Layout {
        location: taffy::Point { x: 50.0, y: 60.0 },
        size: taffy::Size {
            width: 800.0,
            height: 600.0,
        },
        ..Default::default()
    };

    let window_entity = world
        .spawn((
            Window::default(),
            TaffyStyle::default(),
            TaffyComputedLayout::from(layout),
            Arrangement {
                offset: Offset { x: 100.0, y: 200.0 },
                scale: LayoutScale::default(),
                size: Size {
                    width: 800.0,
                    height: 600.0,
                },
            },
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(update_arrangements_system);
    schedule.run(&mut world);

    let arr = world.get::<Arrangement>(window_entity).unwrap();
    // Window entity の offset は taffy の location (50, 60) ではなく、元の (100, 200) を維持
    assert_eq!(
        arr.offset.x, 100.0,
        "Window entity の offset.x は taffy location で上書きされないこと"
    );
    assert_eq!(
        arr.offset.y, 200.0,
        "Window entity の offset.y は taffy location で上書きされないこと"
    );
    // サイズは taffy 結果で更新される
    assert_eq!(arr.size.width, 800.0);
    assert_eq!(arr.size.height, 600.0);
}

/// 非 Window entity の Arrangement.offset は taffy layout.location で正しく更新されることを検証
#[test]
fn test_update_arrangements_applies_offset_for_non_window() {
    let mut world = World::new();

    let layout = taffy::Layout {
        location: taffy::Point { x: 50.0, y: 60.0 },
        size: taffy::Size {
            width: 200.0,
            height: 150.0,
        },
        ..Default::default()
    };

    let entity = world
        .spawn((TaffyStyle::default(), TaffyComputedLayout::from(layout)))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(update_arrangements_system);
    schedule.run(&mut world);

    let arr = world.get::<Arrangement>(entity).unwrap();
    // 非 Window entity は taffy location がそのまま offset に設定される
    assert_eq!(
        arr.offset.x, 50.0,
        "非 Window entity の offset.x は taffy location の値であるべき"
    );
    assert_eq!(
        arr.offset.y, 60.0,
        "非 Window entity の offset.y は taffy location の値であるべき"
    );
}

/// Window entity で Arrangement が未作成の場合、offset は (0, 0) で作成されることを検証
#[test]
fn test_update_arrangements_window_without_existing_arrangement() {
    let mut world = World::new();

    let layout = taffy::Layout {
        location: taffy::Point { x: 50.0, y: 60.0 },
        size: taffy::Size {
            width: 800.0,
            height: 600.0,
        },
        ..Default::default()
    };

    let window_entity = world
        .spawn((
            Window::default(),
            TaffyStyle::default(),
            TaffyComputedLayout::from(layout),
            // Arrangement なし
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(update_arrangements_system);
    schedule.run(&mut world);

    let arr = world.get::<Arrangement>(window_entity).unwrap();
    // Arrangement が新規作成される場合、offset は (0, 0)（taffy の 50, 60 ではない）
    assert_eq!(
        arr.offset.x, 0.0,
        "新規 Window Arrangement の offset.x は 0.0 であるべき"
    );
    assert_eq!(
        arr.offset.y, 0.0,
        "新規 Window Arrangement の offset.y は 0.0 であるべき"
    );
    // サイズは設定される
    assert_eq!(arr.size.width, 800.0);
    assert_eq!(arr.size.height, 600.0);
}
