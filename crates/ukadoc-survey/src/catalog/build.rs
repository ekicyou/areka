//! スナップショット → カタログ（ふるい分け・版番号抽出・ハッシュ）。
//!
//! 純粋層で、スナップショットの**ファイルは読まない**。読み終えた [`SnapshotDoc`] を
//! 受け取るだけで、場所の解決も読み込みも入出力層の仕事である。
//!
//! 順は 4 段。
//!
//! 1. `source` が `ukadoc` の entry だけ残す。
//! 2. 本文から版番号を抜き出し、重複を除いて昇順に並べる。
//! 3. 本文のハッシュを計算し、**本文は捨てる**。
//! 4. 割り当ての無いページがあれば、ページ名を挙げて失敗する。

use std::collections::{BTreeMap, BTreeSet};

use super::{CATALOG_FORMAT, Catalog, CatalogEntry, SnapshotMeta, read};
use crate::assignment::PageAssignment;
use crate::error::SurveyError;
use crate::hash;
use crate::io::snapshot::{RawEntry, SnapshotDoc};
use crate::model::{EntryId, PageName};

/// カタログに残す出典の綴り。
const UKADOC_SOURCE: &str = "ukadoc";

/// 版番号の欄の数（`数字+.数字+.数字+` の 3 欄）。
const VERSION_PART_COUNT: usize = 3;

/// 版番号の欄の区切り。
const VERSION_SEPARATOR: char = '.';

/// 失敗の本文でページ名を並べるときの区切り（中黒）。
const PAGE_SEPARATOR: &str = "・";

/// どの台帳にも割り当ての無いページの失敗。カタログの組み立てと台帳への仕分けが
/// 同じ綴りで落ちるよう、組み立てはここ 1 か所だけが持つ。
///
/// `pages` は [`PageAssignment::unassigned`] の返り値（名前順・重複無し）をそのまま渡す。
pub(crate) fn page_not_assigned(pages: &[PageName]) -> SurveyError {
    SurveyError::PageNotAssigned {
        pages: pages
            .iter()
            .map(PageName::as_str)
            .collect::<Vec<&str>>()
            .join(PAGE_SEPARATOR),
    }
}

/// スナップショットからカタログを組み立てる。
///
/// 受け取る文書は `source` が `ukadoc` 以外の entry を含んでいてよい。ふるい分けは
/// この段の判断で、入出力層は判断を持たない。
///
/// 次の場合に失敗し、いずれも何がどう悪いかを本文に載せる。
///
/// - 正典由来の id が 2 形のどちらでもない → [`SurveyError::BadEntryId`]
/// - 同じ id が 2 度現れる → [`SurveyError::SnapshotShape`]
/// - 同じ URL が 2 つの id に現れる → [`SurveyError::SnapshotShape`]
/// - 別のページが同じページ URL に畳まれる → [`SurveyError::SnapshotShape`]
/// - どの台帳にも割り当ての無いページがある → [`SurveyError::PageNotAssigned`]
pub fn build(doc: &SnapshotDoc, assignment: &PageAssignment) -> Result<Catalog, SurveyError> {
    let mut entries: BTreeMap<EntryId, CatalogEntry> = BTreeMap::new();
    let mut id_of_url: BTreeMap<&str, &str> = BTreeMap::new();
    let mut page_of_url: BTreeMap<&str, PageName> = BTreeMap::new();
    for raw in doc.entries.iter().filter(|raw| raw.source == UKADOC_SOURCE) {
        let entry = catalog_entry(raw)?;
        let id = entry.id.clone();
        let page = entry.page.clone();
        // 同じ id の 2 件目が 1 件目を黙って上書きすると、件数だけが 1 つ減って
        // 形は整って見える。
        if let Some(previous) = entries.insert(id, entry) {
            return Err(SurveyError::SnapshotShape {
                detail: format!("同じ項目 id が 2 度現れる: {}", previous.id.as_str()),
            });
        }
        // URL からの逆引き（`Catalog::by_url`）は 1 対 1 が前提。2 つ目を通すと、
        // その URL の証拠が片方の id にだけ黙って付く。
        if let Some(previous) = id_of_url.insert(&raw.url, &raw.id) {
            return Err(SurveyError::SnapshotShape {
                detail: format!(
                    "同じ URL が 2 つの項目 id に現れる: {}（{previous}・{}）",
                    raw.url, raw.id
                ),
            });
        }
        // ページ URL からの逆引き（`Catalog::page_urls`）も 1 対 1 が前提。畳まれると
        // ページ単位の証拠が別のページに付く。
        let page_url = read::page_url(&raw.url);
        if let Some(previous) = page_of_url.insert(page_url, page.clone())
            && previous != page
        {
            return Err(SurveyError::SnapshotShape {
                detail: format!(
                    "別のページが同じページ URL に畳まれる: {page_url}（{}・{}）",
                    previous.as_str(),
                    page.as_str()
                ),
            });
        }
    }

    let unassigned = assignment.unassigned(entries.values().map(|entry| &entry.page));
    if !unassigned.is_empty() {
        return Err(page_not_assigned(&unassigned));
    }

    Ok(Catalog {
        snapshot: SnapshotMeta {
            package: doc.package.clone(),
            package_version: doc.package_version.clone(),
            snapshot_version: doc.version,
            generated_at: doc.generated_at.clone(),
            // 全件数は出典を問わない件数、正典の件数は残した件数。
            total_entries: doc.entries.len(),
            ukadoc_entries: entries.len(),
            catalog_format: CATALOG_FORMAT,
            hash_algorithm: hash::HASH_ALGORITHM.to_owned(),
        },
        entries,
    })
}

