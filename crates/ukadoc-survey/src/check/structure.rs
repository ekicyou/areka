//! 構造の検査（要件 3.1・3.2・3.5・6.3・6.4）。
//!
//! 台帳とカタログの対応・ページの割り当て・台帳の並び順を判定する。設計 check 節の
//! 「判定の内訳」から、この段が受け持つのは次の 7 種である。
//!
//! - `LedgerIdNotInCatalog`（6.3）——台帳の id がカタログに無い
//! - `CatalogIdMissingFromLedgers` / `CatalogIdInMultipleLedgers`（6.4・3.2）
//! - `LedgerIdPageMismatch` / `LedgerPagesMismatch`（3.1・3.2）
//! - `LedgerOutOfOrder`（3.3a・付録 A・設計 D-12）
//! - `PageNotAssigned`（3.5）
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。失敗も
//! しない——食い違いは [`Finding`] として全部集めて返し、1 件目で止めない（設計
//! Error Handling）。
//!
//! # 担当の正本は割り当て表であって、台帳の前置きではない
//!
//! `LedgerIdPageMismatch` は [`crate::assignment::PageAssignment`] に照らして判定する。
//! 台帳の `[ledger].pages` を正本にすると、前置きが誤っている台帳では項目の側が
//! 「正しい」ことになってしまい、誤りが誤りを隠す。前置きそのものの誤りは
//! `LedgerPagesMismatch` が**集合として**別に拾う（要件 3.3a により前置きは以後
//! バイト列のまま写されるので、ここで守らないと永久に残る）。
//!
//! # 並び順は本文の順を見る
//!
//! [`Ledger::entries`] は id を鍵にした表なので作りからして昇順であり、そこを見ても
//! 何も判らない。見るのは [`Ledger::file_order`]（本文に現れた順）である。判定は
//! **厳密な**昇順で、同じ id が 2 回現れても落ちる（設計 D-12）。備考の複数行文字列の
//! 中に既存 id と同じ綴りの見出し行が隠れた場合、塊の切り分けの較正（鍵の集合との
//! 一致）では見抜けず、この厳密さだけが穴を閉じる。
//!
//! # `LedgerDomainMismatch` はここでは作れない
//!
//! `[ledger].domain` の食い違いは `ledger::read` の段で落ちる。[`Ledger`] は `domain`
//! を 1 つしか持たず、その値はファイル名から来るので、宣言された綴りが残るのは読み
//! 取りの中だけである。[`CheckInput`] は台帳ごとのファイル名を受け取らないから、
//! 検査層はこの判定を作り直せない——**書けないのが正しい**（設計 check 節の注記）。

use super::{CheckInput, Finding, FindingKind};
use crate::ledger::Ledger;
use crate::model::{Domain, EntryId, PageName};
use std::collections::BTreeSet;

/// カタログの場所（所見の「場所」に載る綴り）。
const CATALOG_FILE: &str = "doc/ukadoc-coverage/catalog.toml";

/// その台帳の場所。
fn ledger_file(domain: Domain) -> String {
    format!("doc/ukadoc-coverage/ledger/{}.toml", domain.as_key())
}

/// ページ名を本文へ並べる。1 つも無ければ「なし」と書く（空文字だと読めない）。
fn list_pages(pages: &[&str]) -> String {
    if pages.is_empty() {
        "なし".to_owned()
    } else {
        pages.join("・")
    }
}

/// 構造の食い違いを集める。
///
/// 並びは決まっている——台帳を渡された順に見て、1 本ごとに「前置き → 項目（id の
/// byte 昇順）→ 本文の順」を見る。最後にカタログの側を id の byte 昇順で見る。
/// 同じ入力なら同じ並びで返る（要件 7.3 の決まり方を検査の出力にも通す）。
pub fn check(input: &CheckInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    for ledger in input.ledgers {
        check_preamble_pages(input, ledger, &mut findings);
        check_entries(input, ledger, &mut findings);
        check_file_order(ledger, &mut findings);
    }
    check_catalog_side(input, &mut findings);
    findings
}

/// 前置きの担当ページが割り当て表と**集合として**一致するか（要件 3.1）。
///
/// 集合で比べるのは、並びの正しさが初期台帳の生成の担当だからである（要件 3.3a）。
fn check_preamble_pages(input: &CheckInput, ledger: &Ledger, findings: &mut Vec<Finding>) {
    let declared: BTreeSet<&PageName> = ledger.pages.iter().collect();
    let canonical = input.assignment.pages_of(ledger.domain);
    let assigned: BTreeSet<&PageName> = canonical.iter().collect();
    if declared == assigned {
        return;
    }

    let missing: Vec<&str> = assigned
        .difference(&declared)
        .map(|page| page.as_str())
        .collect();
    let extra: Vec<&str> = declared
        .difference(&assigned)
        .map(|page| page.as_str())
        .collect();
    findings.push(Finding::new(
        FindingKind::LedgerPagesMismatch,
        None,
        ledger_file(ledger.domain),
        format!(
            "前置きの担当ページが割り当て表と違う（足りない: {} / 余分: {}）",
            list_pages(&missing),
            list_pages(&extra)
        ),
    ));
}

