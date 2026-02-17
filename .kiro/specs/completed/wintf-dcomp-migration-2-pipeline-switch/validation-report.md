# 実装バリデーションレポート: wintf-dcomp-migration-2-pipeline-switch

> **生成日時**: 2026-02-18  
> **フェーズ**: completed  
> **判定**: **GO** ✅

---

## 1. 検出対象

| 項目                     | 値                                        |
| ------------------------ | ----------------------------------------- |
| Feature                  | `wintf-dcomp-migration-2-pipeline-switch` |
| Parent Spec              | `wintf-dcomp-to-layered-migration`        |
| Phase                    | `completed`                               |
| Requirements approved    | ✅                                         |
| Design approved          | ✅                                         |
| Tasks approved           | ✅                                         |
| ready_for_implementation | ✅                                         |

---

## 2. タスク完了状況

全 14 サブタスク完了。

| Task                                                 | 状態 |
| ---------------------------------------------------- | ---- |
| 1.1 PreLayout ステージ DComp システム除去            | ✅    |
| 1.2 GraphicsSetup ステージ切り替え                   | ✅    |
| 1.3 Draw ステージ chain 再構成                       | ✅    |
| 1.4 PreRenderSurface ステージ空化                    | ✅    |
| 1.5 RenderSurface ステージ空化                       | ✅    |
| 1.6 Composition ステージ切り替え                     | ✅    |
| 1.7 CommitComposition ステージ空化                   | ✅    |
| 2.1 on_visual_add DComp コンポーネント除去           | ✅    |
| 3.1 invalidate_dependent_components DComp Query 除去 | ✅    |
| 3.2 WindowD3D11Compositor Query 追加                 | ✅    |
| 4.1 構造検証                                         | ✅    |
| 4.2 ビルド・テスト確認                               | ✅    |

---

## 3. 要件トレーサビリティ

### Requirement 1: ECS Schedule 切り替え (4 AC)

| AC                                        | 検証方法                                                                                 | 結果   |
| ----------------------------------------- | ---------------------------------------------------------------------------------------- | ------ |
| AC 1: DComp 8 システム除去                | `world.rs` grep — 旧 DComp システム名 0 マッチ                                           | ✅ PASS |
| AC 2: Phase 1 新システム登録              | `world.rs` grep — `compositor_init_system` (L369), `composite_render_system` (L414) 確認 | ✅ PASS |
| AC 3: `mark_dirty_surfaces` Schedule 除去 | `world.rs` grep — `mark_dirty_surfaces` 0 マッチ。PreRenderSurface はコメントのみ (L404) | ✅ PASS |
| AC 4: `commit_composition` Schedule 除去  | `world.rs` grep — `commit_composition` 0 マッチ。CommitComposition はコメントのみ        | ✅ PASS |

### Requirement 2: on_visual_add フック更新 (5 AC)

| AC                                           | 検証方法                                                                 | 結果   |
| -------------------------------------------- | ------------------------------------------------------------------------ | ------ |
| AC 1: `VisualGraphics::default()` 除去       | `components.rs` L264-296 コード検査 — 不在確認                           | ✅ PASS |
| AC 2: `SurfaceGraphics::default()` 除去      | `components.rs` L264-296 コード検査 — 不在確認                           | ✅ PASS |
| AC 3: `SurfaceGraphicsDirty::default()` 除去 | `components.rs` L264-296 コード検査 — 不在確認                           | ✅ PASS |
| AC 4: `Arrangement::default()` 維持          | `components.rs` L282 — `entity_cmds.insert(Arrangement::default())` 確認 | ✅ PASS |
| AC 5: `BrushInherit` 維持                    | `components.rs` L290 — `entity_cmds.insert(BrushInherit)` 確認           | ✅ PASS |

### Requirement 3: YELLOW システム改修 (4 AC)

| AC                                        | 検証方法                                                                                            | 結果   |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------- | ------ |
| AC 1: DComp Query 除去                    | `systems.rs` L796-815 コード検査 — `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` Query 不在 | ✅ PASS |
| AC 2: `WindowD3D11Compositor` Query 追加  | `systems.rs` L800 — `Query<&mut WindowD3D11Compositor>` + L809 `comp.invalidate()` 確認             | ✅ PASS |
| AC 3: `BitmapSourceGraphics` 維持         | `systems.rs` L801 — `Query<&mut BitmapSourceGraphics>` 確認                                         | ✅ PASS |
| AC 4: `mark_dirty_surfaces` Schedule 除去 | Req 1 AC 3 と連動 — `world.rs` から除去済み                                                         | ✅ PASS |

### Requirement 4: DComp 参照除去検証 (2 AC)

| AC                                   | 検証方法                                                                            | 結果   |
| ------------------------------------ | ----------------------------------------------------------------------------------- | ------ |
| AC 1: Schedule に DComp システム不在 | `world.rs` grep — 10 DComp システム名 0 マッチ                                      | ✅ PASS |
| AC 2: Phase 1 新システム在           | `world.rs` grep — `compositor_init_system` (L369), `composite_render_system` (L414) | ✅ PASS |

### Requirement 5: Phase 2 完了検証基準 (6 AC)

