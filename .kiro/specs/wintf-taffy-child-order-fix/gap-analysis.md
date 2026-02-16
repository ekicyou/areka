# Gap Analysis: wintf-taffy-child-order-fix

## 概要

`sync_taffy_tree_system` および `visual_hierarchy_sync_system` が bevy_ecs のアーキタイプ反復順序に依存しており、
エンティティのコンポーネント構成が異なると Flexbox レイアウトの兄弟順序や DComp Visual の Z-order が
spawn 順序と一致しなくなるバグのギャップ分析。

---

## 1. 現状調査

### 1.1 対象ファイルとコンポーネント

| 資産 | パス | 状態 |
|------|------|------|
| `sync_taffy_tree_system` | `crates/wintf/src/ecs/layout/systems.rs` L143-189 | **バグあり** |
| `visual_hierarchy_sync_system` | `crates/wintf/src/ecs/graphics/systems.rs` L881-992 | **潜在的バグ** |
| `TaffyLayoutResource` | `crates/wintf/src/ecs/layout/taffy.rs` | 既存（修正不要） |
| `DCompositionVisualExt` | `crates/wintf/src/com/dcomp.rs` | 既存（`add_visual`, `remove_all_visuals` 利用可能） |

### 1.2 既存のアーキテクチャパターン

- **階層管理**: `ChildOf` コンポーネントで親子関係を設定。bevy_ecs が自動的に親の `Children` コンポーネントを管理し、挿入順序を保持
- **変更検知**: `Changed<ChildOf>` / `Added<TaffyStyle>` クエリで差分検知。`Children` は順序の権威的ソースだが、現行システムでは参照されていない
- **インポート**: `bevy_ecs::hierarchy::{ChildOf, Children}` は `ecs/mod.rs` から re-export 済み。両ファイルで既にインポートされている

### 1.3 バグの根本原因

**`sync_taffy_tree_system`** (L171-183):
```rust
for (entity, child_of) in changed_hierarchy.iter() {
    // iter() はアーキタイプテーブル順で反復 → spawn順序と一致しない
    let _ = taffy_res.taffy_mut().add_child(parent_node, node_id);
    // add_child は末尾追加 → 反復順がそのままtaffy子順序になる
}
```

**`visual_hierarchy_sync_system`** (L898-903):
```rust
for (entity, child_of, child_vg, child_name) in child_query.iter() {
    // iter() はアーキタイプテーブル順 → 兄弟間順序不定
    if child_vg.parent_visual().is_none() {
        // 未同期エンティティを収集
    }
}
// depth でソート（親→子の順序のみ保証、兄弟間順序は保証されない）
updates.sort_by_key(|item| item.4);
```

---

## 2. 要件と既存資産のマッピング

| 要件 | 必要な技術能力 | 既存資産 | ギャップ |
|------|--------------|---------|---------|
| R1: Taffyツリー子順序保証（アーキタイプ非依存含む） | `Children` に基づく taffy 子順序設定 | `TaffyTree::set_children()` API 利用可能 | **Missing**: `sync_taffy_tree_system` が `Children` を参照していない |
| R2: Visual階層Z-order保証 | `Children` に基づく Visual 追加順序 | `add_visual`, `remove_all_visuals` API 利用可能 | **Missing**: `visual_hierarchy_sync_system` に兄弟順序ロジックなし（R1と同じ根本原因） |
| R3: 回帰防止テスト | アーキタイプ混在時の順序検証テスト | `taffy_advanced_test.rs` にテスト基盤あり | **Missing**: アーキタイプが異なる兄弟での順序検証テスト |

---

## 3. 実装アプローチ選択肢

### Option A: 既存システム拡張（`Children` 参照 + `set_children` 一括設定）

**修正対象**: `sync_taffy_tree_system`, `visual_hierarchy_sync_system`

**`sync_taffy_tree_system` の修正方針**:
- 階層変更処理後、変更があった親エンティティの `Children` を取得
- `Children` の順序に基づいて taffy の `set_children(parent_node, &ordered_children)` を呼び出し
- `add_child` による逐次追加を `set_children` による一括設定に置換

```rust
// 概念的な修正イメージ
// 1. changed_hierarchy から変更のあった親エンティティを収集
// 2. 各親の Children コンポーネントを取得
// 3. Children の順序で taffy ノードIDリストを構築
// 4. set_children で一括設定
```

**`visual_hierarchy_sync_system` の修正方針**:
- 現在の depth ソート後に、同一親の兄弟間を `Children` の順序でソート
- または、`Children` が参照可能なクエリを追加して兄弟順序を保証

**トレードオフ**:
- ✅ 最小限の変更（既存システム関数のロジック修正のみ）
- ✅ 既存のテストパターンを流用可能
- ✅ `set_children` は taffy API として用意されており堅実
- ❌ `sync_taffy_tree_system` に `Query<&Children>` パラメータ追加が必要（システムシグネチャ変更）
- ❌ 全子ノードの再設定は、変更の有無にかかわらず実行される可能性（パフォーマンス影響は軽微）

