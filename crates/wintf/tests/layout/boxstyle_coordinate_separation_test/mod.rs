//! BoxStyle座標分離テスト
//!
//! boxstyle-coordinate-separation仕様の統合テスト:
//! - Task 7.1: BoxStyle.inset 不変性
//! - Task 7.2: Changed<BoxStyle> 発火タイミング
//! - Task 7.3: ドラッグ終了同期
//! - Task 7.4: WindowDragging ライフサイクル
//! - Task 7.5: update_arrangements Window offset スキップ
//!
//! 元の `boxstyle_coordinate_separation_test.rs` を凝集度ごとにサブモジュールへ分割（挙動非破壊）。

mod boxstyle_inset;
mod changed_timing;
mod drag_lifecycle;
mod window_sync;
