# Implementation Plan

- [x] 1. wintf graphics モジュール分割

- [x] 1.1 graphics/systems.rs を6サブモジュールに分割
  - `graphics/systems/` ディレクトリを作成し、初期化系（`init.rs`）・描画系（`render.rs`）・Surface管理（`surface.rs`）・Visualツリー同期（`visual_sync.rs`）・ウィンドウ位置管理（`window_pos.rs`）・Brush継承（`brushes.rs`）に機能単位で切り出す
  - `graphics/systems/mod.rs` で全公開シンボルを `pub use` により再export し既存パスの互換性を維持する
  - `cargo build` でシンボル未解決エラーがないことを確認する
  - _Requirements: 1.1, 1.4, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 1.2 compositor_systems.rs を2ファイルに分割
  - 初期化・リサイズ処理を `compositor_init.rs` に、描画パイプライン・画面転送処理を `compositor_render.rs` に切り出す
  - `graphics/mod.rs` のモジュール宣言を更新し `pub use` で再export を維持する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 2. (P) wintf ecs/window.rs をディレクトリモジュールに分割
  - `ecs/window/` ディレクトリを作成し、コンポーネント定義・hooks を `components.rs`、DPI型・変換メソッドを `dpi.rs`、`ZOrder`・`WindowStyle`・`WindowPos` builder を `window_pos.rs`、`SetWindowPosGuard`・`SetWindowPosCommand` を `command.rs` に切り出す
  - `window/mod.rs` で全公開シンボルを `pub use` により再export し `use crate::ecs::window::*` 等の既存パスとの互換性を維持する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 3. (P) wintf ecs/pointer/mod.rs を3サブファイルに分割
  - 基本型定義群（`PhysicalPoint`, `PointerState`, `PointerBuffer`, `ButtonBuffer`, `WheelBuffer` 等）を `pointer/types.rs` に切り出す
  - ECSシステム関数群（`process_pointer_buffers`, `clear_transient_*`, `debug_*`）を `pointer/systems.rs` に切り出す
  - thread_local バッファ定義・バッファ操作ヘルパー・`transfer_buffers_to_world` を `pointer/buffers.rs` に切り出す
  - `pointer/mod.rs` を `pub mod dispatch;` と `pub use` による再export のみに縮小する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 1.4, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 4. (P) wintf com/d2d/command.rs を2ファイルに分割
  - 全コマンド struct 定義と `DrawCommand` enum を `command_types.rs` に切り出す
  - `RecCommandSink` と全 `ID2D1CommandSink*_Impl` 実装を `command_sink.rs` に切り出す
  - `d2d/mod.rs` で `pub use` による再export を維持する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 5. wintf ecs/layout 分割

- [x] 5.1 (P) layout/systems.rs を4サブモジュールに分割
  - `systems/` ディレクトリを作成し、Arrangement伝播を `arrangement_systems.rs`、Taffyレイアウトパイプラインを `taffy_systems.rs`、WindowPos⇔Arrangement同期を `window_pos_systems.rs`、Monitor・LayoutRoot管理を `monitor_systems.rs` に切り出す
  - `systems/mod.rs` で全公開シンボルを `pub use` により再export する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 5.2 (P) hit_test.rs のインラインテストを外部ファイルに移動
  - `hit_test.rs` 末尾の `#[cfg(test)] mod tests { ... }` ブロック全体を `hit_test_tests.rs` として切り出す
  - `hit_test.rs` に `#[cfg(test)] #[path = "hit_test_tests.rs"] mod tests;` を追加して参照する
  - `cargo test` で全テストがパスすることを確認する
  - _Requirements: 1.1, 2.1, 2.2, 5.2, 5.3_

- [x] 5.3 (P) hit_region.rs のインラインテストを外部ファイルに移動
  - `hit_region.rs` 末尾の `#[cfg(test)] mod tests { ... }` ブロック全体を `hit_region_tests.rs` として切り出す
  - `hit_region.rs` に `#[cfg(test)] #[path = "hit_region_tests.rs"] mod tests;` を追加して参照する
  - `cargo test` で全テストがパスすることを確認する
  - _Requirements: 1.1, 2.1, 2.2, 5.2, 5.3_

- [x] 6. (P) wintf ecs/world.rs をディレクトリモジュールに分割
  - `world/` ディレクトリを作成し、`FrameCount` と12個のスケジュールラベル定義を `schedule_labels.rs` に、`IS_TICK_FLUSH_IN_PROGRESS`・`TickFlushGuard`・`VsyncTick` trait実装を `vsync.rs` に切り出す
  - `world/mod.rs` に `EcsWorld` 定義を残し `pub use` で再export する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 7. wintf ecs/window_proc 分割

