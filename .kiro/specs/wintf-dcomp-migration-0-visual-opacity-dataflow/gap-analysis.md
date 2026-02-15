# ギャップ分析: wintf-dcomp-migration-0-visual-opacity-dataflow

## 分析サマリー

本分析は、`Opacity` コンポーネント（`ecs/layout/metrics.rs`）から `Visual.opacity` / `Visual.is_visible` フィールド（`ecs/graphics/components.rs`）へのデータフロー移行に必要な既存コードベースとのギャップを特定する。

---

## 1. 現状調査（Current State）

### 1.1 データソースの二重構造

| データソース | 定義場所 | 書き込み元 | 読み取り元 | 状態 |
|------------|---------|-----------|-----------|------|
| `Opacity` コンポーネント | `ecs/layout/metrics.rs` L110 | Example コード（手動 `.spawn(Opacity(0.5))`） | `visual_property_sync_system`, `hit_test_entity`, `hit_test_entity_ex` | **実使用中** |
| `Visual.opacity` フィールド | `ecs/graphics/components.rs` L252 | なし（常にデフォルト 1.0） | なし | **未使用** |
| `Visual.is_visible` フィールド | `ecs/graphics/components.rs` L251 | なし（常にデフォルト true） | なし | **未使用** |

### 1.2 `Opacity` コンポーネント 全参照マップ

#### プロダクションコード（4ファイル・8箇所）

| ファイル | 行 | 用途 | 移行影響 |
|---------|------|------|---------|
| `ecs/layout/metrics.rs` | L108-133 | 構造体定義 + impl + Default | `#[deprecated]` 付与 |
| `ecs/graphics/systems.rs` | L1011, L1019 | `visual_property_sync_system` クエリ | `Visual` に切り替え |
| `ecs/graphics/systems.rs` | L1076-1089 | Opacity 同期ロジック | `Visual.opacity` 読み取りに変更 |
| `ecs/layout/hit_test.rs` | L204-206 | `hit_test_entity` Opacity 読み取り | **Phase 0 スコープ内** (Req 3.5) |
| `ecs/layout/hit_test.rs` | L337-339 | `hit_test_entity_ex` Opacity 読み取り | **Phase 0 スコープ内** (Req 3.5) |
| `ecs/world.rs` | L433 | コメント内参照 | コメント更新 |

#### Example コード（2ファイル・8箇所）

| ファイル | 行 | 値 | 移行方法 |
|---------|------|-----|---------|
| `examples/taffy_flex_demo.rs` | L79 | import | `Visual` import に変更 |
| `examples/taffy_flex_demo.rs` | L317 | `Opacity(1.0)` | 削除（Visual default と同一） |
| `examples/taffy_flex_demo.rs` | L372, L403, L427, L757 | `Opacity(0.5)` | `Visual { opacity: 0.5, ..default() }` 移行 |
| `examples/taffy_flex_demo_old.rs` | L13 | import | `Visual` import に変更 |
| `examples/taffy_flex_demo_old.rs` | L126, L175, L200 | `Opacity(0.5)` | `Visual { opacity: 0.5, ..default() }` 移行 |

#### テストコード（1ファイル・6テスト関数）

| ファイル | テスト関数 | 行 | `Opacity` 使用 |
|---------|-----------|------|--------------|
| `ecs/layout/hit_test.rs` | `test_hit_test_entity_bounds_alpha_boundary_above` | L1281-1289 | `Opacity(0.502)` |
| `ecs/layout/hit_test.rs` | `test_hit_test_entity_bounds_alpha_boundary_below` | L1310-1318 | `Opacity(0.501)` |
| `ecs/layout/hit_test.rs` | `test_hit_test_entity_bounds_low_opacity` | L1339-1347 | `Opacity(0.4)` |
| `ecs/layout/hit_test.rs` | `test_hit_test_entity_bounds_low_foreground_alpha` | L1368-1376 | `Opacity(1.0)` |
| `ecs/layout/hit_test.rs` | `test_hit_test_entity_bounds_no_opacity_no_brushes` | L1394-1396 | Opacity 無しケース |
| `ecs/layout/hit_test.rs` | `test_hit_test_entity_bounds_inherit_foreground` | L1416-1424 | `Opacity(0.502)` |

