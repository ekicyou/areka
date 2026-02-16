# Research: wintf-taffy-child-order-fix

## サマリー

`sync_taffy_tree_system` および `visual_hierarchy_sync_system` が `Children` コンポーネントを参照せず、アーキタイプ反復順序に依存している問題の設計前調査。Gap Analysis の Option A（既存システム拡張）を推奨アプローチとして採用し、具体的な API 適合性・パフォーマンス影響・Visual 再配置戦略を調査した。

---

## 調査ログ

### 調査 1: taffy `set_children` API 適合性

**目的**: `TaffyTree::set_children()` が要件 R1 の実装に適合するか確認

**結果**:
- `TaffyTree::set_children(parent: NodeId, children: &[NodeId]) -> Result<(), TaffyError>`
- 既存の子ノードをすべて置換し、引数の配列順序がそのまま子順序になる
- 子ノードが他の親に属していた場合、自動的に旧親から削除される（`add_child` と同じ挙動）
- `taffy_res.taffy_mut().set_children(parent_node, &ordered_children)` の形で呼び出し可能
- 既存コードでは `set_children` は未使用だが、`taffy::TaffyTree` の公開 API として利用可能

**結論**: ✅ 完全適合。`Children` の順序で `NodeId` 配列を構築し `set_children` に渡すだけで要件を満たす。

### 調査 2: `Children` コンポーネントのクエリパターン

**目的**: 既存コードベースでの `Query<&Children>` 使用パターンを確認

**結果**:
- `crates/wintf/src/ecs/layout/systems.rs` L27, L66: `sync_arrangements_system` で `&Children` を使用（ルート→子の順序伝播）
- `crates/wintf/src/ecs/common/tree_system.rs` L26, L89, L242: 汎用ツリーシステムで `&Children` を使用
- `crates/wintf/src/ecs/graphics/systems.rs` L134: `draw_recursive`（旧方式、dead_code）で `&Children` を使用
- パターン: `Children` は `Deref<Target = [Entity]>` を実装しており、`for child in children.iter()` でスライスとして反復

**結論**: ✅ プロジェクト全体で確立されたパターン。`sync_taffy_tree_system` への `Query<&Children>` 追加は既存パターンに合致。

### 調査 3: Visual 再配置戦略

**目的**: `visual_hierarchy_sync_system` の兄弟順序修正に最適な DComp API 操作を決定

**調査内容**:
- `DCompositionVisualExt::add_visual(visual, insertabove, referencevisual)`: `insertabove=false, referencevisual=None` で最前面に追加
- `DCompositionVisualExt::remove_all_visuals()`: 全子 Visual を一括削除
- `DCompositionVisualExt::remove_visual(visual)`: 個別 Visual を削除

**選択肢**:

| 戦略 | 説明 | 長所 | 短所 |
|------|------|------|------|
| A: 全再構築 | `remove_all_visuals` → `Children` 順で `add_visual` | 実装単純、確実に正しい順序 | 未変更の親も再構築される可能性 |
| B: 増分更新 | `Children` のインデックスで `add_visual(insertabove, referencevisual)` を制御 | 変更最小 | `referencevisual` の管理が複雑 |
| C: ソートキー拡張 | 現行の depth ソートに sibling_index を追加 | 既存パターン維持 | Parent のソート安定性に依存 |

**結論**: 戦略 A を採用。理由:
1. `visual_hierarchy_sync_system` は `parent_visual.is_none()` の未同期エンティティのみ処理する設計であり、既に同期済みの Visual は処理対象外
2. 初回同期時（全エンティティが未同期）では全再構築と等価
3. 親変更時は `VisualGraphics` の `parent_visual` が `None` にリセットされるため、再同期が自然に発生
4. ただし、同一親の兄弟が**一部だけ**未同期の場合、兄弟間の順序が崩れる可能性がある → 同一親の子が1つでも未同期なら、その親の全子を `remove_all_visuals` + 再追加する戦略が安全

### 調査 4: 変更があった親のみ処理する最適化

**目的**: パフォーマンス影響を最小化するため、変更検知の範囲を限定する方法を調査

**結果**:
- `sync_taffy_tree_system`:
  - `changed_hierarchy` クエリが `Changed<ChildOf>` を検出 → 変更があった子の `parent()` を収集すれば、影響を受けた親の集合が得られる
  - `set_children` はべき等操作（同じ順序で再設定しても副作用なし）なので、多少の余分な再設定は許容される
  
- `visual_hierarchy_sync_system`:
  - 現行設計が `parent_visual.is_none()` で未同期を検出 → 変更検知は既に組み込まれている
  - 兄弟順序修正のため、未同期エンティティの親を収集し、その親の全子を再配置する必要がある

