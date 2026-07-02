//! kv — デコード済み文字列を素朴な `key,value` フラットマップへ変換。
//!
//! 公開面集約のスケルトン。後続タスクで以下を埋める:
//! - `parse` : 内部・行分割＋`split_once(',')`＋trim＋後勝ち（公開 facade `parse_kv`）
//!
//! 単一責務ゆえ内部は `parse` 1 本（過剰分割回避）。本 `mod.rs` は
//! `parse_kv` を公開面へ `pub use` で集約する。

mod parse;

#[cfg(test)]
mod parse_tests;

#[cfg(test)]
mod validation_tests;

pub use parse::parse_kv;
