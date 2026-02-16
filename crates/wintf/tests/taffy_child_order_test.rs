//! Taffyツリーの子ノード兄弟順序テスト
//!
//! 異なるアーキタイプ（コンポーネント構成）を持つ兄弟エンティティでも、
//! `Children` コンポーネントの順序どおりに taffy ツリーの子順序が保たれることを検証する。
//! (Requirements: 3.1, 3.2)

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::*;
use taffy::prelude::*;
use wintf::ecs::layout::taffy::{TaffyComputedLayout, TaffyLayoutResource, TaffyStyle};
use wintf::ecs::layout::{BoxStyle, LayoutRoot};
use wintf::ecs::sync_taffy_tree_system;

/// テスト用マーカーコンポーネント（アーキタイプを変えるために使用）
#[derive(Component, Default)]
struct ExtraComponentA;

/// テスト用マーカーコンポーネント（別のアーキタイプを作る）
#[derive(Component, Default)]
struct ExtraComponentB;

/// ヘルパー: sync_taffy_tree_system を2回実行する（1回目でAdded検知、2回目でChanged検知をクリア）
fn run_sync_system(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(sync_taffy_tree_system);
    schedule.run(world);
}

// ===== Task 3.1: 異なるアーキタイプ兄弟の taffy ツリー順序テスト =====

/// 同一親に異なるコンポーネント構成を持つ3つの子エンティティを spawn し、
/// `taffy().children(parent_node)` の順序が `Children` コンポーネントの順序と一致することを検証。
#[test]
fn test_different_archetype_siblings_maintain_children_order_in_taffy() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // 親エンティティ
    let parent = world.spawn(TaffyStyle::default()).id();

    // 子エンティティ：それぞれ異なるアーキタイプ
    // child_a: TaffyStyle のみ
    let child_a = world.spawn((TaffyStyle::default(), ChildOf(parent))).id();
    // child_b: TaffyStyle + ExtraComponentA（異なるアーキタイプ）
    let child_b = world
        .spawn((TaffyStyle::default(), ExtraComponentA, ChildOf(parent)))
        .id();
    // child_c: TaffyStyle + ExtraComponentB（さらに別のアーキタイプ）
    let child_c = world
        .spawn((TaffyStyle::default(), ExtraComponentB, ChildOf(parent)))
        .id();

    // sync_taffy_tree_system を実行
    run_sync_system(&mut world);

    // Children コンポーネントの順序を取得
    let children = world
        .get::<Children>(parent)
        .expect("Parent should have Children");
    let children_order: Vec<Entity> = children.iter().collect();

    // spawn 順序と Children 順序が一致していることを確認
    assert_eq!(
        children_order,
        vec![child_a, child_b, child_c],
        "Children should reflect spawn order"
    );

    // taffy ツリーの子ノード順序を取得
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let parent_node = taffy_res
        .get_node(parent)
        .expect("Parent should have taffy node");
    let taffy_children = taffy_res
        .taffy()
        .children(parent_node)
        .expect("Should get children");

    // taffy 子ノード順序を Entity に変換
    let taffy_child_entities: Vec<Entity> = taffy_children
        .iter()
        .map(|node_id| {
            taffy_res
                .get_entity(*node_id)
                .expect("Should have entity mapping")
        })
        .collect();

    // taffy ツリーの子順序が Children の順序と一致することを検証
    assert_eq!(
        taffy_child_entities, children_order,
        "Taffy tree child order should match Children component order, \
         regardless of archetype differences. \
         Expected: {:?}, Got: {:?}",
        children_order, taffy_child_entities
    );
}

/// 5つの兄弟を持ち、交互に異なるアーキタイプを使う場合の順序保証テスト
#[test]
fn test_many_siblings_with_alternating_archetypes_maintain_order() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let parent = world.spawn(TaffyStyle::default()).id();

    // 交互に異なるアーキタイプの子を spawn
    let child_0 = world
        .spawn((TaffyStyle::default(), ExtraComponentA, ChildOf(parent)))
        .id();
    let child_1 = world.spawn((TaffyStyle::default(), ChildOf(parent))).id();
    let child_2 = world
        .spawn((TaffyStyle::default(), ExtraComponentA, ChildOf(parent)))
        .id();
    let child_3 = world.spawn((TaffyStyle::default(), ChildOf(parent))).id();
    let child_4 = world
        .spawn((TaffyStyle::default(), ExtraComponentB, ChildOf(parent)))
        .id();

    run_sync_system(&mut world);

    let children = world.get::<Children>(parent).unwrap();
    let children_order: Vec<Entity> = children.iter().collect();
    assert_eq!(
        children_order,
        vec![child_0, child_1, child_2, child_3, child_4]
    );

    let taffy_res = world.resource::<TaffyLayoutResource>();
    let parent_node = taffy_res.get_node(parent).unwrap();
    let taffy_children = taffy_res.taffy().children(parent_node).unwrap();
    let taffy_entities: Vec<Entity> = taffy_children
        .iter()
        .map(|n| taffy_res.get_entity(*n).unwrap())
        .collect();

    assert_eq!(
        taffy_entities, children_order,
        "Taffy order should match Children order for alternating archetypes"
    );
}

