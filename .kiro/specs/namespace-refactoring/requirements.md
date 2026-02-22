# Requirements Document

## Introduction

全クレート（`areka`・`dola`・`wintf`）のモジュール構成を精査し、増加したテストファイルを適切なサブモジュールに整理するとともに、プロダクションコードの名前空間も一貫性のある構造にリファクタリングする。特に `dola/tests/`（21ファイル）と `wintf/tests/`（41ファイル）はフラットな名前空間にテストが散在しており、機能ドメイン別のサブディレクトリ整理が急務である。

## Project Description (Input)

全クレートおよびテストの名前空間整理。
特にテストにおいてファイル数が増えすぎている個所があり、適切な名前空間に仕分けるべき。その他、全体機能について一度適切な名前空間を検討し、名前空間リファクタリングを行う。

## Requirements

### Requirement 1: dola テストファイルのサブモジュール整理

**Objective:** 開発者として、`dola/tests/` 配下の21テストファイルを機能ドメイン別サブディレクトリに整理したい。テストファイルを探しやすくし、新規テスト追加時の配置判断を容易にするため。

#### 受入基準

1. The refactoring shall テスト群を以下のドメイン別サブディレクトリに分類する:
   - `tests/compile/` — コンパイル関連テスト（`compile_error_test`, `compile_integration_test`, `compile_metadata_test`, `compile_serde_test`, `compile_time_resolution_test`, `compile_transition_test`）
   - `tests/runtime/` — ランタイム関連テスト（`runtime_core_types_test`, `runtime_facade_test`, `conflict_resolution_test`, `loop_integration_test`, `loop_offset_test`）
   - `tests/trigger/` — トリガー関連テスト（`trigger_compile_test`, `trigger_runtime_test`, `trigger_serde_test`, `trigger_validation_test`）
   - `tests/validation/` — バリデーション関連テスト（`validation_keyframe_test`, `validation_schema_test`, `validation_transition_test`）
   - `tests/core/` — コア定義・横断結合テスト（`builder_test`, `core_types_test`, `integration_test`）
2. When テストファイルをサブディレクトリに移動する際, the refactoring shall 既存の共通モジュール（`compile_common/`, `trigger_common/`）を対応するサブディレクトリ内に配置する
3. The refactoring shall 移動後も `cargo test -p dola` が全テストパスすることを保証する
4. The refactoring shall 各サブディレクトリに `mod.rs` を配置せず、Cargo の integration test 規約（各 `.rs` ファイルが独立テストバイナリ）に従う。ただし共通ヘルパーモジュールは `tests/<domain>/common/mod.rs` として配置可能とする

### Requirement 2: wintf テストファイルのサブモジュール整理

**Objective:** 開発者として、`wintf/tests/` 配下の41テストファイルを機能ドメイン別サブディレクトリに整理したい。テスト対象コンポーネントとの対応を明確にし、保守性を向上させるため。

#### 受入基準

1. The refactoring shall テスト群を以下のドメイン別サブディレクトリに分類する:
   - `tests/layout/` — レイアウト関連テスト（`arrangement_bounds_test`, `client_area_positioning_test`, `layout_component_conversion_test`, `layout_graphics_sync_test`, `taffy_*_test` 群, `hierarchical_bounds_test`, `boxstyle_coordinate_separation_test`, `box_style_consolidation_test`, `feedback_loop_convergence_test`）
   - `tests/graphics/` — グラフィックス関連テスト（`graphics_core_test`, `graphics_core_ecs_test`, `graphics_reinit_unit_test`, `dcomp_*_test` 群, `compositor_*_test` 群, `surface_optimization_test`）
   - `tests/visual/` — ビジュアルツリー関連テスト（`visual_*_test` 群, `parent_visual_test`, `insert_visual_test`, `remove_visual_api_test`, `widget_visual_auto_insert_test`, `transform_test`）
   - `tests/widget/` — ウィジェット関連テスト（`bitmap_source_integration_test`, `vertical_text_layout_test`, `entity_name_format_test`）
   - `tests/window/` — ウィンドウ関連テスト（`multiwindow_event_test`, `monitor_hierarchy_test`, `composition_mode_test`, `find_owner_composition_mode_test`）
   - `tests/ecs/` — ECS パターンテスト（`component_state_pattern_test`, `lazy_reinit_pattern_test`, `resource_removal_detection_test`）
2. The refactoring shall 移動後も `cargo test -p wintf` が全テストパスすることを保証する
3. If テストファイルが複数ドメインにまたがる場合, the refactoring shall 最も主要な関心ドメインのサブディレクトリに配置する

### Requirement 3: テスト命名規約の統一

**Objective:** 開発者として、全クレートのテストファイル命名規約を統一したい。新規テスト追加時の命名判断を容易にするため。

#### 受入基準

1. The refactoring shall 統合テストファイルの命名規約を `{対象機能}_{テスト種別}_test.rs` に統一する
2. The refactoring shall ユニットテスト（`#[path]` パターン使用）の命名規約を `{モジュール名}_tests.rs` に統一する
3. The refactoring shall `wintf` 内の `#[path]` テスト（`dispatch_tests.rs`, `hit_region_tests.rs`, `hit_test_tests.rs`, `hit_test_ex_tests.rs`, `graphics_tests.rs`）および `dola/src/runtime/` 内の `#[path]` テスト（4件）の命名一貫性を確認・修正する
4. If 既存テストファイル名が規約に従っていない場合, the refactoring shall 規約に準拠するようリネームする
5. The refactoring shall 命名規約を `structure.md` ステアリングドキュメントに追記する

### Requirement 4: wintf プロダクションコードのモジュール構造検証

**Objective:** 開発者として、`wintf` クレートの `ecs/` 配下モジュール構造が適切かを検証したい。現在のサブモジュール分割がドメインモデルおよびsteering（`structure.md`）の指針と整合しているか確認するため。

#### 受入基準

1. The refactoring shall `ecs/` 配下の各サブモジュール（`common/`, `drag/`, `graphics/`, `layout/`, `pointer/`, `transform/`, `widget/`, `window/`, `window_proc/`, `world/`）およびルート直下ファイル（`app.rs`, `monitor.rs`, `nchittest_cache.rs`, `window_system.rs`）の配置妥当性を検証する
2. If `ecs/` ルート直下にサブモジュールに属すべきファイルが存在する場合, the refactoring shall 適切なサブモジュールへの移動を実施する（例: `window_system.rs` → `window/`, `monitor.rs` → `window/`）
3. When 内部モジュールパスが変更された場合, the refactoring shall `pub use` で旧パスからのアクセスを維持するか、全参照箇所（`areka/src/main.rs`, `examples/` 等）を更新し、コンパイル・実行可能な状態を保証する
4. The refactoring shall `widget/` 配下のサブモジュール（`bitmap_source/`, `shapes/`, `text/`）の構成がウィジェット種別と一致していることを確認する

> **参考（dola プロダクションコードについて）**: dola のモジュール構造は検証済み。ルート直下9ファイルは `pub use` フラットエクスポートの設計意図がありサブモジュール化しない。`runtime/`（12ファイル）は `pub(crate)` の内部結合が多く分割リスクが工数に見合わないため現状維持（開発者確認済み）。
