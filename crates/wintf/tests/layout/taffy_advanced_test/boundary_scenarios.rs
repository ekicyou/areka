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

/// 本番の Layout と同じ順（`.chain()`）で 1 回回す
fn run_layout(world: &mut World) {
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
    schedule.run(world);
}

fn px_box(width: f32, height: f32) -> Option<BoxSize> {
    Some(BoxSize {
        width: Some(Dimension::Px(width)),
        height: Some(Dimension::Px(height)),
    })
}

#[test]
fn test_many_siblings() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let root = world
        .spawn((
            BoxStyle {
                size: px_box(1000.0, 1000.0),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();

    // 100個の兄弟ノードを作成
    let children: Vec<Entity> = (0..100)
        .map(|_| world.spawn((BoxStyle::default(), ChildOf(root))).id())
        .collect();

    run_layout(&mut world);

    // taffy の根の節に 100 個の子が並び、どの子も根の高さ（既定の stretch）に伸びる
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let root_node = taffy_res.get_node(root).unwrap();
    assert_eq!(taffy_res.taffy().children(root_node).unwrap().len(), 100);
    for child in children {
        assert_eq!(world.get::<Arrangement>(child).unwrap().size.height, 1000.0);
    }
}

#[test]
fn test_deep_hierarchy() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 20階層の深いツリーを作成
    let mut parent = world
        .spawn((
            BoxStyle {
                size: px_box(800.0, 600.0),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();
    let mut current = parent;
    for _ in 0..20 {
        parent = current;
        current = world.spawn((BoxStyle::default(), ChildOf(parent))).id();
    }

    run_layout(&mut world);

    // 最も深い節の親は 1 つ上の実体の節で、高さは根から stretch で届く
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let deepest_node = taffy_res.get_node(current).unwrap();
    assert_eq!(
        taffy_res.taffy().parent(deepest_node),
        taffy_res.get_node(parent)
    );
    assert_eq!(
        world.get::<Arrangement>(current).unwrap().size.height,
        600.0
    );
}

#[test]
fn test_zero_size_box() {
    use wintf::ecs::layout::{AlignItems, JustifyContent};

    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let root = world
        .spawn((
            BoxStyle {
                size: px_box(100.0, 100.0),
                justify_content: Some(JustifyContent::CENTER),
                align_items: Some(AlignItems::CENTER),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();
    let entity = world
        .spawn((
            BoxStyle {
                size: px_box(0.0, 0.0),
                ..Default::default()
            },
            ChildOf(root),
        ))
        .id();

    run_layout(&mut world);

    // ゼロサイズのまま中央（計算して初めて出る位置）に置かれる
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!((arrangement.offset.x, arrangement.offset.y), (50.0, 50.0));
    assert_eq!(
        (arrangement.size.width, arrangement.size.height),
        (0.0, 0.0)
    );
}

#[test]
fn test_negative_margin_handling() {
    use wintf::ecs::layout::{BoxMargin, LengthPercentageAuto, Rect};

    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let root = world
        .spawn((
            BoxStyle {
                size: px_box(200.0, 200.0),
                ..Default::default()
            },
            LayoutRoot,
        ))
        .id();
    let entity = world
        .spawn((
            BoxStyle {
                size: px_box(100.0, 100.0),
                margin: Some(BoxMargin(Rect {
                    left: LengthPercentageAuto::Px(-10.0),
                    right: LengthPercentageAuto::Px(-10.0),
                    top: LengthPercentageAuto::Px(-10.0),
                    bottom: LengthPercentageAuto::Px(-10.0),
                })),
                ..Default::default()
            },
            ChildOf(root),
        ))
        .id();

    run_layout(&mut world);

    // 負のマージンの分だけ親の外へはみ出し、大きさは保つ
    let arrangement = world.get::<Arrangement>(entity).unwrap();
    assert_eq!((arrangement.offset.x, arrangement.offset.y), (-10.0, -10.0));
    assert_eq!(
        (arrangement.size.width, arrangement.size.height),
        (100.0, 100.0)
    );
}
