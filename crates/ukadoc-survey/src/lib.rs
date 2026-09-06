//! ukadoc 調査ツールキットのライブラリ本体。
//!
//! ukadoc（SSP 公式仕様書）の項目について「正典の写し（カタログ）」と
//! 「areka の判定（台帳）」を建て、その整合を常時走るテストで守る道具である。
//! areka の実行時コードは 1 行も参照しない（要件 9.1）。
//!
//! 中身は 2 層に分かれる（設計「Architecture Pattern & Boundary Map」）。
//!
//! - 純粋層 — ファイルにもスナップショットにも触らない。文字列と値だけを受け取り、
//!   文字列と値だけを返す（[`model`]・[`assignment`]・[`hash`]・[`tomlout`]・
//!   [`catalog`]・[`ledger`]・[`evidence`]・[`check`]・[`report`]・[`diff`]）。
//! - 入出力層 — 場所の解決・読み書き・走査・JSON の読み込み。判断は持たない（[`io`]）。
//!
//! 入口は実行ファイル（[`cli`]）と常時走るテスト（`tests/consistency.rs`）の 2 つで、
//! 判定の実体は純粋層に 1 つしかない。

pub mod assignment;
pub mod catalog;
pub mod check;
pub mod cli;
pub mod diff;
pub mod error;
pub mod evidence;
pub mod hash;
pub mod io;
pub mod ledger;
pub mod model;
pub mod report;
pub mod tomlout;

pub use error::SurveyError;

// 見本データは全モジュールのテストが共用する（設計 File Structure Plan）。
// 別ディレクトリのテストからも `crate::lib_test_support` で引けるよう、
// 接続はここに 1 つだけ置く。
#[cfg(test)]
#[path = "lib_test_support.rs"]
pub(crate) mod lib_test_support;
