//! Task 7.2-7.7: taffy advanced 追加ユニットテスト
//! TaffyComputedLayout変換、マッピング管理、階層同期、増分計算、クリーンアップ、境界値シナリオ
//!
//! 元の `taffy_advanced_test.rs` を凝集度ごとにサブモジュールへ分割（挙動非破壊）。

mod boundary_scenarios;
mod layout_conversion;
mod mapping_management;
mod tree_sync;
