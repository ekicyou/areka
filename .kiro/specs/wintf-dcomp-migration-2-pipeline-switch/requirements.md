# 要件定義書: wintf-dcomp-migration-2-pipeline-switch

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 2「DComp パイプライン置換」を担当する。Phase 1（`wintf-dcomp-migration-1-d2d1-composition`）で構築される D2D1 合成スタック（`WindowD3D11Compositor`, `composite_render_system`, `compositor_init_system`）を `world.rs` の ECS Schedule に登録し、既存 DComp パイプラインのシステム群を Schedule から除去して新パイプラインに切り替える。

### 前提条件

本子仕様は **Phase 1 の完了を前提** とする。Phase 1 が提供する以下の成果物が存在することを前提に要件を定義する:

- `ecs/graphics/compositor.rs`: `WindowD3D11Compositor` コンポーネント
- `ecs/graphics/compositor_systems.rs`: `compositor_init_system`, `composite_render_system`
- `com/ulw.rs`: `transfer_to_hbitmap` ユーティリティ
- `ecs/layout/arrangement.rs`: `GlobalArrangement.global_opacity` フィールド

### 本子仕様のスコープ

- `ecs/world.rs`: DComp システムの Schedule 登録解除 + Phase 1 新システムの登録
- `ecs/graphics/core.rs`: `GraphicsCoreInner` から DComp 初期化・フィールド・メソッドを除去
- `ecs/graphics/components.rs`: `on_visual_add` フックから DComp コンポーネント自動挿入を除去
- `ecs/graphics/systems.rs`: YELLOW システム（`invalidate_dependent_components`, `mark_dirty_surfaces`）を新コンポーネント型に適合
- `ecs/graphics/systems.rs`: `commit_composition` の DComp 依存を除去

### Non-Goals

- DComp コードファイルの物理的削除（Phase 4 で実施）
- `UpdateLayeredWindow` 呼び出し（Phase 3 で実施）
- `WS_EX_LAYERED` ウィンドウスタイル変更（Phase 3 で実施）
- Phase 1 新モジュール（`compositor.rs`, `compositor_systems.rs`）の新規実装

---

## Requirements

### Requirement 1: ECS Schedule 切り替え

**Objective:** 開発者として、`world.rs` の描画パイプラインを DComp システムから D2D1 合成システムに切り替え、全既存 example が新パイプラインで動作するようにしたい。

_Parent: Req 2.3, 3.3_

#### Acceptance Criteria

1. The `world.rs` shall 以下の DComp システムを各 Schedule ステージから除去する:
   - PreLayout: `visual_resource_management_system`, `visual_hierarchy_sync_system`
   - GraphicsSetup: `init_window_graphics`, `window_visual_integration_system`
   - Draw: `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed`
   - RenderSurface: `render_surface`
   - Composition: `visual_property_sync_system`

2. The `world.rs` shall Phase 1 で構築された以下の新システムを Schedule に登録する:
   - GraphicsSetup ステージ: `compositor_init_system`
   - RenderSurface ステージ: `composite_render_system`

3. The `world.rs` shall PreRenderSurface ステージの `mark_dirty_surfaces` を除去するか、`composite_render_system` 内のダーティ判定に置換する

4. When Schedule 切り替えが完了した時, the wintf crate shall 全既存 example（`taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image`）が D2D1 合成パイプラインで正常動作する

### Requirement 2: GraphicsCore DComp 除去

**Objective:** 開発者として、`GraphicsCore` から DComp 初期化コードとフィールドを除去し、D2D1 デバイス中心のシンプルな初期化フローにしたい。

_Parent: Req 5.1, 5.2, 5.3, 5.4_

#### Acceptance Criteria

1. The `GraphicsCoreInner` shall `desktop: IDCompositionDesktopDevice` フィールドおよび `dcomp: IDCompositionDevice3` フィールドを削除する

2. The `GraphicsCore` shall `dcomp()` アクセサメソッドおよび `desktop()` アクセサメソッドを削除する

3. The `GraphicsCore::new()` shall `dcomp_create_desktop_device()` 呼び出しおよび `desktop.cast::<IDCompositionDevice3>()` 呼び出しを除去する

4. The `GraphicsCore` shall 以下のデバイスチェーンを変更なく維持する:
   - `D3D11CreateDevice` → `ID3D11Device` → `IDXGIDevice4`
   - `D2D1CreateFactory` → `ID2D1Factory` → `ID2D1Device` → `ID2D1DeviceContext`
   - `DWriteCreateFactory` → `IDWriteFactory2`

5. When デバイスロストが発生した時, the `GraphicsCore` shall `invalidate()` → 再初期化フローを DComp 再初期化ステップなしで正常に完了する

### Requirement 3: on_visual_add フック更新

**Objective:** 開発者として、`Visual` コンポーネント追加時の自動コンポーネント挿入から DComp リソースコンポーネントを除去し、新パイプラインに不要な DComp コンポーネントの生成を停止したい。

