# Implementation Plan

## Task Format

- `(P)` — 並列実行可能なタスク（前タスクの完了を待たずに着手可能）
- `*` — 任意のテストカバレッジタスク（MVP 後に対処可能）

---

- [x] 1. dola テストをドメイン別サブディレクトリに再編する
- [x] 1.1 (P) compile ドメイン: エントリポイント生成とファイル移動
  - `tests/compile.rs` エントリポイントファイルを作成し `mod` 宣言のみを記述する
  - `compile_common/` の内容を `tests/compile/common/mod.rs` に移動する
  - `compile_error_test.rs` → `compile/error_test.rs`、`compile_integration_test.rs` → `compile/integration_test.rs` など6ファイルをドメインプレフィックス除去しつつ移動する
  - テストファイル内の `compile_common::` 参照を `super::common::` に更新する
  - _Requirements: 1.1, 1.2, 1.4_

- [x] 1.2 (P) runtime ドメイン: エントリポイント生成とファイル移動
  - `tests/runtime.rs` エントリポイントファイルを作成する
  - `runtime_core_types_test.rs` → `runtime/core_types_test.rs`、`runtime_facade_test.rs` → `runtime/facade_test.rs` など5ファイルをドメインプレフィックス除去しつつ移動する
  - _Requirements: 1.1, 1.4_

- [x] 1.3 (P) trigger ドメイン: エントリポイント生成とファイル移動
  - `tests/trigger.rs` エントリポイントファイルを作成する
  - `trigger_common/` の内容を `tests/trigger/common/mod.rs` に移動する
  - `trigger_compile_test.rs` → `trigger/compile_test.rs` など4ファイルをドメインプレフィックス除去しつつ移動する
  - テストファイル内の `trigger_common::` 参照を `super::common::` に更新する
  - _Requirements: 1.1, 1.2, 1.4_

- [x] 1.4 (P) validation ドメイン: エントリポイント生成・ファイル移動・共通ヘルパー統合
  - `tests/validation.rs` エントリポイントファイルを作成する
  - `validation_keyframe_test.rs` → `validation/keyframe_test.rs` など3ファイルをドメインプレフィックス除去しつつ移動する
  - 3ファイルに重複する `minimal_valid_doc()` を `validation/common/mod.rs` に統合する
  - _Requirements: 1.1, 1.2, 1.4_

- [x] 1.5 (P) general ドメイン: エントリポイント生成とファイル移動
  - `tests/general.rs` エントリポイントファイルを作成する（Rust 組み込み `core` クレートとの衝突回避のため `core` でなく `general` を使用）
  - `builder_test.rs`・`core_types_test.rs`・`integration_test.rs` を `general/` に移動する
  - _Requirements: 1.1, 1.4_

- [x] 1.6 dola テスト整理の動作確認
  - `cargo test -p dola` を実行し全テストがパスすることを確認する
  - _Requirements: 1.3_

---

- [x] 2. wintf テストをドメイン別サブディレクトリに再編する
- [x] 2.1 (P) layout ドメイン: エントリポイント生成とファイル移動
  - `tests/layout.rs` エントリポイントファイルを作成する
  - `layout_component_conversion_test.rs` → `layout/component_conversion_test.rs`、`layout_graphics_sync_test.rs` → `layout/graphics_sync_test.rs` の2件をプレフィックス除去して移動する
  - 残り10件（`arrangement_bounds_test.rs`、`client_area_positioning_test.rs`、`taffy_*` 群、`hierarchical_bounds_test.rs`、`boxstyle_coordinate_separation_test.rs`、`box_style_consolidation_test.rs`、`feedback_loop_convergence_test.rs`）をそのまま移動する
  - _Requirements: 2.1, 2.3_

- [x] 2.2 (P) graphics ドメイン: エントリポイント生成とファイル移動
  - `tests/graphics.rs` エントリポイントファイルを作成する
  - `graphics_core_test.rs` → `graphics/core_test.rs`、`graphics_core_ecs_test.rs` → `graphics/core_ecs_test.rs`、`graphics_reinit_unit_test.rs` → `graphics/reinit_unit_test.rs` の3件をプレフィックス除去して移動する
  - 残り7件（`dcomp_integration_test.rs`・`dcomp_resource_test.rs`・`compositor_*` 群・`surface_optimization_test.rs`）をそのまま移動する
  - _Requirements: 2.1, 2.3_

- [x] 2.3 (P) visual ドメイン: エントリポイント生成・ファイル移動・共通ヘルパー統合
  - `tests/visual.rs` エントリポイントファイルを作成する
  - `visual_child_order_test.rs` → `visual/child_order_test.rs` など5件（`visual_` プレフィックスを持つファイル）をプレフィックス除去して移動する
  - `parent_visual_test.rs`・`insert_visual_test.rs`・`remove_visual_api_test.rs`・`widget_visual_auto_insert_test.rs`・`transform_test.rs` の5件をそのまま移動する
  - 5ファイルに重複する `setup_graphics()` を `visual/common/mod.rs` に統合し、各ファイルのローカル定義を削除して `super::common::setup_graphics()` で参照する
  - _Requirements: 2.1, 2.3_

