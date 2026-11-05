//! charset — バイト列の文字コード検出・全体デコード。
//!
//! 公開面の集約。中身は次のとおり:
//! - `model`   : 既定エンコード指定型 `DefaultEncoding`（下流と共有する契約の片側）
//! - `prescan` : 内部・charset 宣言の先読み（ファイル全体の行）で charset 名を抽出（D1/D2）
//! - `decode`  : 内部・prescan 結果＋encoding_rs で全体デコード（公開 facade `decode`）。
//!   解けない・ASCII と互換でないラベルと壊れた並びは `warn!` を残して読み続ける
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
