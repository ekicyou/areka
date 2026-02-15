# 要件定義書: wintf-dcomp-migration-2-pipeline-switch

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 2「DComp パイプライン置換」を担当する。Phase 1 で構築した D2D1 合成スタック（WindowD3D11Compositor, composite_render_system, compositor_init_system）を world.rs の ECS Schedule に登録し、DComp パイプラインを無効化して新パイプラインに切り替える。

### 本子仕様のスコープ

- `ecs/world.rs`: DComp システムの登録解除 + 新システムの登録
- `ecs/graphics/core.rs`: GraphicsCore からの DComp 初期化除去
- `ecs/graphics/components.rs`: `on_visual_add` フックから DComp コンポーネント挿入を除去
- `ecs/graphics/systems.rs`: DComp システムの Schedule 登録解除
- DComp 依存 YELLOW システムの改修（invalidate_dependent_components, mark_dirty_surfaces）

### Non-Goals

- DComp コードの物理的削除（Phase 4 で実施）
- UpdateLayeredWindow 呼び出し（Phase 3 で実施）
- WS_EX_LAYERED ウィンドウスタイル変更（Phase 3 で実施）

---

## Requirements

### Requirement 1: ECS Schedule 切り替え

**Objective:** 開発者として、world.rs の描画パイプラインを DComp システムから D2D1 合成システムに切り替えたい。

_Parent: Req 2.3, 3.3_

#### Acceptance Criteria

1. The `world.rs` shall 以下の DComp システムを Schedule から**除去**する:
   - PreLayout: `visual_resource_management_system`, `visual_hierarchy_sync_system`
   - GraphicsSetup: `init_window_graphics`, `window_visual_integration_system`
   - Draw: `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed`
   - RenderSurface: `render_surface`
   - Composition: `visual_property_sync_system`

2. The `world.rs` shall 以下の新システムを Schedule に**登録**する:
   - GraphicsSetup: `compositor_init_system`
   - RenderSurface: `composite_render_system`

3. The Schedule 切り替え後, 全既存 example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）が新パイプラインで動作すること

### Requirement 2: GraphicsCore DComp 除去

**Objective:** 開発者として、GraphicsCore から DComp 初期化コードとフィールドを除去し、初期化フローを簡素化したい。

_Parent: Req 5.1, 5.2, 5.3, 5.4_

#### Acceptance Criteria

1. The `GraphicsCoreInner` shall `desktop: IDCompositionDesktopDevice` および `dcomp: IDCompositionDevice3` フィールドを削除する

2. The `GraphicsCore` shall `dcomp()` および `desktop()` アクセサメソッドを削除する

3. The `GraphicsCore::new()` shall `dcomp_create_desktop_device()` および `desktop.cast::<IDCompositionDevice3>()` の呼び出しを除去する

4. The `GraphicsCore` shall 以下のデバイスチェーンを維持する:
   - D3D11CreateDevice → ID3D11Device → IDXGIDevice4
   - D2D1CreateFactory → ID2D1Factory → ID2D1Device → ID2D1DeviceContext
   - DWriteCreateFactory → IDWriteFactory2

5. The `invalidate()` → 再初期化フロー shall DComp 再初期化ステップを省略した状態で正常動作する

### Requirement 3: on_visual_add フック更新

**Objective:** 開発者として、Visual コンポーネント追加時の自動コンポーネント挿入から DComp リソースを除去したい。

_Parent: Req 6.2, 6.3_

#### Acceptance Criteria

1. The `on_visual_add` フック shall `VisualGraphics::default()` の自動挿入を**除去**する

2. The `on_visual_add` フック shall `SurfaceGraphics::default()` の自動挿入を**除去**する

3. The `on_visual_add` フック shall `SurfaceGraphicsDirty::default()` の自動挿入を**除去**する

4. The `on_visual_add` フック shall `Arrangement::default()` と `BrushInherit` マーカーの挿入を**維持**する

### Requirement 4: YELLOW システム改修

**Objective:** 開発者として、DComp コンポーネント型への参照を持つ YELLOW システムを新コンポーネント型に追従させたい。

_Parent: Req 3.3_

#### Acceptance Criteria

1. The `invalidate_dependent_components` shall DComp コンポーネント（VisualGraphics, SurfaceGraphics）への参照を除去し、WindowD3D11Compositor への参照に更新する

2. The `mark_dirty_surfaces` shall per-entity SurfaceGraphicsDirty から per-window ダーティ判定に改修する（composite_render_system 内のダーティ判定と整合）

3. The `init_graphics_core` shall DComp デバイスの有効性チェックを除去する

### Requirement 5: DComp API 呼び出しゼロ検証

**Objective:** 開発者として、ECS パイプラインからの DComp API 呼び出しが完全にゼロであることを検証したい。

_Parent: Req 2.3, 10.1_

#### Acceptance Criteria

1. The Phase 2 完了後, `grep -r "IDComposition" ecs/` の結果がゼロ件であること（`com/dcomp.rs` 自体は残存許容）

2. The Phase 2 完了後, `cargo test` の全テストがパスすること

3. The Phase 2 完了後, `cargo build --examples` の全 example がビルドできること

4. The Phase 2 完了後, 全 example の実行で DComp API 呼び出しが発生しないこと

### Requirement 6: Phase 2 検証基準

**Objective:** 開発者として、Phase 2 の完了を客観的に判定できる検証基準が欲しい。

_Parent: Req 10.1, 10.2_

#### Acceptance Criteria

1. The 全既存 example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）が D2D1 合成パイプラインで正常動作すること

2. The `GraphicsCore` から DComp フィールド・メソッドが除去されていること

3. The ECS Schedule に DComp システムが登録されていないこと

4. The `cargo test` 全テストがパスすること

---

## 要件カバレッジサマリー

| 子仕様要件 | 親要件 | 概要 |
|-----------|--------|------|
| Req 1 | 2.3, 3.3 | ECS Schedule 切り替え |
| Req 2 | 5.1-5.4 | GraphicsCore DComp 除去 |
| Req 3 | 6.2, 6.3 | on_visual_add フック更新 |
| Req 4 | 3.3 | YELLOW システム改修 |
| Req 5 | 2.3, 10.1 | DComp API ゼロ検証 |
| Req 6 | 10.1, 10.2 | Phase 2 検証基準 |
