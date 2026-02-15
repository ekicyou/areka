# 設計書: wintf-dcomp-migration-2-pipeline-switch

## Overview

Phase 1 で構築した D2D1 合成スタックを ECS Schedule に登録し、DComp パイプラインを無効化する。旧 DComp コードは物理的に残存するが（Phase 4 で削除）、Schedule からの登録は解除され、実行時の DComp API 呼び出しはゼロになる。

### Goals

- world.rs の Schedule を DComp → D2D1 合成に切り替え
- GraphicsCore から DComp 初期化・フィールドを除去
- on_visual_add フックから DComp コンポーネント挿入を除去
- YELLOW システムを新コンポーネント型に追従
- 全 example が新パイプラインで動作

### Non-Goals

- DComp コードの物理的削除（Phase 4）
- ULW 呼び出し（Phase 3）
- WS_EX_LAYERED 変更（Phase 3）

---

## Architecture

### 変更対象ファイル一覧

| ファイル | 変更種別 | 内容 |
|---------|---------|------|
| `ecs/world.rs` | 改修 | DComp システム解除 + 新システム登録 |
| `ecs/graphics/core.rs` | 改修 | DComp 初期化除去、フィールド・メソッド削除 |
| `ecs/graphics/components.rs` | 改修 | on_visual_add フック更新 |
| `ecs/graphics/systems.rs` | 改修 | invalidate_dependent_components, mark_dirty_surfaces 改修 |
| `ecs/graphics/mod.rs` | 改修 | compositor, compositor_systems のモジュール公開（Phase 1 で追加済み）|

### Schedule 変更差分

**除去するシステム（9個）**:
```
PreLayout:
  - visual_resource_management_system
  - visual_hierarchy_sync_system
GraphicsSetup:
  - init_window_graphics
  - window_visual_integration_system
Draw:
  - deferred_surface_creation_system
  - cleanup_surface_on_commandlist_removed
RenderSurface:
  - render_surface
Composition:
  - visual_property_sync_system
CommitComposition:
  - (commit_composition は Phase 3 まで温存 — ULW 実装まで暫定稼働)
```

**追加するシステム（2個）**:
```
GraphicsSetup:
  + compositor_init_system
RenderSurface:
  + composite_render_system
```

**注意**: `commit_composition` は Phase 3（ULW 統合）まで Schedule に残す。Phase 2 完了時点では DComp Commit() が呼ばれるが、DComp デバイスが除去済みのため実質 no-op もしくは削除。具体的な対応は実装時に判断する。

---

## Components 変更

### GraphicsCoreInner（改修）

**削除するフィールド**:
- `desktop: IDCompositionDesktopDevice`
- `dcomp: IDCompositionDevice3`

**削除するメソッド**:
- `pub fn dcomp(&self) -> &IDCompositionDevice3`
- `pub fn desktop(&self) -> &IDCompositionDesktopDevice`

**削除する初期化ステップ**（`GraphicsCore::new()` 内）:
- `dcomp_create_desktop_device(dxgi)` 呼び出し
- `desktop.cast::<IDCompositionDevice3>()` 呼び出し

**維持するフィールド**: d3d, dxgi, d2d_factory, d2d_device, d2d_dc, dwrite_factory — 全て変更なし

### on_visual_add フック（改修）

**削除する挿入**:
- `VisualGraphics::default()`
- `SurfaceGraphics::default()`
- `SurfaceGraphicsDirty::default()`

**維持する挿入**:
- `Arrangement::default()`
- `BrushInherit` マーカー

---

## Systems 変更

### invalidate_dependent_components（改修）

- DComp コンポーネント（VisualGraphics, SurfaceGraphics）への参照を除去
- WindowD3D11Compositor の invalidate トリガーを追加（デバイスロスト時）

### mark_dirty_surfaces（改修）

- `SurfaceGraphicsDirty` per-entity マーカーの使用を廃止
- composite_render_system のダーティ判定（Changed<GraphicsCommandList/GlobalArrangement/Visual>）に統合
- 本システム自体を簡素化するか、composite_render_system 内に統合する

### init_graphics_core（改修）

- DComp デバイスの有効性チェック（`dcomp().is_valid()` 等）を除去
- GraphicsCore の再初期化フローから DComp ステップを省略

---

## Requirements Traceability

| Requirement | Design Component | Verification |
|-------------|-----------------|-------------|
| Req 1.1-1.3 | world.rs Schedule 変更 | 全 example 動作確認 |
| Req 2.1-2.5 | GraphicsCoreInner 改修 | コンパイル + ユニットテスト |
| Req 3.1-3.4 | on_visual_add フック | コード検査 |
| Req 4.1-4.3 | systems.rs YELLOW 改修 | ユニットテスト + 統合テスト |
| Req 5.1-5.4 | 全変更の結果 | grep 検証 + cargo test + example 実行 |
| Req 6.1-6.4 | 全変更の結果 | E2E 検証 |

---

## Testing Strategy

### Unit Tests
- GraphicsCore 再初期化（DComp なし）テスト
- invalidate_dependent_components → WindowD3D11Compositor invalidate テスト

### Integration Tests
- world.rs Schedule 変更後の完全パイプライン動作テスト
- デバイスロスト → 新パイプラインでの自動復帰テスト

### E2E Tests
- `cargo run --example taffy_flex_demo` — 正常描画確認
- `cargo run --example typewriter_demo` — テキスト描画確認
- `cargo run --example multi_window_test` — マルチウィンドウ確認
- `cargo run --example split_image` — 画像描画確認

### Grep 検証
- `grep -r "IDComposition" crates/wintf/src/ecs/` → ゼロ件
- `grep -r "dcomp()" crates/wintf/src/ecs/` → ゼロ件
