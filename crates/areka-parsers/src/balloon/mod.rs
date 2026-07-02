//! balloon — バルーン記述（descript.txt 等の KV）→ 型付きバルーンモデルへの写像。
//!
//! 公開面集約のスケルトン。後続タスクで以下を順次追加する:
//! - `model` : バルーンモデル型（`BalloonModel`＋幾何・フォント sub-struct・I/O 契約の正本）
//! - `parse` : 公開 facade（2 層マージ＋KV→型写像＋符号保持＋整数パース）
//!
//! 依存方向は `model ← parse`。本 `mod.rs` は後続タスクで
//! `parse` / `BalloonModel` / 値型を公開面へ `pub use` で集約する
//! （`sakura/mod.rs`・`kv/mod.rs` 流儀）。
//! 型・facade は 2.1/2.2 で実体化するため、現段階では宣言のみを置く。

mod model;
mod parse;