_Parent: Req 6.2, 6.3_

#### Acceptance Criteria

1. The `on_visual_add` フック shall `VisualGraphics::default()` の自動挿入を除去する

2. The `on_visual_add` フック shall `SurfaceGraphics::default()` の自動挿入を除去する

3. The `on_visual_add` フック shall `SurfaceGraphicsDirty::default()` の自動挿入を除去する

4. The `on_visual_add` フック shall `Arrangement::default()` の挿入を維持する

5. The `on_visual_add` フック shall `BrushInherit` マーカーの挿入を維持する

### Requirement 4: YELLOW システム改修

**Objective:** 開発者として、DComp コンポーネント型への参照を持つ YELLOW 分類システムを、Phase 1 の新コンポーネント型（`WindowD3D11Compositor`）に追従させたい。

_Parent: Req 3.3_

#### Acceptance Criteria

1. The `invalidate_dependent_components` shall `VisualGraphics` および `SurfaceGraphics` への Query パラメータを除去する

2. The `invalidate_dependent_components` shall `WindowD3D11Compositor` への Query パラメータを追加し、デバイスロスト時に `WindowD3D11Compositor` を invalidate する

3. The `mark_dirty_surfaces` shall per-entity `SurfaceGraphicsDirty` ベースのダーティ検出を廃止する

4. The `mark_dirty_surfaces` の機能 shall `composite_render_system` 内の `Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>` によるダーティ判定に統合される（システム自体の除去または空実装化を含む）

5. The `init_graphics_core` shall DComp デバイスの有効性チェックを除去する（DComp フィールド参照が `GraphicsCore` から消滅するため）

### Requirement 5: commit_composition の DComp 依存除去

**Objective:** 開発者として、`commit_composition` システムの `IDCompositionDevice3::Commit()` 呼び出しを除去し、DComp API 依存をゼロにしたい。

_Parent: Req 2.3_

#### Acceptance Criteria

1. The `commit_composition` shall `IDCompositionDevice3::Commit()` の呼び出しを除去する

2. If Phase 3（ULW 統合）まで `commit_composition` の Schedule 登録を維持する場合, the `commit_composition` shall DComp API を一切呼び出さない no-op 実装とする

3. If `commit_composition` の責務が Phase 2 時点で不要と判断された場合, the `world.rs` shall 当該システムを CommitComposition ステージから除去する

### Requirement 6: ECS コードからの DComp 参照ゼロ検証

**Objective:** 開発者として、ECS パイプライン実装（`ecs/` ディレクトリ）からの DComp API 参照が完全にゼロであることを静的に検証したい。

_Parent: Req 2.3, 10.1_

#### Acceptance Criteria

1. When Phase 2 が完了した時, the `ecs/` ディレクトリ shall `IDComposition` 型への参照を含まない（`com/dcomp.rs` ファイル自体の残存は Phase 4 まで許容）

2. When Phase 2 が完了した時, the `ecs/` ディレクトリ shall `dcomp()` や `desktop()` メソッド呼び出しを含まない

3. The wintf crate shall `cargo test` の全テストがパスする

4. The wintf crate shall `cargo build --examples` で全 example がビルド成功する

### Requirement 7: Phase 2 完了検証基準

**Objective:** 開発者として、Phase 2 の完了を客観的に判定できる包括的な検証基準が欲しい。

_Parent: Req 10.1, 10.2_

#### Acceptance Criteria

1. The 全既存 example（`taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image`）shall D2D1 合成パイプラインで正常に描画される

2. The `GraphicsCoreInner` shall DComp 関連フィールド（`desktop`, `dcomp`）を含まない

3. The `GraphicsCore` shall DComp 関連アクセサメソッド（`dcomp()`, `desktop()`）を含まない

4. The `world.rs` の Schedule shall DComp システム（Req 1.1 の9システム）を含まない

5. The `world.rs` の Schedule shall Phase 1 新システム（`compositor_init_system`, `composite_render_system`）を含む

6. The `on_visual_add` フック shall DComp コンポーネント（`VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty`）の挿入を含まない

7. The `cargo test` shall 全テストがパスする

---

## 要件カバレッジサマリー

| 子仕様要件 | 親要件 | 概要 |
|-----------|--------|------|
| Req 1 | 2.3, 3.3 | ECS Schedule 切り替え（DComp 除去 + D2D1 登録） |
| Req 2 | 5.1–5.4 | GraphicsCore DComp フィールド・初期化除去 |
| Req 3 | 6.2, 6.3 | on_visual_add フックから DComp コンポーネント除去 |
| Req 4 | 3.3 | YELLOW システム（invalidate / mark_dirty / init）改修 |
| Req 5 | 2.3 | commit_composition の DComp 依存除去 |
| Req 6 | 2.3, 10.1 | ECS コードからの DComp 参照ゼロ静的検証 |
| Req 7 | 10.1, 10.2 | Phase 2 完了検証基準（E2E + 構造検証） |
