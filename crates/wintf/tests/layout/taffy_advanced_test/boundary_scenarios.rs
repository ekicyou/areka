//! Task 7.7: 境界値シナリオテスト
use bevy_ecs::prelude::*;
use wintf::ecs::ChildOf;
use wintf::ecs::layout::systems::{
    build_taffy_styles_system, compute_taffy_layout_system, sync_taffy_tree_system,
    update_arrangements_system,
};
use wintf::ecs::layout::taffy::{TaffyComputedLayout, TaffyLayoutResource, TaffyStyle};
use wintf::ecs::layout::{Arrangement, BoxSize, BoxStyle, Dimension, LayoutRoot};

// ===== Task 7.7: 境界値シナリオテスト =====

#[test]
fn test_empty_tree() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // エンティティなしでシステム実行
    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
    ));

    // クラッシュしないことを確認
    schedule.run(&mut world);
}

#[test]
fn test_single_node_tree() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let entity = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            Arrangement::default(),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(100.0)),
                    height: Some(Dimension::Px(100.0)),
                }),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            build_taffy_styles_system,
            sync_taffy_tree_system,
            compute_taffy_layout_system,
            update_arrangements_system,
        )
            .chain(),
    );
    schedule.run(&mut world);

    // TaffyComputedLayoutが設定されていることを確認
    assert!(world.entity(entity).contains::<TaffyComputedLayout>());

    // Arrangementも設定されていることを確認
    assert!(world.entity(entity).contains::<Arrangement>());

    // Arrangementの値を検証
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arrangement.size.width, 100.0);
    assert_eq!(arrangement.size.height, 100.0);
}

#[test]
fn test_many_siblings() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let root = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(1000.0)),
                    height: Some(Dimension::Px(1000.0)),
                }),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();

    // 100個の兄弟ノードを作成
    let mut children = Vec::new();
    for _ in 0..100 {
        let child = world
            .spawn((
                TaffyStyle::default(),
                TaffyComputedLayout::default(),
                BoxStyle::default(),
                ChildOf(root),
            ))
            .id();
        children.push(child);
    }

    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
        update_arrangements_system,
    ));

    // クラッシュしないことを確認
    schedule.run(&mut world);

    // すべての子にTaffyComputedLayoutが設定されていることを確認
    for child in children {
        assert!(world.entity(child).contains::<TaffyComputedLayout>());
    }
}

#[test]
fn test_deep_hierarchy() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 20階層の深いツリーを作成
    let mut current = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(800.0)),
                    height: Some(Dimension::Px(600.0)),
                }),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();

    for _ in 0..20 {
        let child = world
            .spawn((
                TaffyStyle::default(),
                TaffyComputedLayout::default(),
                BoxStyle::default(),
                ChildOf(current),
            ))
            .id();
        current = child;
    }

    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
        update_arrangements_system,
    ));

    // クラッシュしないことを確認
    schedule.run(&mut world);

    // 最深部のエンティティにもTaffyComputedLayoutが設定されていることを確認
    assert!(world.entity(current).contains::<TaffyComputedLayout>());
}

#[test]
fn test_zero_size_box() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let entity = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(0.0)),
                    height: Some(Dimension::Px(0.0)),
                }),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
        update_arrangements_system,
    ));

    // ゼロサイズでもクラッシュしないことを確認
    schedule.run(&mut world);

    // TaffyComputedLayoutが設定されていることを確認
    assert!(world.entity(entity).contains::<TaffyComputedLayout>());
    assert!(world.entity(entity).contains::<Arrangement>());

    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!(arrangement.size.width, 0.0);
    assert_eq!(arrangement.size.height, 0.0);
}

#[test]
fn test_negative_margin_handling() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    use wintf::ecs::layout::{BoxMargin, LengthPercentageAuto, Rect};

    let entity = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(100.0)),
                    height: Some(Dimension::Px(100.0)),
                }),
                margin: Some(BoxMargin(Rect {
                    left: LengthPercentageAuto::Px(-10.0),
                    right: LengthPercentageAuto::Px(-10.0),
                    top: LengthPercentageAuto::Px(-10.0),
                    bottom: LengthPercentageAuto::Px(-10.0),
                })),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
        update_arrangements_system,
    ));

    // 負のマージンでもクラッシュしないことを確認
    schedule.run(&mut world);

    // TaffyComputedLayoutが設定されていることを確認
    assert!(world.entity(entity).contains::<TaffyComputedLayout>());
    assert!(world.entity(entity).contains::<Arrangement>());
}
