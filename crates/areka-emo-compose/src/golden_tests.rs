//! emo2 fixture の計画の golden テストの親（in-source `#[cfg(test)]`・`MemoryDecoder`+bake 経路）。
//!
//! 実上流（実行時エンジン）非依存に、fixture／正規化モデル直入力で計画を観測する。emo2 fixture の
//! surfaces.txt をパースし、COM/WIC/表示に一切依存しない `MemoryDecoder`＋`bake` 経路で `AtlasTable` を
//! 構築し、`Composer::plan` でパイプライン全段（parse → fold → bake → bind → plan）を駆動する。
//! テストは下のテーマ別の子モジュールに置き、共通の口は `test_support`（`golden_tests_test_support.rs`）。
//!
//! ## 決定性
//! - 挿入画像はテストの中で組む固定パターンゆえ、実行環境に依らず不変。
//! - `bake`／トリム／packing は純粋な整数演算（`areka-emo-atlas` の `src/emo2_golden.rs` 参照）。
//! - COM/WIC/表示なし（`MemoryDecoder`+`bake` は CPU-only）。

use std::path::{Path, PathBuf};

use areka_emo_atlas::{AtlasTable, MemoryDecoder, SetId};
use areka_parsers::shell::Shell;

use crate::{BindSet, Composer, DrawPlan, EmoWorld, PatternState};

// テーマ別サブモジュール。
#[cfg(test)]
#[path = "golden_tests_blink_static_tests.rs"]
mod blink_static_tests;
#[cfg(test)]
#[path = "golden_tests_determinism_budget_tests.rs"]
mod determinism_budget_tests;
#[cfg(test)]
#[path = "golden_tests_frame_extent_tests.rs"]
mod frame_extent_tests;
#[cfg(test)]
#[path = "golden_tests_plan_order_tests.rs"]
mod plan_order_tests;
#[cfg(test)]
#[path = "golden_tests_surface1000_bind_tests.rs"]
mod surface1000_bind_tests;
#[cfg(test)]
#[path = "golden_tests_test_support.rs"]
mod test_support;