- [x] 2.4 (P) widget ドメイン: エントリポイント生成とファイル移動
  - `tests/widget.rs` エントリポイントファイルを作成する
  - `bitmap_source_integration_test.rs`・`vertical_text_layout_test.rs`・`entity_name_format_test.rs` の3件を移動する
  - _Requirements: 2.1, 2.3_

- [x] 2.5 (P) window ドメイン: エントリポイント生成とファイル移動
  - `tests/window.rs` エントリポイントファイルを作成する
  - `multiwindow_event_test.rs`・`monitor_hierarchy_test.rs`・`composition_mode_test.rs`・`find_owner_composition_mode_test.rs` の4件を移動する
  - _Requirements: 2.1, 2.3_

- [x] 2.6 (P) ecs ドメイン: エントリポイント生成とファイル移動
  - `tests/ecs.rs` エントリポイントファイルを作成する
  - `component_state_pattern_test.rs`・`lazy_reinit_pattern_test.rs`・`resource_removal_detection_test.rs` の3件を移動する
  - _Requirements: 2.1, 2.3_

- [x] 2.7 wintf テスト整理の動作確認
  - `cargo test -p wintf` を実行し全テストがパスすることを確認する
  - _Requirements: 2.2_

---

- [x] 3. テスト命名規約を structure.md に文書化する
  - 統合テストのファイル命名規約（`{feature}_{type}_test.rs`）・エントリポイント形式・共通ヘルパー配置・ドメインプレフィックス除去ルールを `structure.md` の `Naming Conventions` セクションに追記する
  - ユニットテストの命名規約（`tests.rs` / インラインモジュール）も併記する
  - _Requirements: 3.1, 3.2, 3.5_

---

- [x] 4. wintf ecs/ プロダクションファイルを適切なサブモジュールに移動する
- [x] 4.1 monitor.rs と window_system.rs を window/ サブモジュールに移動する
  - `ecs/monitor.rs` → `ecs/window/monitor.rs`、`ecs/window_system.rs` → `ecs/window/window_system.rs` に移動する
  - `ecs/window/mod.rs` に `pub mod monitor;` と `pub(crate) mod window_system;` の宣言を追加する
  - _Requirements: 4.1, 4.2_

- [x] 4.2 nchittest_cache.rs を pointer/ サブモジュールに移動する
  - `ecs/nchittest_cache.rs` → `ecs/pointer/nchittest_cache.rs` に移動する
  - `ecs/pointer/mod.rs` に `pub(crate) mod nchittest_cache;` の宣言を追加する
  - _Requirements: 4.1, 4.2_

- [x] 4.3 ecs/mod.rs の pub use チェーンを更新する
  - `pub mod monitor;` と `pub use monitor::*;` を削除し、`pub use window::monitor::*;` に置き換える
  - `mod nchittest_cache;` と `mod window_system;` を削除する
  - _Requirements: 4.2, 4.3_

- [x] 4.4 内部参照パスを更新する
  - `world/mod.rs` 内の `crate::ecs::window_system::` を `crate::ecs::window::window_system::` に更新する
  - `world/mod.rs` 内の `crate::ecs::nchittest_cache::` を `crate::ecs::pointer::nchittest_cache::` に更新する
  - `window_proc/mouse_move.rs` 内の `crate::ecs::nchittest_cache::` を `crate::ecs::pointer::nchittest_cache::` に更新する
  - `layout/systems/monitor_systems.rs` 内の `crate::ecs::monitor::` を `crate::ecs::window::monitor::` に更新する
  - _Requirements: 4.3_

- [x] 4.5 ecs/ サブモジュール構成の検証
  - ecs/ 配下のサブモジュール（common/drag/graphics/layout/pointer/transform/widget/window/window_proc/world）がドメインモデルと整合していることを確認する
  - widget/ 配下（bitmap_source/shapes/text/brushes.rs）の構成がウィジェット種別と一致していることを確認する
  - _Requirements: 4.1, 4.4_

---

- [x] 5. 全体統合検証を行う
- [x] 5.1 ワークスペース全体のテストパスを確認する
  - `cargo test --workspace` を実行し dola・wintf・areka すべてのテストがパスすることを確認する
  - _Requirements: 1.3, 2.2, 4.3_

- [x] 5.2 examples ビルドを確認する
  - `cargo build --examples` を実行し examples がコンパイルエラーなくビルドできることを確認する
  - _Requirements: 4.3_
