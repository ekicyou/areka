//! kv — デコード済み文字列を素朴な `key,value` フラットマップへ変換。
//!
//! 公開面の集約。中身は次のとおり:
//! - `parse` : 内部・行分割＋`split_once(',')`＋trim＋後勝ち（行の番号つきの
//!   `parse_kv_with_line` が本体・公開 facade `parse_kv` と `parse_kv_lowercased` はその射影）
//!
//! 単一責務ゆえ内部は `parse` 1 本（過剰分割回避）。本 `mod.rs` は
//! 公開面へ `pub use` で集約する。

mod parse;

#[cfg(test)]
mod parse_tests;

pub(crate) use parse::parse_kv_with_line;
pub use parse::{parse_kv, parse_kv_lowercased};
