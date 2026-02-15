# 設計文書: wintf-dcomp-migration-4-dcomp-removal

## 1. 概要

Phase 4 は DComp コードの完全削除フェーズである。Phase 1-3 で DComp→ULW の完全移行が完了しており、DComp 関連コードは実行されない状態で残存している。本フェーズではこれらの死コードを体系的に削除し、コードベースをクリーンな状態に戻す。

### 前提条件
- Phase 3 完了済み: ULW 方式で全ウィンドウの描画が動作
- DComp API 呼び出しはゼロ（Phase 2 で Schedule から登録解除済み）
- 残存する DComp コードは dead code（コンパイルはされるが実行されない）

### 削除対象一覧

| ファイル | 種別 | 推定行数 | 内容 |
|---------|------|---------|------|
| `com/dcomp.rs` | ファイル全削除 | ~315行 | DComp COM ラッパー |
| `ecs/graphics/visual_manager.rs` | ファイル全削除 | ~170行 | DComp Visual リソース管理 |
| `examples/dcomp_demo.rs` | ファイル全削除 | 不明 | DComp 直接使用デモ |
| `ecs/graphics/components.rs` | 部分削除 | - | VisualGraphics, SurfaceGraphics 等 |
| `ecs/graphics/systems.rs` | 部分削除 | - | RED 分類の9システム関数 |
| `ecs/graphics/core.rs` | 部分削除 | - | DComp 関連 use/フィールド残留 |
| `ecs/graphics/mod.rs` | 部分削除 | - | mod 宣言 |
| `com/mod.rs` | 部分削除 | - | mod 宣言 |
| `crates/wintf/tests/*` | 修正/削除 | - | DComp 参照テスト |

---

## 2. 削除順序戦略

コンパイルエラーを最小化するため、**リーフ（参照される側）から削除する**のではなく、**参照する側から順に削除**する。

### 推奨削除順序

```
Step 1: examples/dcomp_demo.rs 削除（独立、外部依存なし）
Step 2: ecs/graphics/systems.rs の RED システム関数削除
Step 3: ecs/graphics/components.rs の DComp コンポーネント削除
Step 4: ecs/graphics/visual_manager.rs 全削除
Step 5: com/dcomp.rs 全削除
Step 6: mod 宣言・use 文のクリーンアップ
Step 7: テストファイルの修正
```

各ステップ後に `cargo check` でコンパイルエラーを確認し、エラーが出た箇所を即座に修正する。

---

## 3. 各ファイルの削除詳細

### 3.1 com/dcomp.rs（全削除）

DComp COM ラッパー関数群:
- `dcomp_create_desktop_device()`
- `IDCompositionDesktopDevice` / `IDCompositionDevice3` 関連ヘルパー
- `IDCompositionTarget` 作成関数
- `IDCompositionVisual3` 作成・操作関数
- `IDCompositionSurface` 作成・操作関数

### 3.2 ecs/graphics/visual_manager.rs（全削除）

DComp Visual リソースのライフサイクル管理:
- Visual 作成・破棄
- Surface 作成・破棄
- Visual ツリー構築（`InsertVisual`/`RemoveVisual`）

### 3.3 ecs/graphics/components.rs（部分削除）

削除対象コンポーネント:
- `VisualGraphics` — `IDCompositionVisual3` を保持
- `SurfaceGraphics` — `IDCompositionSurface` を保持
- `SurfaceGraphicsDirty` — Surface の dirty マーカー
- `SurfaceCreationStats` — Surface 作成統計

維持するコンポーネント（GREEN/新規）:
- `WindowD3D11Compositor`（Phase 1 で追加済み）
- `GraphicsCommandList`
- `HasGraphicsResources`
- `Arrangement`, `GlobalArrangement`（ECS レイアウト系）

### 3.4 ecs/graphics/systems.rs（部分削除）

削除対象（RED 分類 — Phase 2 で Schedule 登録解除済み）:
1. `visual_resource_management_system`
2. `visual_hierarchy_sync_system`
3. `init_window_graphics`（Phase 2 で置換済み）
4. `window_visual_integration_system`
5. `deferred_surface_creation_system`
6. `cleanup_surface_on_commandlist_removed`
7. `render_surface`
8. `visual_property_sync_system`
9. `commit_composition`

維持するシステム:
- `compositor_init_system`（Phase 1 で追加）
- `composite_render_system`（Phase 1 で追加）
- `ulw_present_system`（Phase 3 で追加）
- YELLOW 改修済みシステム（Phase 2 で改修済み）
- GREEN システム（ウィジェット描画系）

### 3.5 examples/dcomp_demo.rs（全削除）

ECS を使用しない独立 DComp デモ。DComp API を直接呼び出す唯一のサンプル。

### 3.6 テストファイルの修正

DComp コンポーネント型（`VisualGraphics`, `SurfaceGraphics` 等）を参照するテストを特定し、修正または削除する。主な修正対象:
- コンポーネント存在確認テスト
- on_visual_add テスト（Phase 2 で更新済みだが残留参照を確認）
- systems.rs のシステム関数テスト

---

## 4. 要件トレーサビリティ

| 子仕様要件 | 設計セクション |
|-----------|---------------|
| Req 1 (dcomp.rs削除) | §3.1 |
| Req 2 (コンポーネント削除) | §3.3 |
| Req 3 (システム関数削除) | §3.4 |
| Req 4 (visual_manager.rs削除) | §3.2 |
| Req 5 (dcomp_demo.rs削除) | §3.5 |
| Req 6 (use文クリーンアップ) | §2 Step 6 |
| Req 7 (テスト修正) | §3.6 |
| Req 8 (最終検証) | §5 テスト戦略 |

---

## 5. テスト戦略

### 5.1 各ステップの検証

| ステップ | 検証コマンド |
|---------|-------------|
| Step 1 (dcomp_demo.rs) | `cargo build --examples` |
| Step 2 (systems.rs) | `cargo check` |
| Step 3 (components.rs) | `cargo check` |
| Step 4 (visual_manager.rs) | `cargo check` |
| Step 5 (dcomp.rs) | `cargo check` |
| Step 6 (クリーンアップ) | `cargo check` |
| Step 7 (テスト修正) | `cargo test` |

### 5.2 最終検証

1. `grep -r "IDComposition" crates/wintf/src/` → ゼロ件
2. `grep -r "dcomp" crates/wintf/src/` → コード参照ゼロ（コメント許容）
3. `cargo test` → 全テストパス
4. `cargo build --examples` → 全 example ビルド成功
5. `cargo clippy` → 新規 warning ゼロ
6. `git diff --stat` で削除行数を確認

---

## 6. エラーハンドリング

Phase 4 はコード削除フェーズのため、ランタイムエラーハンドリングの新規設計は不要。

**コンパイルエラー対応**:
- 各ステップ後に `cargo check` を実行
- コンパイルエラーが出た場合は参照元を追跡して修正
- `unused import` warning は `cargo clippy` で一括検出・修正