/// 台帳の項目 1 つずつを、カタログの実在（要件 6.3）と担当ページ（要件 3.1・3.2）で
/// 見る。
fn check_entries(input: &CheckInput, ledger: &Ledger, findings: &mut Vec<Finding>) {
    for id in ledger.entries.keys() {
        if !input.catalog.entries.contains_key(id) {
            findings.push(Finding::new(
                FindingKind::LedgerIdNotInCatalog,
                Some(id.clone()),
                ledger_file(ledger.domain),
                "カタログに無い id",
            ));
        }

        let page = id.page();
        let detail = match input.assignment.domain_of(&page) {
            // 担当が一致していれば何も言うことは無い。
            Some(owner) if owner == ledger.domain => continue,
            Some(owner) => format!(
                "ページ {} の担当は {} で、この台帳（{}）ではない",
                page.as_str(),
                owner.as_key(),
                ledger.domain.as_key()
            ),
            // 割り当ての無いページは、どの台帳の担当でもない。ページ自体の
            // 申し立ては `PageNotAssigned` が別に出す。
            None => format!(
                "ページ {} に割り当てが無く、この台帳（{}）の担当ではない",
                page.as_str(),
                ledger.domain.as_key()
            ),
        };
        findings.push(Finding::new(
            FindingKind::LedgerIdPageMismatch,
            Some(id.clone()),
            ledger_file(ledger.domain),
            detail,
        ));
    }
}

/// 本文に現れた順が id の byte **厳密**昇順か（要件 3.3a・設計 D-12）。
///
/// 破っている側（後ろに置かれた id）を挙げる。等しい隣り合わせも落とす。
fn check_file_order(ledger: &Ledger, findings: &mut Vec<Finding>) {
    for pair in ledger.file_order.windows(2) {
        if pair[1] > pair[0] {
            continue;
        }
        findings.push(Finding::new(
            FindingKind::LedgerOutOfOrder,
            Some(pair[1].clone()),
            ledger_file(ledger.domain),
            format!("直前の id {} より後ろに並んでいない", pair[0].as_str()),
        ));
    }
}

/// カタログの側から見る——id がちょうど 1 つの台帳に現れるか（要件 6.4・3.2）と、
/// ページに割り当てがあるか（要件 3.5）。
fn check_catalog_side(input: &CheckInput, findings: &mut Vec<Finding>) {
    for id in input.catalog.entries.keys() {
        let owners = owning_domains(input, id);
        if owners.is_empty() {
            findings.push(Finding::new(
                FindingKind::CatalogIdMissingFromLedgers,
                Some(id.clone()),
                CATALOG_FILE,
                "どの台帳にも現れない id",
            ));
        } else if owners.len() > 1 {
            let names: Vec<&str> = owners.iter().map(Domain::as_key).collect();
            findings.push(Finding::new(
                FindingKind::CatalogIdInMultipleLedgers,
                Some(id.clone()),
                CATALOG_FILE,
                format!("2 つ以上の台帳にある id: {}", names.join("・")),
            ));
        }
    }

    let pages = input.assignment.unassigned(
        input
            .catalog
            .entries
            .values()
            .map(|entry: &crate::catalog::CatalogEntry| &entry.page),
    );
    for page in pages {
        findings.push(Finding::new(
            FindingKind::PageNotAssigned,
            None,
            CATALOG_FILE,
            format!("割り当ての無いページ: {}", page.as_str()),
        ));
    }
}

/// その id を持っている台帳のドメイン（重複を畳まない）。
///
/// 並びは `Domain` の導出された `Ord`＝宣言順に並べ直したもので、これは
/// [`Domain::ALL`] の並びと同じである。渡された台帳の並びは結果に出ない（要件 7.3。
/// 見本の台帳は最初から [`Domain::ALL`] の順なので、この並べ直しが効いていることは
/// 逆順で通す在中テストだけが確かめられる）。
///
/// 畳まないのは、同じドメインの台帳が 2 本渡された場合も「2 つ以上の台帳にある」と
/// 言うためである。1 本の台帳の中で同じ id が 2 度現れる形は [`Ledger::entries`] が
/// 表である以上作れず、本文の側の重複は `LedgerOutOfOrder` が拾う（設計 D-12）。
fn owning_domains(input: &CheckInput, id: &EntryId) -> Vec<Domain> {
    let mut owners: Vec<Domain> = input
        .ledgers
        .iter()
        .filter(|ledger| ledger.entries.contains_key(id))
        .map(|ledger| ledger.domain)
        .collect();
    // 渡された台帳の並びに結果を左右させない（要件 7.3）。
    owners.sort();
    owners
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
