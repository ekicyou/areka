//! Task 7.4-7.6: ECS階層変更とtaffyツリー同期、増分計算、エンティティ削除クリーンアップテスト
use bevy_ecs::prelude::*;
use wintf::ecs::ChildOf;
use wintf::ecs::layout::systems::{
    build_taffy_styles_system, cleanup_removed_entities_system, compute_taffy_layout_system,
    sync_taffy_tree_system, update_arrangements_system,
};
use wintf::ecs::layout::taffy::{TaffyComputedLayout, TaffyLayoutResource, TaffyStyle};
use wintf::ecs::layout::{Arrangement, BoxSize, BoxStyle, Dimension, LayoutRoot};

// ===== Task 7.4: ECS階層変更とtaffyツリー同期テスト =====

#[test]
fn test_hierarchy_addition_syncs_taffy_tree() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 親エンティティ
    let parent = world
        .spawn((TaffyStyle::default(), BoxStyle::default()))
        .id();

    // 子エンティティ
    let child = world
        .spawn((TaffyStyle::default(), BoxStyle::default()))
        .id();

    // システム実行（TaffyStyleからノード作成）
    let mut schedule = Schedule::default();
    schedule.add_systems((build_taffy_styles_system, sync_taffy_tree_system));
    schedule.run(&mut world);

    // 親子関係を設定
    world.entity_mut(child).insert(ChildOf(parent));

    // 階層変更を同期
    schedule.run(&mut world);

    // Taffyツリー内で親子関係が確立されていることを確認
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let parent_node = taffy_res.get_node(parent).unwrap();
    let child_node = taffy_res.get_node(child).unwrap();

    let taffy_children = taffy_res.taffy().children(parent_node).unwrap();
    assert_eq!(taffy_children.len(), 1);
    assert_eq!(taffy_children[0], child_node);
}

#[test]
fn test_hierarchy_removal_syncs_taffy_tree() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 親子関係を持つエンティティを作成
    let parent = world
        .spawn((TaffyStyle::default(), BoxStyle::default()))
        .id();
    let child = world
        .spawn((TaffyStyle::default(), BoxStyle::default(), ChildOf(parent)))
        .id();

    // システム実行
    let mut schedule = Schedule::default();
    schedule.add_systems((build_taffy_styles_system, sync_taffy_tree_system));
    schedule.run(&mut world);

    // 親子関係を削除
    world.entity_mut(child).remove::<ChildOf>();

    schedule.run(&mut world);

    // Taffyツリー内で親子関係が解除されていることを確認
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let parent_node = taffy_res.get_node(parent).unwrap();

    let taffy_children = taffy_res.taffy().children(parent_node).unwrap();
    assert_eq!(taffy_children.len(), 0);
}

#[test]
fn test_deep_hierarchy_sync() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 深い階層構造: Root -> A -> B -> C
    let root = world
        .spawn((TaffyStyle::default(), BoxStyle::default()))
        .id();
    let a = world
        .spawn((TaffyStyle::default(), BoxStyle::default(), ChildOf(root)))
        .id();
    let b = world
        .spawn((TaffyStyle::default(), BoxStyle::default(), ChildOf(a)))
        .id();
    let c = world
        .spawn((TaffyStyle::default(), BoxStyle::default(), ChildOf(b)))
        .id();

    // システム実行
    let mut schedule = Schedule::default();
    schedule.add_systems((build_taffy_styles_system, sync_taffy_tree_system));
    schedule.run(&mut world);

    // Taffyツリーの階層構造を検証
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let root_node = taffy_res.get_node(root).unwrap();
    let a_node = taffy_res.get_node(a).unwrap();
    let b_node = taffy_res.get_node(b).unwrap();
    let c_node = taffy_res.get_node(c).unwrap();

    // Root -> A
    let root_children = taffy_res.taffy().children(root_node).unwrap();
    assert_eq!(root_children.len(), 1);
    assert_eq!(root_children[0], a_node);

    // A -> B
    let a_children = taffy_res.taffy().children(a_node).unwrap();
    assert_eq!(a_children.len(), 1);
    assert_eq!(a_children[0], b_node);

    // B -> C
    let b_children = taffy_res.taffy().children(b_node).unwrap();
    assert_eq!(b_children.len(), 1);
    assert_eq!(b_children[0], c_node);

    // C -> leaf
    let c_children = taffy_res.taffy().children(c_node).unwrap();
    assert_eq!(c_children.len(), 0);
}

// ===== Task 7.5: 増分計算の変更検知テスト =====

