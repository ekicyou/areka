//! sakura — さくらスクリプトパーサ。
//!
//! 公開面集約のスケルトン。後続タスクで以下を順次追加する:
//! - `model`  : 命令モデル型（下流 engine 共有 I/O 契約）
//! - `lexer`  : 構文層（手書き線形スキャナ）
//! - `decode` : 意味層（構文トークン → Instruction）
//! - `parse`  : 公開 facade（`pub fn parse(input: &str) -> Vec<Instruction>`）
//!
//! 依存方向は `model ← lexer ← decode ← parse`。本 `mod.rs` は
//! `parse` / `Instruction` / 値型を公開面へ `pub use` で集約する。
//!
//! 現時点では `model`（命令モデル型）のみを公開する。

mod model;

#[cfg(test)]
mod model_tests;

mod lexer;

#[cfg(test)]
mod lexer_tests;

mod decode;

#[cfg(test)]
mod decode_tests;

pub use model::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