### Option B: 変更された親のみ `Children` で再同期（選択的再構築）

**修正対象**: 同上

**方針**:
- `changed_hierarchy` イテレーション後、変更があった **親エンティティの集合** を収集
- 各親エンティティについてのみ `Children` → `set_children` を実行
- 変更のない親はスキップ

**トレードオフ**:
- ✅ 必要最小限の処理のみ実行
- ✅ パフォーマンス影響が最小
- ❌ 親エンティティ収集のための追加ロジックが必要
- ❌ Option A と比較して実質的な差は小さい（初回同期時は全親が対象）

### Option C: ハイブリッド（taffy は `set_children`、Visual は `Children` 順ソート）

**方針**:
- taffy 側: Option A/B の `set_children` アプローチ
- Visual 側: 既存の depth ソートに加えて、同一 depth 内で `Children` のインデックス順にソート（`remove_all_visuals` + 順序付き `add_visual` は不要に）

**トレードオフ**:
- ✅ Visual 側の大幅なロジック変更を回避
- ✅ 既存の「未同期検出→収集→ソート→処理」パターンを維持
- ❌ ソートキーの拡張が必要（depth + sibling_index）
- ❌ `Children` から sibling_index を取得するための追加クエリが必要

---

## 4. 推奨アプローチ

**Option A（既存システム拡張）を推奨**。理由:
- 変更箇所が明確で影響範囲が限定的
- taffy の `set_children` API が用途に完全一致
- 初期化時（全エンティティが新規）でも正しく動作
- 「変更があった親のみ再同期」の最適化は Option A の中で自然に実装可能

---

## 5. 実装複雑度とリスク

| 項目 | 評価 | 根拠 |
|------|------|------|
| 工数 | **S（1-3日）** | 既存パターンの拡張、API は既に利用可能、テスト基盤あり |
| リスク | **Low** | 既知の API を使用、影響範囲が限定的、回帰テストで検証可能 |

---

## 6. Research Needed（設計フェーズで調査）

| 項目 | 詳細 |
|------|------|
| `visual_hierarchy_sync_system` の兄弟順序影響度 | 現在 Z-order が DComp Visual の `add_visual` 呼び出し順で決まる。アーキタイプ混在時に実際にどの程度 Z-order が崩れるか、再現確認が必要 |
| DComp Visual 再構築の安全性 | `remove_all_visuals` + 順序付き `add_visual` がフレーム間で視覚的アーティファクトを生じないか |

**解決済み調査項目**:
- **`Children` クエリ追加のパフォーマンス影響**: 読み取り専用 `Query<&Children>` であり、bevy_ecs のスケジューリング競合は発生しない。設計フェーズでの調査は不要。

---

## 7. 出力チェックリスト

- [x] 要件-資産マッピング（セクション 2: ギャップタグ付き）
- [x] Option A/B/C の評価とトレードオフ（セクション 3）
- [x] 工数（S）とリスク（Low）の評価（セクション 5）
- [x] 設計フェーズへの推奨事項（セクション 4: Option A 推奨）
- [x] 研究項目の明記（セクション 6: Visual 影響度、再構築最適化、Children 順序保証）
- [x] スコープ確定の反映（両システムを本仕様で一括修正）

---

## 8. 設計フェーズへの推奨事項

### 次のステップ

1. **Option A の詳細設計**:
   - `sync_taffy_tree_system` の `Query<&Children>` パラメータ追加と `set_children` ロジック実装
   - `visual_hierarchy_sync_system` の兄弟順序ソートロジック（深さ + sibling_index）実装
   
2. **Research Item の調査**:
   - Visual Z-order の実際の影響度を `taffy_flex_demo` 相当シナリオで確認
   - DComp Visual の再配置戦略を決定（増分更新 vs 全再構築）

3. **テスト設計**:
   - R3-AC1: 異なるアーキタイプの兄弟順序検証（taffy ツリー）
   - R3-AC2: `taffy_flex_demo` 相当シナリオ（アーキタイプ混在時の順序維持）
   - R3-AC3: Visual 階層の兄弟順序検証

4. **実装タスク分割**:
   - タスク 1: `sync_taffy_tree_system` 修正
   - タスク 2: `visual_hierarchy_sync_system` 修正
   - タスク 3: 回帰テスト実装
   - タスク 4: 既存テスト（`taffy_flex_demo`）での動作確認

### 設計フェーズで決定すべき事項

- **Visual 再配置戦略**: 既存 Visual を削除して再追加するか、増分更新するか
- **パフォーマンス最適化**: 変更があった親のみ処理する最適化の実装詳細
- **エラーハンドリング**: `Children` に存在するが taffy ノード/Visual が存在しないエンティティの扱い

---

_次のコマンド_: `/kiro-spec-design wintf-taffy-child-order-fix` で設計ドキュメント生成へ進む
