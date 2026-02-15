# 実装計画: wintf-dcomp-migration-4-dcomp-removal

## タスク概要

Phase 4 — DComp コード削除・クリーンアップ。Phase 1-3 で完全移行済みの DComp 関連コードを体系的に削除する。各ステップで `cargo check` を実行し、コンパイルエラーを即座に修正する。

---

## 実装タスク

### Phase 4A: 独立ファイル削除

- [ ] 1. dcomp_demo.rs 削除
  - `examples/dcomp_demo.rs` を削除する
  - `Cargo.toml` の `[[example]]` セクションから `dcomp_demo` エントリを除去する（存在する場合）
  - `cargo build --examples` でビルド成功を確認する
  - _Requirements: 5.1, 5.2, 5.3_
  - _Dependencies: Phase 3 完了_

### Phase 4B: ECS コード削除

- [ ] 2. RED システム関数のコード削除
  - `ecs/graphics/systems.rs` から以下の9関数の実装コードを削除する：
    - `visual_resource_management_system`, `visual_hierarchy_sync_system`, `init_window_graphics`（旧版）, `window_visual_integration_system`, `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed`, `render_surface`, `visual_property_sync_system`, `commit_composition`
  - 関連するヘルパー関数・プライベート関数も合わせて削除する
  - `cargo check` でコンパイルを確認する
  - _Requirements: 3.1, 3.2, 3.3_
  - _Dependencies: Task 1_

- [ ] 3. DComp コンポーネント定義の削除
  - `ecs/graphics/components.rs` から以下の struct を削除する：
    - `VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty`, `SurfaceCreationStats`
  - 関連する `impl` ブロック、`derive` マクロ、トレイト実装も合わせて削除する
  - `cargo check` でコンパイルを確認し、参照エラーを修正する
  - _Requirements: 2.1, 2.2_
  - _Dependencies: Task 2_

- [ ] 4. visual_manager.rs ファイル全削除
  - `ecs/graphics/visual_manager.rs` ファイルを削除する
  - `ecs/graphics/mod.rs` から `mod visual_manager` 宣言を除去する
  - `cargo check` でコンパイルを確認する
  - _Requirements: 4.1, 4.2, 4.3_
  - _Dependencies: Task 3_

### Phase 4C: COM レイヤー削除

- [ ] 5. com/dcomp.rs ファイル全削除
  - `com/dcomp.rs` ファイル（約315行）を削除する
  - `com/mod.rs` から `mod dcomp` 宣言を除去する
  - `ecs/graphics/core.rs` 等から `com::dcomp::` への use 文を除去する
  - `cargo check` でコンパイルを確認する
  - _Requirements: 1.1, 1.2, 1.3_
  - _Dependencies: Task 4_

### Phase 4D: クリーンアップ・検証

- [ ] 6. use 文・参照の最終クリーンアップ
  - `grep -r "IDComposition" crates/wintf/src/` でゼロ件を確認する
  - `grep -r "dcomp" crates/wintf/src/` でコード参照ゼロを確認する（コメント許容）
  - `cargo clippy` で `unused_imports` 等の warning をゼロにする
  - 残存する DComp 関連の型エイリアスやコメントを整理する
  - _Requirements: 6.1, 6.2, 6.3_
  - _Dependencies: Task 5_

- [ ] 7. テストファイルの修正
  - DComp コンポーネント・システムを参照するテストを特定する
  - 不要なテストを削除、修正可能なテストを更新する
  - `cargo test` 全テストパスを確認する
  - _Requirements: 7.1, 7.2, 7.3_
  - _Dependencies: Task 6_

- [ ] 8. Phase 4 最終検証
  - `grep -r "IDComposition" crates/wintf/src/` → ゼロ件
  - `grep -r "dcomp" crates/wintf/src/` → コード参照ゼロ
  - `cargo test` → 全テストパス
  - `cargo build --examples` → 全 example ビルド成功（dcomp_demo 除外済み）
  - `cargo clippy` → 新規 warning ゼロ
  - `git diff --stat` で削除行数を記録する
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_
  - _Dependencies: Task 7_

---

## 依存関係サマリー

```
Task 1 (dcomp_demo) ──→ Task 2 (systems) ──→ Task 3 (components) ──→ Task 4 (visual_manager)
                                                                          ↓
                                                                     Task 5 (dcomp.rs) ──→ Task 6 (クリーンアップ) ──→ Task 7 (テスト) ──→ Task 8 (最終検証)
```

## 要件カバレッジサマリー

| 要件 | タスク |
|------|--------|
| Req 1 (dcomp.rs削除) | 5 |
| Req 2 (コンポーネント削除) | 3 |
| Req 3 (システム関数削除) | 2 |
| Req 4 (visual_manager削除) | 4 |
| Req 5 (dcomp_demo削除) | 1 |
| Req 6 (use文クリーンアップ) | 6 |
| Req 7 (テスト修正) | 7 |
| Req 8 (最終検証) | 8 |

全8要件がタスクにマッピング済み。
