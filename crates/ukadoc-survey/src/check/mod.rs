//! 整合検査の入口（要件 6.3〜6.8・6.10〜6.12・7.4・7.5）。
//!
//! ここは純粋層で、**ファイルには 1 つも触らない**（設計 check 節の事前条件
//! 「入力はすべて読み込み済みの値」）。カタログ・台帳 4 本・割り当て表・テーマ名・
//! 証拠の索引・ドメイン別報告の本文をすべて値として受け取り、食い違いの一覧と走査の
//! 件数を返す。読み込みは入出力層と入口（実行ファイル・`tests/consistency`）の仕事で
//! ある。スナップショットには触れない（要件 6.2）。
//!
//! # 判定は 3 つに分かれ、入口は全部を通す
//!
//! - [`structure`] — 台帳とカタログの対応・割り当て・並び順（要件 6.3・6.4・3.5）
//! - [`content`] — URL・証拠・関連・版・テーマ（要件 6.5〜6.8・6.10）
//! - [`freshness`] — ドメイン別報告と台帳の一致（要件 7.4・7.5）
//!
//! 3 つは [`JUDGEMENTS`] という 1 本の表に載っており、[`run`] はその表をたたみ込む
//! だけである。走った判定の名前は [`ScanStats::judgements_run`] に残る——部品を
//! 釘付けしても入口がその部品を呼んでいるかは別に守る必要があるからで、判定が所見を
//! 1 件も返さない間はこの記録だけが呼び忘れを見分けられる。
//!
//! # 1 件目で止めない
//!
//! 見つけた食い違いは全部集めてから返す（設計 Error Handling）。直す人が 1 度の
//! 実行で全部を読めるようにするためで、赤にするのは呼び手の仕事である。

use std::collections::{BTreeMap, BTreeSet};

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

/// 検査の入力（設計 check 節）。
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
    /// テーマ名（要件 4.4 の 8 つ）。
    pub themes: &'a [&'a str],
    /// 証拠の索引。
    pub evidence: &'a EvidenceIndex,
    /// repo にあるドメイン別報告の本文（復帰文字を落としたもの・設計 D-6）。
    pub domain_reports: &'a BTreeMap<Domain, String>,
}

/// 検査の結果。
pub struct CheckOutcome {
    /// 見つかった食い違い。空なら緑（設計 check 節の事後条件）。
    pub findings: Vec<Finding>,
    /// 何をどれだけ見たか。
    pub stats: ScanStats,
}

/// 走査の件数（要件 6.13 の「検査対象が 0 件でない」側が読む値）。
///
/// 初期台帳は全行が未分類なので、要件 6.6〜6.8 の判定は対象 0 件でも緑になる。
/// 「違反 0 件」だけでは道具が壊れていても気づけないから、**何件を見たのか**を同じ
/// 実行から取り出せるようにしてある。
///
/// **走査したソースのファイル数はここに無い**。[`CheckInput`] はソースの本文を
/// 受け取らず（受け取るのは解決の済んだ [`EvidenceIndex`]）、設計が固定した入力に
/// 足すことはしない。ソースの走査件数はタスク 8.3 が `io::sources` の戻り値を直に
/// 数える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanStats {
    /// 走らせた判定の名前（走った順）。
    pub judgements_run: Vec<&'static str>,
    /// カタログの項目数。
    pub catalog_entries: usize,
    /// カタログに現れたページの種類数。
    pub catalog_pages: usize,
    /// カタログに現れたカテゴリの種類数。
    pub catalog_categories: usize,
    /// 受け取った台帳の本数。
    pub ledgers: usize,
    /// 台帳の項目数（ドメイン別）。同じドメインの台帳が 2 本来たら足し合わせる
    /// ——黙って片方を落とすと、重複を数える側が空回りする。
    pub ledger_entries: BTreeMap<Domain, usize>,
    /// 割り当て表のページ数。
    pub assigned_pages: usize,
    /// 割り当て表のページ数（ドメイン別）。
    pub assigned_pages_by_domain: BTreeMap<Domain, usize>,
    /// 突き合わせに使ったテーマ名の数。
    pub themes: usize,
    /// 証拠の付いた項目 id の数。
    pub evidence_ids: usize,
    /// 証拠として現れたファイルパスの種類数。
    pub evidence_paths: usize,
    /// 受け取ったドメイン別報告の本数。
    pub domain_reports: usize,
    /// うち本文が空でない本数（要件 7.4 の検査が空回りしていないこと）。
    pub non_empty_domain_reports: usize,
}