- [x] 7.1 mouse_button.rs をクリック系とダブルクリック・ホイール系に分割
  - `handle_button_message` と8つの `WM_*BUTTON*` ラッパーを `mouse_click.rs` に切り出す
  - `handle_double_click_message`・4つの `WM_*DBLCLK` ラッパー・`WM_MOUSEWHEEL/MOUSEHWHEEL`・`find_ancestor_with_drag_config` を `mouse_dblclick_wheel.rs` に切り出す
  - `window_proc/mod.rs` のモジュール宣言を更新する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 7.2 window_proc/window_pos.rs からDPIヘルパーを分離
  - DPI計算純粋関数（`calculate_physical_size_from_box_style`・`calculate_center_correction`・`correct_position_for_dpi_center_preserve`）と付随ユニットテストを `dpi_helpers.rs` に切り出す
  - `window_proc/mod.rs` の宣言を更新する
  - `cargo test` で切り出されたテストがパスすることを確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2, 5.3_

- [x] 8. (P) wintf typewriter_systems.rs をレイアウトと描画に分割
  - レイアウト無効化・LayoutCache初期化・IR変換処理を `typewriter_layout.rs` に切り出す
  - フレーム状態更新・描画・空トーク背景描画を `typewriter_draw.rs` に切り出す
  - `text/mod.rs` のモジュール宣言を更新する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 9. (P) dola compile.rs をディレクトリモジュールに分割
  - `compile/` ディレクトリを作成し、公開データ型（`CompiledStoryboard`・`CompiledVariableTimeline`・`CompiledSegment`・`VariableTypeHint`・`CompiledTrigger`）を `compile/types.rs` に切り出す
  - 依存グラフ構築・トポロジカルソート・全解決ヘルパー関数を `compile/resolve.rs` に切り出す
  - `compile/mod.rs` に `compile_storyboard` メイン関数を残し `pub use types::*` で再export する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 10. (P) dola validate.rs をtrait定義とルール実装に分割
  - 全 `validate_*` 関数・`collect_keyframe_names_from_ref`・`dfs_detect_cycle` を `validate_rules.rs` に切り出す
  - `validate.rs` は `Validate` trait 定義と `impl Validate for DolaDocument` のオーケストレーションのみに縮小する
  - `cargo build` で動作確認する
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2_

- [x] 11. 300〜500行ソースファイルの推奨分割
  - dola・wintf の300〜500行ファイル16件（`high_level.rs`・`timeline_manager.rs`・`mouse_move.rs`・`facade.rs`・`instance_manager.rs`・`subscription_manager.rs`・`components.rs`・`bitmap_source/systems.rs`・`dispatch.rs`・`areka/main.rs`・`interpolator.rs`・`drag/dispatch.rs`・`tree_system.rs`・`win_style.rs`・`drag/state.rs`・`typewriter.rs`）を順次確認する
  - 明確な責務境界が存在するファイルは機能単位で分割し、分割後は300行以下を目標とする
  - 分割ポイントが不明瞭なファイルは現状維持とし、その判断をコミットメッセージに記録する
  - 各分割後に `cargo build` で動作確認する
  - _Requirements: 1.1, 3.1, 3.2, 3.3_

- [x] 12. テスト・サンプルファイル分割

- [x] 12.1 (P) examples/taffy_flex_demo.rs をディレクトリ例に変換
  - `examples/taffy_flex_demo/` ディレクトリを作成し `main.rs`（エントリポイント）・`setup.rs`（ECSセットアップ）・`widgets.rs`（ウィジェット定義）・`styles.rs`（スタイル設定）・`handlers.rs`（イベントハンドラ）等に分割する
  - `cargo run --example taffy_flex_demo` で正常動作することを確認する
  - _Requirements: 1.3, 4.1, 4.2, 4.3_

- [x] 12.2 (P) dola tests/compile_test.rs をカテゴリ別に分割
  - テスト関数を機能カテゴリ（基本コンパイル・タイミング解決・変数型等）別にファイル分割する
  - 共有ヘルパー関数を `tests/common/mod.rs` に抽出して重複を排除する
  - `cargo test` で全テストがパスすることを確認する
  - _Requirements: 1.2, 4.1, 4.2, 4.4_

- [x] 12.3 (P) dola tests/trigger_test.rs をカテゴリ別に分割
  - テスト関数をトリガー種別ごとにファイル分割し、共有ヘルパーを `tests/common/` に集約する
  - `cargo test` で全テストがパスすることを確認する
  - _Requirements: 1.2, 4.1, 4.2, 4.4_

- [x] 12.4 (P) dola tests/validation_test.rs をカテゴリ別に分割
  - テスト関数をバリデーションルール別にファイル分割し、共有ヘルパーを `tests/common/` に集約する
  - `cargo test` で全テストがパスすることを確認する
  - _Requirements: 1.2, 4.1, 4.2, 4.4_

- [x] 13. 最終検証とフォーマット適用
  - 全ファイル分割完了後に `cargo build` を実行し、全クレートのビルド成功を確認する
  - `cargo test` を実行し、全テストがパスすることを確認する（分割前後でテスト数が変化しないこと）
  - `cargo fmt --all` を実行してコードフォーマットを統一する
  - `cargo fmt --all` 適用後に再度 `cargo build` と `cargo test` を実行し最終確認する
  - _Requirements: 5.2, 5.3, 5.4, 6.1, 6.2, 6.3_
