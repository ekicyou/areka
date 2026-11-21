//! 整合検査の入口。
//!
//! ここは純粋層で、**ファイルには 1 つも触らない**。カタログ・台帳 4 本・割り当て表・
//! テーマ名・証拠の索引・ドメイン別報告と全体報告の本文をすべて読み込み済みの値として
//! 受け取り、食い違いの一覧を返す。読み込みは入出力層と入口（実行ファイル・
//! `tests/consistency`）の仕事である。スナップショットには触れない。
//!
//! # 判定は 3 つに分かれ、入口は全部を通す
//!
//! - [`structure`] — 台帳とカタログの対応・割り当て・並び順
//! - [`content`] — URL・証拠・関連・版
//! - [`freshness`] — ドメイン別報告 4 本と全体報告が台帳と一致するか
//!
//! [`run`] はこの 3 つを順に呼んで所見を足す。どの判定のテストも `run` を通すので、
//! 呼び忘れた判定があればその判定の所見を見るテストが赤になる。
//!
//! # 1 件目で止めない
//!
//! 見つけた食い違いは全部集めてから返す。直す人が 1 度の実行で全部を読めるようにするため
//! で、赤にするのは呼び手の仕事である。

use std::collections::BTreeMap;

use crate::assignment::PageAssignment;
use crate::catalog::Catalog;
use crate::evidence::EvidenceIndex;
use crate::ledger::Ledger;
use crate::model::Domain;

pub mod content;
pub mod finding;
pub mod freshness;
pub mod structure;

pub use finding::{Finding, FindingKind, render};

/// 検査の入力。
///
/// すべて借りた値で、この型は何も所有しない。読み込みの済んだ値だけを受け取ることが、
/// 「検査はファイルに触らない」を型の形で表している。
pub struct CheckInput<'a> {
    /// 正典の写し。
    pub catalog: &'a Catalog,
    /// 台帳 4 本。
    pub ledgers: &'a [Ledger],
    /// ページ→ドメインの割り当て（担当の正本）。
    pub assignment: &'a PageAssignment,
    /// テーマ名。
    pub themes: &'a [&'a str],
    /// 証拠の索引。
    pub evidence: &'a EvidenceIndex,
    /// repo にあるドメイン別報告の本文。
    pub domain_reports: &'a BTreeMap<Domain, String>,
    /// repo にある全体報告 `report/summary.md` の本文（ドメイン別報告と同じく復帰文字を
    /// 落としたもの）。
    pub summary_report: &'a str,
}

/// 検査の結果。
pub struct CheckOutcome {
    /// 見つかった食い違い。空なら緑。
    pub findings: Vec<Finding>,
}

/// 整合検査を走らせる。
///
/// 同じ入力なら同じ結果になる。失敗しない——食い違いは値として返り、赤にするかは
/// 呼び手が決める。
pub fn run(input: &CheckInput) -> CheckOutcome {
    let mut findings = structure::check(input);
    findings.extend(content::check(input));
    findings.extend(freshness::check(input));
    CheckOutcome { findings }
}

#[cfg(test)]
mod test_support;
