//! emo2 fixture の pixel 観測 golden テスト（in-source `#[cfg(test)]`・`MemoryDecoder`+bake 経路）。
//!
//! 実上流（実行時エンジン）非依存に、fixture／正規化モデル直入力で合成結果を観測する。
//! 本モジュールは task 8.1（要件 **11.1**・**11.4**）を担う: emo2 fixture の surfaces.txt を
//! パースし、COM/WIC/表示に一切依存しない `MemoryDecoder`＋`bake` 経路で `AtlasTable` を構築、
//! `Composer::compose` でパイプライン全段（parse → fold → bake → bind → plan → blit）を駆動して
//! surface0（`element0,overlay,surface0.png,0,0` の単層 base surface）の合成結果が、`MemoryDecoder`
//! へ挿入した決定的な既知画像と**バイト等価**であることを検証する。
//!
//! ## なぜ単層 base surface が「挿入画像とバイト等価」になるか
//! surface0 は `element0` 一本のみを持つ base surface で、その element は原点 (0,0) の overlay。
//! 有効 bind 集合が空（surface0 は着せ替え bind を一切持たない）ゆえ、合成は「全透明キャンバスへ
//! element0 単層を (0,0) で SourceOver する」＝単なるコピーに帰着する。挿入画像を
//! **全不透明・透明マージン無し**にすることで α-bbox トリムが恒等（`trim_offset=(0,0)`・
//! `uv size == original`）となり、合成結果はキャンバス外形 == 画像外形の premultiplied BGRA
//! バッファそのものになる。よって `composed.bytes() == inserted_premultiplied_bytes`。
//!
//! ## 決定性
//! - 挿入画像は本テスト内で構成する固定パターンゆえ、実行環境に依らず不変。
//! - `bake`／トリム／packing は純粋な整数演算（`emo2_golden.rs` 参照）。
//! - COM/WIC/表示なし（`MemoryDecoder`+`bake` は CPU-only・要件 11.4）。

use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::Shell;

use crate::{BindSet, Composer, ComposedSurface, EmoWorld, PatternState};

// テーマ別サブモジュール（要件 1.7 のテーマ分割・設計判断 #1／#13 の接続規約）。
#[cfg(test)]
#[path = "golden_tests_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "golden_tests_surface0_base_tests.rs"]
mod surface0_base_tests;
#[cfg(test)]
#[path = "golden_tests_surface1000_bind_tests.rs"]
mod surface1000_bind_tests;
#[cfg(test)]
#[path = "golden_tests_trim_equivalence_tests.rs"]
mod trim_equivalence_tests;
#[cfg(test)]
#[path = "golden_tests_determinism_budget_tests.rs"]
mod determinism_budget_tests;
#[cfg(test)]
#[path = "golden_tests_blink_static_tests.rs"]
mod blink_static_tests;
#[cfg(test)]
#[path = "golden_tests_frame_extent_tests.rs"]
mod frame_extent_tests;