/// 判定 1 つ（名前と関数）。
///
/// 名前は結果に残る（[`ScanStats::judgements_run`]）。関数だけの表にすると、走った
/// かどうかを外から確かめる手立てが無くなる。
pub struct Judgement {
    /// 判定の名前。
    pub name: &'static str,
    /// 判定そのもの。
    pub run: fn(&CheckInput) -> Vec<Finding>,
}

/// 走らせる判定の表。**この並びが走る順である**。
pub const JUDGEMENTS: [Judgement; 3] = [
    Judgement {
        name: "structure",
        run: structure::check,
    },
    Judgement {
        name: "content",
        run: content::check,
    },
    Judgement {
        name: "freshness",
        run: freshness::check,
    },
];

/// 整合検査を走らせる（設計 check 節）。
///
/// 同じ入力なら同じ結果になる（設計の不変条件）。失敗しない——食い違いは値として
/// 返り、赤にするかは呼び手が決める。
pub fn run(input: &CheckInput) -> CheckOutcome {
    run_with(&JUDGEMENTS, input)
}

/// 与えた表の判定を順に走らせる。
///
/// [`run`] から表を切り離してあるのは、代役の判定を渡して「表の項目を 1 つも
/// 落としていないか」を確かめられるようにするためである。本物の 3 つが所見を
/// 返さない間は、それ以外の手立てで畳み込みの取りこぼしを見分けられない。
fn run_with(judgements: &[Judgement], input: &CheckInput) -> CheckOutcome {
    let mut findings = Vec::new();
    let mut judgements_run = Vec::new();
    for judgement in judgements {
        findings.extend((judgement.run)(input));
        judgements_run.push(judgement.name);
    }
    CheckOutcome {
        stats: scan_stats(input, judgements_run),
        findings,
    }
}

/// 何をどれだけ見たかを数える。
fn scan_stats(input: &CheckInput, judgements_run: Vec<&'static str>) -> ScanStats {
    let mut ledger_entries: BTreeMap<Domain, usize> = BTreeMap::new();
    for ledger in input.ledgers {
        *ledger_entries.entry(ledger.domain).or_default() += ledger.entries.len();
    }

    let mut assigned_pages_by_domain: BTreeMap<Domain, usize> = BTreeMap::new();
    let mut assigned_pages = 0;
    for domain in Domain::ALL {
        let count = input.assignment.pages_of(domain).len();
        assigned_pages += count;
        assigned_pages_by_domain.insert(domain, count);
    }

    let catalog_pages: BTreeSet<_> = input
        .catalog
        .entries
        .values()
        .map(|entry| entry.page.clone())
        .collect();
    let catalog_categories: BTreeSet<&str> = input
        .catalog
        .entries
        .values()
        .map(|entry| entry.category.as_str())
        .collect();
    let evidence_paths: BTreeSet<&str> = input
        .evidence
        .by_id
        .values()
        .flat_map(|paths| paths.iter().map(String::as_str))
        .collect();

    ScanStats {
        judgements_run,
        catalog_entries: input.catalog.entries.len(),
        catalog_pages: catalog_pages.len(),
        catalog_categories: catalog_categories.len(),
        ledgers: input.ledgers.len(),
        ledger_entries,
        assigned_pages,
        assigned_pages_by_domain,
        themes: input.themes.len(),
        evidence_ids: input.evidence.by_id.len(),
        evidence_paths: evidence_paths.len(),
        domain_reports: input.domain_reports.len(),
        non_empty_domain_reports: input
            .domain_reports
            .values()
            .filter(|body| !body.is_empty())
            .count(),
    }
}
