# 実装計画: wintf-dcomp-migration-2-pipeline-switch

## タスク概要

Phase 2 — ECS Schedule 切り替え。Phase 1 で構築した D2D1 合成スタック（`compositor_init_system`, `composite_render_system`）を `world.rs` に登録し、DComp パイプラインの 10 システムを Schedule から除去する。旧実装（GraphicsCore DComp フィールド、旧関数本体、コンポーネント型定義）は Phase 4 まで保持し、Phase 2 では **Schedule 登録変更のみ** を実施する。

**重要**: Phase 2 単体では `UpdateLayeredWindow` 呼び出しがないため画面に何も表示されない。視覚的動作確認は Phase 3 完了まで不可能である。

---

## 実装タスク

- [ ] 1. (P) world.rs Schedule 切り替え
- [ ] 1.1 (P) PreLayout ステージ DComp システム除去
  - `visual_resource_management_system`, `visual_hierarchy_sync_system` を chain から除去する
  - `init_graphics_core` を単独 add_systems で再登録する（chain 不要）
  - _Requirements: 1.1_

- [ ] 1.2 (P) GraphicsSetup ステージ新旧置換
  - `init_window_graphics`, `window_visual_integration_system` の chain を除去する
  - `compositor_init_system` を単独登録する
  - _Requirements: 1.2_

- [ ] 1.3 (P) Draw ステージ末尾システム除去
  - chain 末尾の `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed` を除去する
  - chain は `generate_alpha_mask_system` まで維持する
  - _Requirements: 1.1_

- [ ] 1.4 (P) PreRenderSurface ステージ空化
  - `mark_dirty_surfaces` を除去する
  - ステージセクションをコメントアウトまたは除去する
  - _Requirements: 1.3, 3.4_

- [ ] 1.5 (P) RenderSurface ステージ空化
  - `render_surface` を除去する
  - ステージセクションをコメントアウトまたは除去する（WPF 的遅延戦略により焼き付けは Composition で実行）
  - _Requirements: 1.1, 5.3_

- [ ] 1.6 (P) Composition ステージ新旧置換
  - `visual_property_sync_system` を除去する
  - `composite_render_system` を登録する
  - _Requirements: 1.2_

- [ ] 1.7 (P) CommitComposition ステージ空化
  - `commit_composition` を除去する
  - ステージセクションをコメントアウトまたは除去する（Phase 3 `ulw_present_system` 用のハンドオーバーポイント）
  - _Requirements: 1.4_

- [ ] 2. (P) on_visual_add フック更新
- [ ] 2.1 (P) DComp コンポーネント挿入除去
  - `VisualGraphics::default()` 挿入 if ブロックを削除する
  - `SurfaceGraphics::default()` 挿入 if ブロックを削除する
  - `SurfaceGraphicsDirty::default()` 挿入 if ブロックを削除する
  - `Arrangement::default()` と `BrushInherit` 挿入は維持する
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 3. (P) invalidate_dependent_components 改修
- [ ] 3.1 (P) Query パラメータ更新
  - `Query<&mut WindowGraphics>` を除去する
  - `Query<&mut VisualGraphics>` を除去する
  - `Query<&mut SurfaceGraphics>` を除去する
  - `Query<&mut WindowD3D11Compositor>` を追加する
  - `Query<&mut BitmapSourceGraphics>` は維持する
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 3.2 (P) invalidate ループ更新
  - DComp コンポーネントの 3 ループ（`window_graphics_query`, `visual_query`, `surface_query`）を削除する
  - `WindowD3D11Compositor::invalidate()` ループを追加する
  - `BitmapSourceGraphics::invalidate()` ループは維持する
  - generation 比較ロジックは変更しない
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 4. 構造検証 + ビルド確認
- [ ] 4.1 Schedule 構造検査
  - `world.rs` に DComp 10 システムの add_systems 呼び出しが存在しないことを確認する（grep 検証）
  - `compositor_init_system`, `composite_render_system` が Schedule に登録されていることを確認する
  - RenderSurface ステージにシステム登録がないことを確認する
  - _Requirements: 4.1, 4.2, 5.1, 5.2, 5.3, 5.4_

- [ ] 4.2 コンパイル + テスト実行
  - `cargo test` 全テストパスを確認する（旧 DComp テストはコンパイル通過するが Schedule 非登録のため実行されない）
  - `cargo build --examples` 全 example ビルド成功を確認する
  - _Requirements: 5.5, 5.6_

---

## 並列実行可能性分析

**Task 1-3 はすべて並列実行可能**:
- Task 1（world.rs）、Task 2（components.rs）、Task 3（systems.rs）は異なるファイルを操作
- データ依存なし、ファイル競合なし
- 各タスクは独立してコンパイル・検証可能

**Task 4 は Task 1-3 完了後に実行**:
- Schedule 構造の完全性検証には全変更の完了が必要

```
Task 1.1-1.7 (world.rs) ────┐
Task 2.1 (components.rs) ────┼──→ Task 4.1-4.2 (検証)
Task 3.1-3.2 (systems.rs) ───┘
```

---

## 要件カバレッジサマリー

| 要件                               | タスク        |
| ---------------------------------- | ------------- |
| Req 1.1 (DComp 8システム除去)      | 1.1, 1.3, 1.5 |
| Req 1.2 (Phase 1 新システム登録)   | 1.2, 1.6      |
| Req 1.3 (mark_dirty_surfaces 除去) | 1.4           |
| Req 1.4 (commit_composition 除去)  | 1.7           |
| Req 2.1-2.5 (on_visual_add)        | 2.1           |
| Req 3.1-3.3 (invalidate改修)       | 3.1, 3.2      |
| Req 3.4 (mark_dirty Schedule除去)  | 1.4           |
| Req 4.1-4.2 (DComp参照除去検証)    | 4.1           |
| Req 5.1-5.6 (Phase 2完了検証)      | 4.1, 4.2      |

全5要件（15 AC）がタスクにマッピング済み。
