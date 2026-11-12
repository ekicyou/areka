//! Task 7.3: EntityとNodeIdマッピング管理テスト
use bevy_ecs::prelude::*;
use wintf::ecs::layout::taffy::TaffyLayoutResource;

// ===== Task 7.3: EntityとNodeIdマッピングテスト =====

#[test]
fn test_create_node_and_mapping() {
    let mut world = World::new();
    let mut taffy_res = TaffyLayoutResource::default();

    let entity = world.spawn_empty().id();

    // ノード作成
    let node_id = taffy_res
        .create_node(entity)
        .expect("Failed to create node");

    // 順方向マッピング検証
    assert_eq!(taffy_res.get_node(entity), Some(node_id));

    // taffy の木に節が 1 つ在る
    assert_eq!(taffy_res.taffy().total_node_count(), 1);
}

#[test]
fn test_remove_node_and_mapping_cleanup() {
    let mut world = World::new();
    let mut taffy_res = TaffyLayoutResource::default();

    let entity = world.spawn_empty().id();
    taffy_res.create_node(entity).unwrap();

    // ノード削除
    taffy_res
        .remove_node(entity)
        .expect("Failed to remove node");

    // 対応も taffy の木の節も削除されていることを確認
    assert_eq!(taffy_res.get_node(entity), None);
    assert_eq!(taffy_res.taffy().total_node_count(), 0);
}

#[test]
fn test_mapping_consistency() {
    let mut world = World::new();
    let mut taffy_res = TaffyLayoutResource::default();

    // 複数のエンティティでマッピングの一貫性を検証
    let entities: Vec<Entity> = (0..5).map(|_| world.spawn_empty().id()).collect();

    let mut node_ids = Vec::new();
    for entity in &entities {
        let node_id = taffy_res.create_node(*entity).unwrap();
        node_ids.push(node_id);
    }

    // すべてのマッピングが正しいことを確認
    for (i, entity) in entities.iter().enumerate() {
        assert_eq!(taffy_res.get_node(*entity), Some(node_ids[i]));
    }

    // 1つ削除してもその他は影響を受けないことを確認
    taffy_res.remove_node(entities[2]).unwrap();
    assert_eq!(taffy_res.get_node(entities[2]), None);
    assert_eq!(taffy_res.get_node(entities[0]), Some(node_ids[0]));
    assert_eq!(taffy_res.get_node(entities[4]), Some(node_ids[4]));
    // taffy の木の節も 1 つだけ減る
    assert_eq!(taffy_res.taffy().total_node_count(), 4);
}