| AC                                              | 検証方法                                               | 結果   |
| ----------------------------------------------- | ------------------------------------------------------ | ------ |
| AC 1: DComp システム不在                        | Req 4 AC 1 と同一 — 確認済み                           | ✅ PASS |
| AC 2: Phase 1 新システム在                      | Req 4 AC 2 と同一 — 確認済み                           | ✅ PASS |
| AC 3: RenderSurface ステージ空                  | `world.rs` grep — `add_systems(RenderSurface` 0 マッチ | ✅ PASS |
| AC 4: on_visual_add に DComp コンポーネント不在 | Req 2 AC 1-3 と同一 — 確認済み                         | ✅ PASS |
| AC 5: `cargo test` 全パス                       | 全テストスイート 0 failures（500+ tests passed）       | ✅ PASS |
| AC 6: `cargo build --examples` 全ビルド成功     | 全 example ビルド成功（warnings のみ、errors なし）    | ✅ PASS |

---

## 4. テスト結果サマリー

```
cargo test — 全スイート結果:
  test result: ok. 70 passed; 0 failed    (wintf lib)
  test result: ok. 6 passed; 0 failed
  test result: ok. 7 passed; 0 failed
  test result: ok. 30 passed; 0 failed
  test result: ok. 15 passed; 0 failed
  test result: ok. 38 passed; 0 failed
  test result: ok. 17 passed; 0 failed
  test result: ok. 9 passed; 0 failed
  test result: ok. 35 passed; 0 failed
  test result: ok. 14 passed; 0 failed
  test result: ok. 23 passed; 0 failed
  test result: ok. 150 passed; 0 failed   (dola lib)
  test result: ok. 21 passed; 0 failed
  test result: ok. 7 passed; 0 failed
  test result: ok. 12 passed; 0 failed
  test result: ok. 12 passed; 0 failed
  test result: ok. 3 passed; 0 failed
  test result: ok. 6 passed; 0 failed
  test result: ok. 4 passed; 0 failed
  test result: ok. 8 passed; 0 failed
  test result: ok. 11 passed; 0 failed
  test result: ok. 3 passed; 0 failed
  test result: ok. 4 passed; 0 failed
  test result: ok. 8 passed; 0 failed
  test result: ok. 9 passed; 0 failed
  test result: ok. 5 passed; 0 failed
  test result: ok. 4 passed; 0 failed
  test result: ok. 8 passed; 0 failed     (doc-tests: 27 ignored)
  
  合計: 500+ passed, 0 failed
```

---

## 5. 修正されたテストファイル

実装に伴い、`on_visual_add` の DComp コンポーネント自動挿入除去により 3 テストファイルを更新:

| ファイル                                | 変更内容                                              |
| --------------------------------------- | ----------------------------------------------------- |
| `visual_component_test.rs`              | `VisualGraphics::default()` を明示的に spawn          |
| `visual_graphics_auto_creation_test.rs` | 4 テストで `VisualGraphics::default()` を明示的に追加 |
| `visual_hierarchy_sync_test.rs`         | 1 テストで `VisualGraphics::default()` を明示的に追加 |

これらのテストは DComp パイプライン（Schedule 非登録の旧システム関数）のユニットテストであり、Phase 4 までソースコードとして保持される。テスト自体が `VisualGraphics` の存在を前提とするため、`on_visual_add` の自動挿入除去に追従して明示的な挿入に変更した。

---

## 6. 実装時に解決した問題

| 問題                              | 解決策                                                             | ファイル                |
| --------------------------------- | ------------------------------------------------------------------ | ----------------------- |
| bevy_ecs B0001 Query 衝突         | `Added<WindowD3D11Compositor>` 別 Query → `Mut::is_added()` に変更 | `compositor_systems.rs` |
| テスト失敗（VisualGraphics 不在） | `on_visual_add` が自動挿入しなくなったため、テスト側で明示的に追加 | 3 テストファイル        |

---

## 7. 旧実装保持確認

要件の Non-Goals に従い、以下の旧実装は **変更なし・保持済み**:

- `GraphicsCore` の `dcomp`, `desktop` フィールド・アクセサ
- Schedule 非登録の旧システム関数（`init_window_graphics`, `commit_composition` 等）
- DComp コンポーネント型定義（`WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` struct）
- `com/dcomp.rs`, `ecs/graphics/visual_manager.rs` モジュール
- `systems.rs` 内の DComp 関連 import（旧システム関数が参照するため）

---

## 8. GO/NO-GO 判定

| 基準                               | 結果                                  |
| ---------------------------------- | ------------------------------------- |
| 全タスク完了                       | ✅ 14/14                               |
| 全 AC 充足                         | ✅ 21/21 (5 Requirements × 合計 21 AC) |
| `cargo test` 全パス                | ✅ 500+ passed, 0 failed               |
| `cargo build --examples` 全成功    | ✅                                     |
| 旧実装保持戦略準拠                 | ✅                                     |
| Phase 3 ハンドオーバーポイント準備 | ✅ CommitComposition 空化済み          |

### **判定: GO** ✅

Phase 2 の全要件を充足。Phase 3（`ulw_present_system` 統合 + `WS_EX_LAYERED` ウィンドウスタイル変更）への移行準備完了。