### 1.3 `Visual` コンポーネントの現行利用

- **挿入**: 全 Widget on_add フック（Label, Rectangle, Typewriter, BitmapSource, Window）が `Visual::default()` を自動挿入
- **変更操作**: プロダクションコードに `Mut<Visual>` / `&mut Visual` は**存在しない** — Widget 実装はカスタム opacity を設定する手段を持たない
- **読み取り**: `visual_manager.rs` L80 で `&Visual`（読み取りのみ）
- **validation**: `Visual` に `clamped()` / `validate()` メソッドは**存在しない**。`Opacity` にはこれらがある

### 1.4 ECS スケジュール構成

```
Layout → PostLayout → ... → Draw → ... → Composition → CommitComposition
                                          ^^^^^^^^^^^
                                    visual_property_sync_system
```

- `visual_property_sync_system` は `Composition` スケジュールに単独登録
- 明示的な `.after()` / `.before()` 順序制約なし
- 変更検出: `Changed<Arrangement> | Changed<GlobalArrangement> | Changed<Opacity>`

---

## 2. 要件フィジビリティ分析

### 要件 → 実装ギャップマップ

| 要件 | AC | ギャップ種別 | 詳細 |
|------|-----|------------|------|
| **Req 1** (opacity データフロー) | 1.1 | **Missing** | Widget → `Visual.opacity` 書き込みパスが存在しない（Widget on_add は `Visual::default()` のみ） |
| | 1.2 | **Missing** | `Visual` にクランプロジック（`clamped()`）が存在しない |
| | 1.3 | **既存で充足** | `Visual::default()` は既に `Opacity` 無しで動作する |
| | 1.4 | **既存で充足** | `Visual::default().opacity == 1.0` |
| | 1.5 | **既存で充足** | bevy_ecs `#[derive(Component)]` + `PartialEq` で `Changed<Visual>` は自動対応 |
| **Req 2** (is_visible データフロー) | 2.1 | **Missing** | Widget → `Visual.is_visible` 書き込みパスが存在しない |
| | 2.2 | **既存で充足** | `Visual.is_visible: bool` は既に定義済み |
| | 2.3 | **既存で充足** | `Visual::default().is_visible == true` |
| | 2.4 | **既存で充足** | `Changed<Visual>` で自動検出可能 |
| | 2.5 | **既存で充足** | 描画システムは `Visual.is_visible` を参照していない（スキップ判定なし = 継続） |
| **Req 3** (sync system 移行) | 3.1 | **Missing** | クエリに `Visual` が含まれていない |
| | 3.2 | **Missing** | 優先度ロジックが存在しない |
| | 3.3 | **Missing** | `Changed<Visual>` がフィルタにない |
| | 3.4 | **Missing** | `Changed<Opacity>` が残存 |
| | 3.5 | **Missing** | `is_visible` → `SetOpacity(0.0)` 変換ロジックが存在しない |
| **Req 4** (deprecation) | 4.1 | **Missing** | `#[deprecated]` 未付与 |
| | 4.2-4.5 | **方針のみ** | コード変更は将来フェーズ |
| **Req 5** (検証基準) | 5.1 | **部分充足** | `visual_component_test.rs`, `insert_visual_test.rs` が一部カバー |
| | 5.2 | **Missing** | sync system の integration test 不足 |

### 複雑度シグナル

| 観点 | 評価 |
|------|------|
| アルゴリズム複雑度 | 低（値のコピー・クランプ・条件分岐のみ） |
| 外部統合 | 低（DComp COM API の既存呼び出しパターン変更なし） |
| データモデル変更 | 低（`Visual` フィールドは既存、`Opacity` は温存） |
| クロスモジュール影響 | 低（hit_test.rs も Phase 0 で移行） |

---

## 3. 実装アプローチ候補

### Option A: Visual コンポーネント直接拡張（推奨）

**方針**: 既存 `Visual` にメソッド追加 + `visual_property_sync_system` のクエリ修正

#### 変更内容

1. **`Visual` にメソッド追加** (`ecs/graphics/components.rs`)
   - `pub fn set_opacity(&mut self, value: f32)` — 0.0〜1.0 クランプ付き
   - `pub fn clamped_opacity(&self) -> f32` — `Opacity.clamped()` 相当
   - `pub fn set_visible(&mut self, visible: bool)` — setter

