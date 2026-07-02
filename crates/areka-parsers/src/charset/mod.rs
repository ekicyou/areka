//! charset — バイト列の文字コード検出・全体デコード。
//!
//! 公開面集約のスケルトン。後続タスクで以下を順次埋める:
//! - `model`   : 既定エンコード指定型 `DefaultEncoding`（契約の片側・本 spec 所有）
//! - `prescan` : 内部・冒頭 ASCII プリスキャンで charset 名を抽出（D1/D2）
//! - `decode`  : 内部・prescan 結果＋encoding_rs で全体デコード（公開 facade `decode`）
//!
//! 依存方向は `model ← prescan ← decode`。本 `mod.rs` は
//! `decode` / `DefaultEncoding` を公開面へ `pub use` で集約する。

mod model;

#[cfg(test)]
mod model_tests;

mod prescan;

#[cfg(test)]
mod prescan_tests;

mod decode;

#[cfg(test)]
mod decode_tests;

#[cfg(test)]
mod validation_tests;

pub use decode::decode;
pub use model::DefaultEncoding;