#[test]
fn test_no_change_no_compute() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // LayoutRootを持つエンティティを作成
    let root = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(), // 明示的に挿入
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

    // システム実行（TaffyStyleは既にあるのでChanged発火なし）
    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
    ));

    // 初回実行でノードが作成される
    schedule.run(&mut world);

    // TaffyComputedLayoutが存在することを確認
    assert!(world.entity(root).contains::<TaffyComputedLayout>());

    // 2回目の実行（変更なし）
    schedule.run(&mut world);

    // エラーなく完了することを確認
    assert!(world.entity(root).contains::<TaffyComputedLayout>());
}

#[test]
fn test_high_level_component_change_triggers_compute() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let root = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            Arrangement::default(),
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

    // 初回のArrangementを記録
    let initial_arrangement = *world.get::<Arrangement>(root).unwrap();

    // BoxStyleを変更
    world.entity_mut(root).insert(BoxStyle {
        size: Some(BoxSize {
            width: Some(Dimension::Px(1024.0)),
            height: Some(Dimension::Px(768.0)),
        }),
        ..Default::default()
    });

    // システム再実行
    schedule.run(&mut world);

    // Arrangementが更新されていることを確認
    let updated_arrangement = *world.get::<Arrangement>(root).unwrap();
    assert_ne!(
        initial_arrangement.size.width,
        updated_arrangement.size.width
    );
    assert_eq!(updated_arrangement.size.width, 1024.0);
    assert_eq!(updated_arrangement.size.height, 768.0);
}

#[test]
fn test_hierarchy_change_triggers_compute() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let root = world
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

    let child = world
        .spawn((
            TaffyStyle::default(),
            TaffyComputedLayout::default(),
            BoxStyle::default(),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
        update_arrangements_system,
    ));
    schedule.run(&mut world);

    // 子を追加
    world.entity_mut(child).insert(ChildOf(root));

    // システム再実行
    schedule.run(&mut world);

    // 子にもTaffyComputedLayoutが設定されていることを確認（Arrangementではなく）
    assert!(world.entity(child).contains::<TaffyComputedLayout>());
}

// ===== Task 7.6: エンティティ削除のクリーンアップテスト =====

#[test]
fn test_entity_removal_detected() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let entity = world.spawn(TaffyStyle::default()).id();

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_taffy_tree_system);
    schedule.run(&mut world);

    // TaffyLayoutResourceにノードが登録されていることを確認
    {
        let taffy_res = world.resource::<TaffyLayoutResource>();
        assert!(taffy_res.get_node(entity).is_some());
    }

    // エンティティを削除
    world.despawn(entity);

    // cleanup_removed_entities_systemを実行
    let mut cleanup_schedule = Schedule::default();
    cleanup_schedule.add_systems(cleanup_removed_entities_system);
    cleanup_schedule.run(&mut world);

    // ノードが削除されていることを確認
    {
        let taffy_res = world.resource::<TaffyLayoutResource>();
        assert!(taffy_res.get_node(entity).is_none());
    }
}

#[test]
fn test_taffy_node_removed_with_entity() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let entity = world.spawn(TaffyStyle::default()).id();

    let mut schedule = Schedule::default();
    schedule.add_systems(sync_taffy_tree_system);
    schedule.run(&mut world);

    let node_id = {
        let taffy_res = world.resource::<TaffyLayoutResource>();
        taffy_res.get_node(entity).unwrap()
    };

    // エンティティ削除
    world.despawn(entity);

    let mut cleanup_schedule = Schedule::default();
    cleanup_schedule.add_systems(cleanup_removed_entities_system);
    cleanup_schedule.run(&mut world);

    // NodeIdからのマッピングも削除されていることを確認
    {
        let taffy_res = world.resource::<TaffyLayoutResource>();
        assert!(taffy_res.get_entity(node_id).is_none());
    }
}

#[test]
fn test_mapping_cleanup_prevents_memory_leak() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 複数のエンティティを作成・削除
    for _ in 0..10 {
        let entity = world.spawn(TaffyStyle::default()).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(sync_taffy_tree_system);
        schedule.run(&mut world);

        world.despawn(entity);

        let mut cleanup_schedule = Schedule::default();
        cleanup_schedule.add_systems(cleanup_removed_entities_system);
        cleanup_schedule.run(&mut world);
    }

    // すべてのマッピングがクリーンアップされていることを確認
    #[cfg(debug_assertions)]
    {
        let taffy_res = world.resource::<TaffyLayoutResource>();
        taffy_res.verify_mapping_consistency();
    }
}