/// 正典由来の entry 1 件をカタログの 1 項目へ写す。
///
/// 本文を使うのは版番号の抜き出しとハッシュの計算だけで、本文は [`CatalogEntry`] のどの欄にも入らない。
fn catalog_entry(raw: &RawEntry) -> Result<CatalogEntry, SurveyError> {
    let id = EntryId::parse(&raw.id)?;
    let page = id.page();
    Ok(CatalogEntry {
        id,
        page,
        title: raw.title.clone(),
        category: raw.category.clone(),
        versions: versions_in(&raw.content),
        hash: hash::content_hash(&raw.content),
        url: raw.url.clone(),
    })
}

/// 本文に現れる版番号のすべて。重複を除き、文字列として昇順。
///
/// 「前後が数字でも小数点でもない `数字+.数字+.数字+`」をすべて拾う規則を、本文を
/// 「数字と小数点だけが連なる塊」に切り分け、塊まるごとが `数字+.数字+.数字+` に
/// なっているものだけを採る形で実装する。塊の一部だけを採ると両隣のどちらかは必ず
/// 数字か小数点になるので、2 つの言い方は同じものを指す。
///
/// 直前が英字のときは拾う。実データの `SSP2.3.00以降` がその形で、語の境界で切ると
/// その本文から版番号が落ちる。
///
/// `5.19.0` や `7.4.1` のように SSP の版でない値が混じることを承知でそのまま記録する。
/// 形で絞ると将来の 3 系を落とす。
fn versions_in(content: &str) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    for run in content.split(|c: char| !is_version_char(c)) {
        if is_version(run) {
            found.insert(run.to_owned());
        }
    }
    // `BTreeSet<String>` は byte 昇順。版番号は ASCII なので文字列として昇順になる（数としてではない）。
    found.into_iter().collect()
}

/// 版番号の塊を作る文字か（半角数字と小数点だけ）。
///
/// 全角数字は数字として扱わない。広げると塊の切れ目が変わり、規則の意味が変わる。
fn is_version_char(c: char) -> bool {
    c.is_ascii_digit() || c == VERSION_SEPARATOR
}

/// その塊がまるごと `数字+.数字+.数字+` か。
fn is_version(run: &str) -> bool {
    let parts: Vec<&str> = run.split(VERSION_SEPARATOR).collect();
    parts.len() == VERSION_PART_COUNT
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