// ===== Task 3.2: taffy_flex_demo 相当シナリオテスト =====

/// 3つの FlexColumn 子のうち1つに追加コンポーネントを付与してアーキタイプを変更し、
/// レイアウト計算後の Y 座標が spawn 順序どおりに配置されていることを検証。
#[test]
fn test_flex_demo_scenario_with_different_archetypes() {
    use wintf::ecs::layout::compute_taffy_layout_system;

    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    // ルートコンテナ: Column 方向、固定サイズ
    let root = world
        .spawn((
            LayoutRoot,
            TaffyStyle::new(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: Dimension::length(400.0),
                    height: Dimension::length(600.0),
                },
                ..Default::default()
            }),
            TaffyComputedLayout::default(),
            BoxStyle::default(),
        ))
        .id();

    // 子A: 通常のアーキタイプ（高さ100px）
    let child_a = world
        .spawn((
            TaffyStyle::new(Style {
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(100.0),
                },
                ..Default::default()
            }),
            TaffyComputedLayout::default(),
            ChildOf(root),
        ))
        .id();

    // 子B: 追加コンポーネント付き（アーキタイプが異なる、高さ150px）
    let child_b = world
        .spawn((
            TaffyStyle::new(Style {
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(150.0),
                },
                ..Default::default()
            }),
            TaffyComputedLayout::default(),
            ExtraComponentA, // <- アーキタイプを変える
            ChildOf(root),
        ))
        .id();

    // 子C: 通常のアーキタイプ（高さ200px）
    let child_c = world
        .spawn((
            TaffyStyle::new(Style {
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(200.0),
                },
                ..Default::default()
            }),
            TaffyComputedLayout::default(),
            ChildOf(root),
        ))
        .id();

    // sync → compute を実行
    let mut schedule = Schedule::default();
    schedule.add_systems((sync_taffy_tree_system, compute_taffy_layout_system).chain());
    schedule.run(&mut world);

    // taffy ツリーから直接レイアウト結果を取得（TaffyComputedLayout の内部フィールドは pub(crate) のため）
    let taffy_res = world.resource::<TaffyLayoutResource>();
    let node_a = taffy_res.get_node(child_a).unwrap();
    let node_b = taffy_res.get_node(child_b).unwrap();
    let node_c = taffy_res.get_node(child_c).unwrap();
    let layout_a = taffy_res.taffy().layout(node_a).unwrap();
    let layout_b = taffy_res.taffy().layout(node_b).unwrap();
    let layout_c = taffy_res.taffy().layout(node_c).unwrap();

    // Column レイアウト: 子は上から順に配置される
    // child_a: y=0, height=100
    // child_b: y=100, height=150
    // child_c: y=250, height=200
    assert_eq!(
        layout_a.location.y, 0.0,
        "child_a should be at y=0 (first in Children order)"
    );
    assert_eq!(
        layout_b.location.y, 100.0,
        "child_b should be at y=100 (second in Children order, after child_a height=100)"
    );
    assert_eq!(
        layout_c.location.y, 250.0,
        "child_c should be at y=250 (third in Children order, after child_a+child_b = 100+150)"
    );
}

/// Children に含まれるが taffy ノード未作成のエンティティがスキップされることを検証
#[test]
fn test_children_without_taffy_node_are_skipped() {
    let mut world = World::new();
    world.init_resource::<TaffyLayoutResource>();

    let parent = world.spawn(TaffyStyle::default()).id();

    // child_a: TaffyStyle あり
    let child_a = world.spawn((TaffyStyle::default(), ChildOf(parent))).id();
    // child_no_taffy: TaffyStyle なし（taffy ノード作成されない）
    let _child_no_taffy = world.spawn(ChildOf(parent)).id();
    // child_c: TaffyStyle あり
    let child_c = world.spawn((TaffyStyle::default(), ChildOf(parent))).id();

    run_sync_system(&mut world);

    let taffy_res = world.resource::<TaffyLayoutResource>();
    let parent_node = taffy_res.get_node(parent).unwrap();
    let taffy_children = taffy_res.taffy().children(parent_node).unwrap();
    let taffy_entities: Vec<Entity> = taffy_children
        .iter()
        .map(|n| taffy_res.get_entity(*n).unwrap())
        .collect();

    // taffy ノードを持つ child_a と child_c のみが、Children 順で含まれる
    assert_eq!(
        taffy_entities,
        vec![child_a, child_c],
        "Only entities with TaffyStyle should appear in taffy children, in Children order"
    );
}
