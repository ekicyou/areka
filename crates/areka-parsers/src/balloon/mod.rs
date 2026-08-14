//! balloon — バルーン記述（descript.txt 等の KV）→ 型付きバルーンモデルへの写像。
//!
//! 公開面集約層。内訳:
//! - `model` : バルーンモデル型（`BalloonModel`＋幾何・フォント sub-struct・I/O 契約の正本）
//! - `parse` : 公開 facade（2 層マージ＋KV→型写像＋符号保持＋整数パース）
//!
//! 依存方向は `model ← parse`。本 `mod.rs` は `model` のモデル型と
//! `parse`/`parse_str` facade を公開面へ `pub use` で集約する（`sakura/mod.rs`・`kv/mod.rs` 流儀）。

mod model;
mod parse;

#[cfg(test)]
mod model_tests;

#[cfg(test)]
mod parse_tests;

#[cfg(test)]
mod validation_tests;

pub use model::{
    BalloonCursor, BalloonModel, CursorColor, Font, FontColor, Origin, ValidRect, WindowPosition,
    WindowPositionRaw, WordWrapPoint,
};
pub use parse::{parse, parse_str};
