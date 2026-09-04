//! 内容の検査（要件 6.5・6.6・6.7・6.8・6.10）。
//!
//! **中身はタスク 4.3 が書く。** いまは所見を 1 件も返さない空の実装で、入口
//! （[`crate::check::run`]）からの配線だけが済んでいる（タスク 4.1）。判定の一覧は
//! 設計 check 節の「判定の内訳」にある次の 5 種である。
//!
//! - `SourceUrlNotInCatalog`（6.5・6.10）
//! - `ImplementedWithoutEvidence`（6.6）
//! - `LinkEndpointMissing` / `AliasChain` / `IntroducedNotInCatalogVersions`（6.7・2.4）
//! - `UnknownTheme`（6.8）
//!
//! 手掛かり候補（要件 5.8）は見ない。候補は証拠ではない（要件 5.9）。

use super::{CheckInput, Finding};

/// 内容の食い違いを集める。
pub fn check(_input: &CheckInput) -> Vec<Finding> {
    Vec::new()
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