2. **`visual_property_sync_system` 修正** (`ecs/graphics/systems.rs`)
   - クエリに `&Visual` を追加
   - `Option<&Opacity>` と `Changed<Opacity>` を完全削除（完全切断方式）
   - `Changed<Visual>` を変更検出フィルタに追加
   - Opacity 同期ロジックを `Visual.opacity` 読み取りに切り替え
   - `is_visible = false` → `SetOpacity(0.0)` ロジック追加

3. **Example 移行** (`examples/taffy_flex_demo.rs`, `taffy_flex_demo_old.rs`)
   - `Opacity(0.5)` → `Visual { opacity: 0.5, ..Default::default() }` に置換
   - ただし **Widget on_add が `Visual::default()` を自動挿入する問題** への対応要

4. **`Opacity` に `#[deprecated]` 付与** (`ecs/layout/metrics.rs`)

#### Widget on_add 競合問題 ★重要ギャップ★

**現状**: Widget（Rectangle, Label 等）の on_add フックが `Visual::default()` を自動挿入する。Example で `.spawn((Rectangle::new(), Visual { opacity: 0.5, .. })`) としても、`Rectangle::new()` の on_add が再度 `Visual::default()` を上書きする**可能性がある**。

**調査結果**: bevy_ecs 0.18 の `on_add` フック内で `commands.entity(entity).insert(Visual::default())` を使用している。bevy_ecs の仕様上、同一 `spawn()` バンドル内で既に `Visual` が含まれている場合、on_add 内の `insert` が**上書き**する。

**対策候補**:
- **(A-1)** on_add フック内で `Visual` 既存チェック: `if !commands.entity(entity).contains::<Visual>() { ... }`
  - ⚠ bevy_ecs 0.18 の `EntityCommands` に `contains` は存在しない可能性 → **Research Needed**
- **(A-2)** on_add フック内で `world.get::<Visual>(entity)` による既存チェック（on_add は `&mut DeferredWorld` を受け取る）
  - ✅ `DeferredWorld` は `get::<T>()` をサポートする → 有力候補
- **(A-3)** `Visual` の on_add フック側を変更し、Widget on_add からの `Visual::default()` 挿入を廃止
  - ✅ `Visual` 自身の `on_visual_add` が Arrangement 等を挿入しているため、Widget 側は `Visual` を挿入する必要がない
  - ⚠ 既に Widget on_add が `Visual::default()` を挿入している現状との互換性

**Trade-offs**:
- ✅ 最小限のファイル変更（components.rs, systems.rs, metrics.rs, examples 2ファイル）
- ✅ 既存 `Visual` 構造体を活用、新コンポーネント不要
- ✅ `Changed<Visual>` で opacity + is_visible + transform_origin を一括検出
- ❌ `Changed<Visual>` の粒度問題: `transform_origin` 変更でも opacity 同期が発火する
- ❌ Widget on_add 競合の解決が必要

### Option B: ブリッジシステム方式

**方針**: `Opacity` → `Visual.opacity` 同期用の新システムを追加し、段階的に移行

#### 変更内容

1. **新システム `opacity_bridge_system` 追加** (`ecs/graphics/systems.rs`)
   - `Changed<Opacity>` を監視し、`Visual.opacity` に同期
   - `Layout` → `Composition` の間（例: `PostLayout`）に配置
   
2. **`visual_property_sync_system` 修正**
   - `Visual.opacity` を読み取るように変更（Option A 同様）

3. **Example 変更なし**（移行期間中は `Opacity` → ブリッジ → `Visual.opacity` → sync system のチェーン）

4. **Phase 2 でブリッジ削除 + Example 移行を一括実施**

**Trade-offs**:
- ✅ Example / Widget コードの変更を先送りできる
- ✅ 移行期間中の互換性が完全に保たれる
- ❌ 一時的な間接層（ブリッジ）が追加され複雑度が増す
- ❌ `Opacity` → `Visual.opacity` の同期コストが毎フレーム発生
- ❌ `Changed<Visual>` が `opacity_bridge_system` の書き込みで常に発火する問題

