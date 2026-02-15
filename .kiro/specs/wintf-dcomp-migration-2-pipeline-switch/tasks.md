# 実装計画: wintf-dcomp-migration-2-pipeline-switch

## タスク概要

Phase 2 — DComp パイプライン置換。Phase 1 で構築した D2D1 合成スタックを world.rs の ECS Schedule に登録し、DComp パイプラインを無効化する。旧コードは残存するが実行されない状態にする。

---

## 実装タスク

### Phase 2A: GraphicsCore 改修

- [ ] 1. GraphicsCore から DComp 除去
  - `GraphicsCoreInner` から `desktop: IDCompositionDesktopDevice`, `dcomp: IDCompositionDevice3` フィールドを削除する
  - `dcomp()`, `desktop()` アクセサメソッドを削除する
  - `GraphicsCore::new()` から `dcomp_create_desktop_device()` と `desktop.cast()` 呼び出しを除去する
  - `invalidate()` / 再初期化フローから DComp ステップを除去する
  - `use` 文から `com/dcomp.rs` への参照を除去する
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

### Phase 2B: ECS コンポーネント・システム改修

- [ ] 2. on_visual_add フック更新
  - `on_visual_add` から `VisualGraphics::default()`, `SurfaceGraphics::default()`, `SurfaceGraphicsDirty::default()` の挿入を除去する
  - `Arrangement::default()` と `BrushInherit` の挿入は維持する
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Dependencies: None_

- [ ] 3. YELLOW システム改修
  - `invalidate_dependent_components` を更新: DComp コンポーネント参照 → WindowD3D11Compositor 参照
  - `mark_dirty_surfaces` を改修: per-entity SurfaceGraphicsDirty → composite_render_system 内統合（または簡素化）
  - `init_graphics_core` から DComp デバイス有効性チェックを除去する
  - _Requirements: 4.1, 4.2, 4.3_
  - _Dependencies: Task 1_

### Phase 2C: Schedule 切り替え

- [ ] 4. world.rs Schedule 更新
  - PreLayout から `visual_resource_management_system`, `visual_hierarchy_sync_system` を除去する
  - GraphicsSetup の `init_window_graphics`, `window_visual_integration_system` を `compositor_init_system` に置換する
  - Draw から `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed` を除去する
  - RenderSurface の `render_surface` を `composite_render_system` に置換する
  - Composition の `visual_property_sync_system` を除去する
  - CommitComposition の `commit_composition` の扱いを判断する（Phase 3 まで温存 or ここで除去）
  - _Requirements: 1.1, 1.2, 1.3_
  - _Dependencies: Task 1, Task 2, Task 3_

### Phase 2D: 検証

- [ ] 5. DComp API ゼロ検証 + 全 example 動作確認
  - `grep -r "IDComposition" crates/wintf/src/ecs/` がゼロ件であることを確認する
  - `cargo test` 全テストパスを確認する
  - `cargo build --examples` 全 example ビルドを確認する
  - `cargo run --example taffy_flex_demo` 正常動作を確認する
  - `cargo run --example typewriter_demo` 正常動作を確認する
  - `cargo run --example multi_window_test` 正常動作を確認する
  - `cargo run --example split_image` 正常動作を確認する
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4_
  - _Dependencies: Task 4_

---

## 依存関係サマリー

```
Task 1 (GraphicsCore) ──┐
Task 2 (on_visual_add) ──┼──→ Task 4 (world.rs) ──→ Task 5 (検証)
Task 3 (YELLOW改修) ─────┘
```

## 要件カバレッジサマリー

| 要件 | タスク |
|------|--------|
| Req 1 (Schedule切替) | 4 |
| Req 2 (GraphicsCore) | 1 |
| Req 3 (on_visual_add) | 2 |
| Req 4 (YELLOWシステム) | 3 |
| Req 5 (ゼロ検証) | 5 |
| Req 6 (Phase 2検証) | 5 |

全6要件がタスクにマッピング済み。
