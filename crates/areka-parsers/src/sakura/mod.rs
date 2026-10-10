//! sakura — さくらスクリプトパーサ。
//!
//! 公開面の集約。中身は次のとおり:
//! - `model`  : 命令モデル型（下流 engine 共有 I/O 契約）
//! - `lexer`  : 構文層（手書き線形スキャナ）
//! - `decode` : 意味層（構文トークン → Instruction）
//! - `parse`  : 公開 facade（`parse` と、位置と印つきの `parse_noted`）
//!
//! 依存方向は `model ← lexer ← decode ← parse`。本 `mod.rs` は
//! `parse` / `Instruction` / 値型と、字句解析と同じ規則で環境変数を置き換える
//! `substitute_system_vars` を公開面へ `pub use` で集約する。

mod model;

#[cfg(test)]
mod model_tests;

mod lexer;

#[cfg(test)]
mod lexer_tests;

mod decode;

#[cfg(test)]
mod decode_tests;

#[cfg(test)]
mod decode_jump_tests;

#[cfg(test)]
mod decode_anchor_tests;

mod parse;

#[cfg(test)]
mod parse_tests;

#[cfg(test)]
mod validation_tests;

pub use lexer::substitute_system_vars;
pub use model::{
    Anchor, Choice, Instruction, JUMP_TAG_CARRIER, MoveArgs, NewLineRatio, Read, ReadNote,
    SurfaceArg,
};
pub use parse::{parse, parse_noted};
