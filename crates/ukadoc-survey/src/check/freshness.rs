//! ドメイン別報告と台帳の突き合わせ（要件 7.4・7.5）。
//!
//! **中身はタスク 4.4 が書く。** いまは所見を 1 件も返さない空の実装で、入口
//! （[`crate::check::run`]）からの配線だけが済んでいる（タスク 4.1）。判定は
//! `DomainReportStale` の 1 種で、台帳 1 本から作り直した本文（`report::render_domain`）
//! と repo にある本文を突き合わせる（設計 D-11）。
//!
//! 突き合わせは復帰文字を落とした本文どうしで行う（設計 D-6）。全体報告
//! `summary.md` は見ない（要件 7.6）。

use super::{CheckInput, Finding};

/// 報告の古さを集める。
pub fn check(_input: &CheckInput) -> Vec<Finding> {
    Vec::new()
}

#[cfg(test)]
#[path = "freshness_tests.rs"]
mod tests;