### Option C: ハイブリッド（推奨ベース + Example 段階移行）

**方針**: Option A をベースに、Example + hit_test.rs 移行を Phase 0 内で実施

#### 変更内容

**Phase 0（本仕様スコープ）:**
1. `Visual` にメソッド追加（Option A 同様）
2. `visual_property_sync_system` を `Visual` ベースに移行（Option A 同様）
3. Widget on_add の競合解決（Option A の A-2 方式）
4. hit_test.rs の Opacity → `Visual.opacity` 読み取り移行 + テスト更新
5. Example 移行: `Opacity(0.5)` → `Visual { opacity: 0.5, .. }` 
6. `Opacity` に `#[deprecated]` 付与

**後続フェーズで実施:**
- `Opacity` コンポーネント完全削除

**Trade-offs**:
- ✅ Phase 0 でデータ不整合リスクを完全に解消
- ✅ hit_test.rs の Opacity 参照を Example と同一 Phase で移行、一貫性保証
- ✅ `#[deprecated]` により既存 `Opacity` 使用箇所が警告で可視化される
- ❌ hit_test.rs の Opacity 参照箇所が Phase 0 スコープに追加（小規模）

---

## 4. リスク & Research Needed

### Research Needed

| # | 項目 | 理由 | 対応フェーズ |
|---|------|------|------------|
| **R1** | bevy_ecs 0.18 `DeferredWorld` の `get::<T>()` API 可用性 | Widget on_add フック内で Visual 既存チェックに必要 | Design |
| **R2** | bevy_ecs 0.18 `on_add` フック内での既存コンポーネント上書き挙動 | 同一 spawn バンドル内の insert 順序保証 | Design |
| **R3** | `Changed<Visual>` の粒度問題 | `transform_origin` 変更時の不要な sync 発火コスト | Design |

### リスク分析

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| Widget on_add が `Visual::default()` でカスタム opacity を上書き | **高** | on_add 内既存チェック or 挿入ロジック見直し（R1, R2 で調査） |
| `Changed<Visual>` の過剰発火 | **低** | sync system 内で `opacity` / `is_visible` 値の差分チェック追加で対応可能 |
| hit_test のデータ不整合（移行期間中） | ~~**中**~~ → **解消** | Phase 0 で hit_test も Visual.opacity に移行することが決定 |
| Example 移行時の visual regression | **低** | `Opacity(0.5)` → `Visual { opacity: 0.5, .. }` は等価な値変更。既存 example を手動実行で確認 |

---

## 5. 工数 & リスク見積

| 項目 | 見積 | 根拠 |
|------|------|------|
| **工数** | **S（1〜3日）** | 既存パターンの拡張、コード変更量は小規模（components.rs メソッド追加、systems.rs クエリ修正、metrics.rs deprecated 付与、examples 2ファイル修正）。主な不確定要素は Widget on_add 競合の調査 |
| **リスク** | **中（Medium）** | bevy_ecs on_add フック挙動の調査が必要、hit_test 移行期間中のデータ整合性に注意要。ただし全体として既知パターンの適用であり、アーキテクチャ変更は不要 |

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ: **Option C（ハイブリッド）**

Phase 0 スコープを「sync system 移行 + Example 移行 + deprecation」に明確限定し、hit_test.rs 移行は Phase 2 に延期する。

### 設計フェーズで解決すべき事項

1. **Widget on_add 競合問題の解決方針決定**（R1, R2 の調査結果に基づく）
2. **Example 移行戦略**: `Opacity` 削除時に hit_test テストが壊れないか検証設計
3. **`Changed<Visual>` 粒度対策**: 値差分チェックの要否判断
4. **hit_test 移行期間中のデータ整合性**: `Opacity` を Phase 0 で deprecated にしつつ、hit_test は `Opacity` を参照し続ける矛盾の許容範囲

### 設計フェーズに持ち越す Research Items

- R1: `DeferredWorld::get::<T>()` の API 仕様と on_add フック内でのコンポーネント存在確認パターン
- R2: bevy_ecs 0.18 の spawn バンドル処理順序（on_add フック vs 明示的フィールド値の優先度）
- R3: `Changed<Visual>` フィルタの粒度最適化パターン
