# Requirements Document

## Project Description (Input)

sync_taffy_tree_system の子ノード追加順序が bevy_ecs のアーキタイプ反復順序に依存しており、
エンティティのコンポーネント構成（アーキタイプ）が異なると Flexbox レイアウトの兄弟順序が
spawn 順序と一致しなくなるバグの修正。

## 問題の詳細

### 再現手順
1. `taffy_flex_demo.rs` の ClickThrough-Container に `Visual { opacity: 0.3, .. }` を直接追加
2. これにより ClickThrough-Container のアーキタイプが FlexDemo-Container / RegionTest-Container と異なる状態に
3. `cargo run --example taffy_flex_demo` を実行
4. 3段コンテナの縦方向順序が崩壊（ClickThrough が最上段に移動）

### 根本原因

**ファイル**: `crates/wintf/src/ecs/layout/systems.rs` L143-189 (`sync_taffy_tree_system`)

```rust
// 階層変更を処理（新規エンティティの親子関係もここで設定）
for (entity, child_of) in changed_hierarchy.iter() {
    if let Some(node_id) = taffy_res.get_node(entity) {
        if let Some(parent_ref) = child_of {
            let parent_entity = parent_ref.parent();
            if let Some(parent_node) = taffy_res.get_node(parent_entity) {
                // 新しい親に追加（taffyが自動的に既存の親から削除する）
                let _ = taffy_res.taffy_mut().add_child(parent_node, node_id);
            }
        }
    }
}
```

- `changed_hierarchy` クエリの `iter()` はアーキタイプテーブル順で反復する
- `taffy.add_child()` は**末尾追加**
- 異なるアーキタイプのエンティティが混在すると、反復順序 ≠ spawn 順序 → taffy ツリーの子順序が不定になる

### 期待動作
- bevy_ecs の `Children` コンポーネントが保持する正式な兄弟順序に従って taffy ツリーの子順序を設定すべき
- エンティティのアーキタイプ（コンポーネント構成）が異なっても、spawn/ChildOf 挿入順序に基づくレイアウト順序が保証されるべき

### 影響範囲
- `sync_taffy_tree_system` (crates/wintf/src/ecs/layout/systems.rs)
- `visual_hierarchy_sync_system` (crates/wintf/src/ecs/graphics/systems.rs) — 同様の問題がある可能性
- taffy Flexbox レイアウト結果全般

### 関連コンテキスト
- bevy_ecs 0.18.0 の `Children` コンポーネントは `SmallVec<[Entity; 8]>` で兄弟順序を保持
- `ChildOf` は `Changed<ChildOf>` で個別検出されるが、兄弟間の順序情報を持たない
- taffy の `add_child` は常に末尾追加、`insert_child_at_index` で位置指定が可能
- DirectComposition の `AddVisual` も同様に順序依存（`insertabove` / `referencevisual` パラメータ）

## Requirements

### Requirement 1: Taffyツリーの子ノード兄弟順序保証
**Objective:** 開発者として、bevy_ecs の `Children` コンポーネントが保持する兄弟順序どおりに taffy ツリーの子ノードが並ぶことを保証したい。エンティティのアーキタイプ（コンポーネント構成）に関わらず、Flexbox レイアウトの表示順序がエンティティの spawn 順序（＝ChildOf 挿入順序）と一致するようにするためである。

#### Acceptance Criteria
1. When `sync_taffy_tree_system` が階層変更を処理するとき, the `sync_taffy_tree_system` shall 親エンティティの `Children` コンポーネントが保持する兄弟順序に従って taffy ツリーの子ノード順序を設定する
2. The `sync_taffy_tree_system` shall `Changed<ChildOf>` クエリの反復順序（アーキタイプテーブル順）に依存せず、`Children` の正式な順序を権威的なソースとして使用する

> **統合メモ**: 旧 Requirement 2（アーキタイプ非依存の順序保証）は本要件に統合。`Children` を権威的ソースとすることで、アーキタイプ差異による順序不定は解消される。

### Requirement 2: Visual階層同期の兄弟順序保証
**Objective:** 開発者として、DirectComposition Visual 階層における兄弟ビジュアルの順序も `Children` の兄弟順序に従うことを保証したい。`sync_taffy_tree_system` と同じ根本原因（Childrenコンポーネントを参照していない）を持つため、本仕様で一括修正するためである。

> **スコープ確定**: `visual_hierarchy_sync_system` も `sync_taffy_tree_system` と同様にアーキタイプ反復順序に依存している。本仕様のスコープは「親子階層はChildrenコンポーネントが権威ソースである」というポリシー適用であり、同じ問題を持つ両システムを今修正する。

#### Acceptance Criteria
1. When `visual_hierarchy_sync_system` が子 Visual を親 Visual に追加するとき, the `visual_hierarchy_sync_system` shall `Children` コンポーネントの兄弟順序に従った z-order でビジュアルを配置する
2. If `visual_hierarchy_sync_system` にもアーキタイプ反復順序への依存がある場合, the `visual_hierarchy_sync_system` shall `Children` の正式な順序を権威的なソースとして同様に修正される

### Requirement 3: 回帰防止テスト
**Objective:** 開発者として、異なるアーキタイプを持つ子エンティティの兄弟順序が正しく保たれることを検証する自動テストを持ちたい。将来の変更で同様のバグが再発しないようにするためである。

#### Acceptance Criteria
1. The テストスイート shall 異なるコンポーネント構成（アーキタイプ）を持つ複数の子エンティティを同一親に spawn し、taffy ツリーの子ノード順序が `Children` の順序と一致することを検証するテストケースを含む
2. The テストスイート shall `taffy_flex_demo` 相当のシナリオ（一部の子に追加コンポーネントを付与してアーキタイプを変えた状態）で兄弟順序が維持されることを検証するテストケースを含む
3. The テストスイート shall Visual 階層の兄弟順序が `Children` の順序と一致することを検証するテストケースを含む
