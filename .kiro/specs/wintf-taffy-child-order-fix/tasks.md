# Implementation Plan

## Task Overview

本仕様では、`sync_taffy_tree_system` および `visual_hierarchy_sync_system` が `Children` コンポーネントを権威的ソースとして使用するように修正し、エンティティのアーキタイプに関わらず兄弟順序を保証する。

## Tasks

- [ ] 1. (P) Layout レイヤーの階層同期修正
- [ ] 1.1 (P) sync_taffy_tree_system に Children クエリを追加
  - システムシグネチャに `children_query: Query<&Children>` パラメータを追加
  - Phase 3 の階層同期処理で `changed_hierarchy` から影響を受けた親エンティティを収集（`HashSet<Entity>`）
  - _Requirements: 1.1, 1.2_

- [ ] 1.2 (P) Children 順序に基づく taffy 子ノード設定処理を実装
  - 各影響親について `children_query.get(parent)` で `Children` を取得
  - `Children.iter()` 順序で各子の `get_node(child)` を呼び出し、`Some(node_id)` のみ `Vec<NodeId>` に収集
  - 親の `get_node(parent)` で `parent_node` を取得し、`taffy_mut().set_children(parent_node, &ordered_node_ids)` で子順序を一括設定
  - _Requirements: 1.1, 1.2_

- [ ] 2. (P) Graphics レイヤーの階層同期修正
- [ ] 2.1 (P) visual_hierarchy_sync_system に Children クエリを追加
  - システムシグネチャに `children_query: Query<&Children>` パラメータを追加
  - Phase 1 の未同期エンティティ収集後、各未同期エンティティの親を `HashSet<Entity>` に収集（affected_parents）
  - _Requirements: 2.1, 2.2_

- [ ] 2.2 (P) Children 順序に基づく Visual 階層再配置処理を実装
  - 各影響親について `children_query.get(parent)` で `Children` を取得
  - 親の `VisualGraphics` から `parent_visual` を取得し、`remove_all_visuals()` で既存 Z-order をリセット
  - `Children.iter()` 順序で各子の `VisualGraphics` を取得し、`visual()` が `Some(child_visual)` なら `parent_visual.add_visual(child_visual, false, None)` を実行
  - 各処理済み子の `parent_visual` キャッシュを更新
  - _Requirements: 2.1, 2.2_

- [ ] 3. (P) 回帰防止テストの実装
- [ ] 3.1 (P) 異なるアーキタイプ兄弟の taffy ツリー順序テスト
  - 同一親に異なるコンポーネント構成を持つ3つの子エンティティを spawn
  - `sync_taffy_tree_system` 実行後、`taffy().children(parent_node)` の順序が `Children` コンポーネントの順序と一致することを検証
  - _Requirements: 3.1_

- [ ] 3.2 (P) taffy_flex_demo 相当シナリオテスト
  - 3つの子のうち1つに追加コンポーネントを付与してアーキタイプを変更
  - レイアウト計算実行後、各子の `TaffyComputedLayout` の Y 座標が spawn 順序どおりに配置されていることを検証
  - _Requirements: 3.2_

- [ ] 3.3 (P) Visual 階層の兄弟順序テスト
  - 異なるアーキタイプを持つ兄弟エンティティの一部を未同期状態（`parent_visual.is_none()`）にする
  - `visual_hierarchy_sync_system` 実行後、影響を受けた親の全子の `parent_visual` キャッシュが更新されていることを検証
  - 各子の `parent_visual` が正しい親 Visual を参照していることを検証
  - _Requirements: 3.3_

- [ ] 4. システム統合検証
- [ ] 4.1 既存テストスイートの実行と回帰確認
  - `cargo test` で既存のすべてのテストが通過することを確認
  - 特に `layout/` と `graphics/` 配下のテストに注目
  - _Requirements: 1.1, 1.2, 2.1, 2.2_

- [ ] 4.2 サンプルアプリケーションでの動作確認
  - `taffy_flex_demo.rs` で ClickThrough-Container に追加コンポーネント（`Visual { opacity: 0.3 }`）を付与
  - アプリケーション実行時に3段コンテナの縦方向順序が spawn 順序どおりになることを目視確認
  - _Requirements: 3.2_
