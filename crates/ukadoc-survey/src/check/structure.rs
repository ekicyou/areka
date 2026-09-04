//! 構造の検査（要件 6.3・6.4・3.5 と台帳の並び順）。
//!
//! **中身はタスク 4.2 が書く。** いまは所見を 1 件も返さない空の実装で、入口
//! （[`crate::check::run`]）からの配線だけが済んでいる（タスク 4.1）。判定の一覧は
//! 設計 check 節の「判定の内訳」にある次の 7 種である。
//!
//! - `LedgerIdNotInCatalog`（6.3）
//! - `CatalogIdMissingFromLedgers` / `CatalogIdInMultipleLedgers`（6.4・3.2）
//! - `LedgerIdPageMismatch` / `LedgerPagesMismatch`（3.1・3.2）
//! - `LedgerOutOfOrder`（3.3a・付録 A）
//! - `PageNotAssigned`（3.5）
//!
//! `LedgerDomainMismatch` は種類としては [`crate::check::FindingKind`] に残るが、
//! **台帳ファイルからこの所見を出すことはできない**（`Ledger` は `domain` を 1 つしか
//! 持たず、その値はファイル名から来るので、宣言された綴りが残るのは `ledger::read`
//! の中だけである）。書けないのが正しい。

use super::{CheckInput, Finding};

/// 構造の食い違いを集める。
pub fn check(_input: &CheckInput) -> Vec<Finding> {
    Vec::new()
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