**結論**: ✅ 両システムとも、変更があった親エンティティの `Children` のみ処理する最適化が自然に実装可能。

### 調査 5: エラーハンドリング — `Children` に存在するが taffy ノード/Visual が未作成のケース

**目的**: `Children` に含まれるエンティティが taffy ノードや `VisualGraphics` を持たない場合の扱い

**結果**:
- `sync_taffy_tree_system`: `taffy_res.get_node(entity)` が `None` を返す → そのエンティティをスキップ（taffy ノードなし＝レイアウト参加なし）
- `visual_hierarchy_sync_system`: `VisualGraphics` コンポーネントを持たないエンティティは `vg_queries.p0()` のクエリに含まれない → 自然にスキップ
- 既存コードの `add_child` もエラーを `let _ =` で無視しているパターンが確認される

**結論**: ✅ 既存のスキップパターンと整合。`Children` のリスト内で対応するノード/Visual が見つからないエンティティは単にスキップする。

---

## アーキテクチャパターン評価

### 選択: Option A — 既存システム拡張（`Children` 参照 + `set_children` 一括設定）

**受容基準との整合**:

| 要件 | AC | 対応方針 |
|------|-----|---------|
| R1 | 1.1 | `set_children` で `Children` 順序の子ノードリストを一括設定 |
| R1 | 1.2 | `Children` が権威的ソース → `changed_hierarchy.iter()` の順序に非依存 |
| R2 | 2.1 | `remove_all_visuals` + `Children` 順 `add_visual` で Z-order を保証 |
| R2 | 2.2 | 同じ修正パターン（`Children` 参照）を `visual_hierarchy_sync_system` に適用 |
| R3 | 3.1 | 異なるアーキタイプの兄弟を spawn し taffy 子順序を検証 |
| R3 | 3.2 | `taffy_flex_demo` 相当の複合シナリオテスト |
| R3 | 3.3 | Visual 階層の順序検証（COM モック不要 → ECS レベルで更新順序を検証） |

---

## 設計上の決定事項

### D1: `sync_taffy_tree_system` の修正戦略

**決定**: `changed_hierarchy` から変更親エンティティを収集 → `Children` 取得 → `set_children` で一括設定

**根拠**:
- `set_children` は taffy のネイティブ API で、子ノードリストの完全な置換を行う
- `add_child`（末尾追加）の連続呼び出しと異なり、呼び出し順序に依存しない
- Gap Analysis で推奨された Option A そのもの

### D2: `visual_hierarchy_sync_system` の修正戦略

**決定**: 未同期エンティティの親を収集 → 同一親の全子を `remove_all_visuals` + `Children` 順で `add_visual`

**根拠**:
- 部分的な再配置（一部の子のみ `add_visual`）では兄弟間順序を保証できない
- `remove_all_visuals` + 全再追加は DirectComposition の原子的操作であり、`Commit()` 前にはフレームに反映されない → 視覚的アーティファクトなし
- 初回同期では全エンティティが未同期であり、全再構築と等価

### D3: `Query<&Children>` パラメータ追加の範囲

**決定**: 両システムに `children_query: Query<&Children>` を追加

**根拠**:
- 読み取り専用クエリであり、bevy_ecs スケジューリング上の競合は発生しない
- 既存コードベースで確立されたパターン（`sync_arrangements_system`, `tree_system` で同じ手法）

---

## リスクと緩和策

| リスク | 影響度 | 緩和策 |
|--------|--------|--------|
| `set_children` の呼び出し頻度増加によるパフォーマンス劣化 | 低 | 変更があった親のみ処理。`set_children` はべき等操作で副作用なし |
| Visual `remove_all_visuals` + 再追加による描画フリッカー | 低 | DirectComposition は `Commit()` までバッチ処理。原子的更新として反映 |
| `Children` が未同期（bevy_ecs 内部の伝播遅延）の可能性 | 極低 | bevy_ecs 0.18.0 では `Children` は `ChildOf` 変更時に同一フレーム内で自動更新される |

---

## 参考資料

- [bevy-ecs-hierarchy-api-guide.md](references/bevy-ecs-hierarchy-api-guide.md): bevy_ecs 0.18.0 階層 API リファレンス
- [gap-analysis.md](gap-analysis.md): 実装ギャップ分析（Option A/B/C 評価）
- [taffy documentation](https://docs.rs/taffy/0.9.2/taffy/): `TaffyTree::set_children`, `TaffyTree::children` API
- [DirectComposition reference](https://learn.microsoft.com/en-us/windows/win32/directcomp/reference): `IDCompositionVisual::AddVisual`, `RemoveAllVisuals`

