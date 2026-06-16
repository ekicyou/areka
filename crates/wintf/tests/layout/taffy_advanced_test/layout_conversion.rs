//! Task 7.2: TaffyComputedLayout→Arrangement変換テスト
use bevy_ecs::prelude::*;
use wintf::ecs::layout::Arrangement;
use wintf::ecs::layout::systems::update_arrangements_system;
use wintf::ecs::layout::taffy::{TaffyComputedLayout, TaffyLayoutResource, TaffyStyle};

// ===== Task 7.2: TaffyComputedLayout→Arrangement変換テスト =====

#[test]
fn test_computed_layout_to_arrangement_conversion() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // TaffyComputedLayoutを持つエンティティを作成
    let computed = TaffyComputedLayout::default();
    let entity = world.spawn((TaffyStyle::default(), computed)).id();

    // update_arrangements_systemを実行
    let mut schedule = Schedule::default();
    schedule.add_systems(update_arrangements_system);
    schedule.run(&mut world);

    // Arrangementが挿入されていることを確認
    assert!(world.entity(entity).contains::<Arrangement>());
}

#[test]
fn test_computed_layout_position_to_arrangement_offset() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 位置とサイズを持つTaffyComputedLayoutを作成
    let layout = taffy::Layout {
        location: taffy::Point { x: 10.0, y: 20.0 },
        size: taffy::Size {
            width: 100.0,
            height: 50.0,
        },
        ..Default::default()
    };
    let computed = TaffyComputedLayout::from(layout);

    let entity = world.spawn((TaffyStyle::default(), computed)).id();

    // システム実行
    let mut schedule = Schedule::default();
    schedule.add_systems(update_arrangements_system);
    schedule.run(&mut world);

    // Arrangementの値を検証
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arrangement.offset.x, 10.0);
    assert_eq!(arrangement.offset.y, 20.0);
    assert_eq!(arrangement.size.width, 100.0);
    assert_eq!(arrangement.size.height, 50.0);
}

#[test]
fn test_arrangement_coordinate_system_consistency() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 複数のエンティティで座標変換の一貫性を検証
    let layouts = vec![
        (0.0, 0.0, 100.0, 100.0),
        (50.0, 50.0, 200.0, 150.0),
        (-10.0, -20.0, 80.0, 60.0), // 負の座標
    ];

    for (x, y, w, h) in layouts {
        let layout = taffy::Layout {
            location: taffy::Point { x, y },
            size: taffy::Size {
                width: w,
                height: h,
            },
            ..Default::default()
        };
        let computed = TaffyComputedLayout::from(layout);
        let entity = world.spawn((TaffyStyle::default(), computed)).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_arrangements_system);
        schedule.run(&mut world);

        let arrangement = world.get::<Arrangement>(entity).unwrap();
        assert_eq!(arrangement.offset.x, x, "X coordinate mismatch");
        assert_eq!(arrangement.offset.y, y, "Y coordinate mismatch");
        assert_eq!(arrangement.size.width, w, "Width mismatch");
        assert_eq!(arrangement.size.height, h, "Height mismatch");
    }
}
