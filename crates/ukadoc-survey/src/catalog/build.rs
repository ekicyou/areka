//! スナップショット → カタログ（ふるい分け・版番号抽出・ハッシュ）。
//!
//! ここは純粋層である。スナップショットの**ファイルは読まない**——読み終えた
//! [`SnapshotDoc`] を受け取るだけで、場所の解決も読み込みも入出力層の仕事である
//! （要件 6.2・設計 Architecture）。おかげでこの段はすべて文字列と値だけで確かめられる。
//!
//! 順は設計「System Flows・カタログ再生成」のとおり 4 段。
//!
//! 1. `source` が `ukadoc` の entry だけ残す（要件 1.4）。
//! 2. 本文から版番号を抜き出し、重複を除いて昇順に並べる（要件 1.2）。
//! 3. 本文のハッシュを計算し、**本文は捨てる**（要件 1.3・9.4）。
//! 4. 割り当ての無いページがあれば、ページ名を挙げて失敗する（要件 3.5）。

use std::collections::{BTreeMap, BTreeSet};

use super::{CATALOG_FORMAT, Catalog, CatalogEntry, SnapshotMeta};
use crate::assignment::PageAssignment;
use crate::error::SurveyError;
use crate::hash;
use crate::io::snapshot::{RawEntry, SnapshotDoc};
use crate::model::{EntryId, PageName};

/// カタログに残す出典の綴り（要件 1.4）。
const UKADOC_SOURCE: &str = "ukadoc";

/// 版番号の欄の数（`数字+.数字+.数字+` の 3 欄）。
const VERSION_PART_COUNT: usize = 3;

/// 版番号の欄の区切り。
const VERSION_SEPARATOR: char = '.';

/// 失敗の本文でページ名を並べるときの区切り（要件 3.1 の表と同じ中黒）。
const PAGE_SEPARATOR: &str = "・";

/// スナップショットからカタログを組み立てる（要件 1.2・1.3・1.4・1.6・1.9・3.5・9.4）。
///
/// 受け取る文書は `source` が `ukadoc` 以外の entry を含んでいてよい。ふるい分けは
/// この段の判断で、入出力層は判断を持たない（設計 catalog「事前条件」）。
///
/// 失敗するのは 3 つの場合だけで、いずれも何がどう悪いかを本文に載せる。
///
/// - 正典由来の id が 2 形のどちらでもない → [`SurveyError::BadEntryId`]（要件 1.9）
/// - 同じ id が 2 度現れる → [`SurveyError::SnapshotShape`]
/// - どの台帳にも割り当ての無いページがある → [`SurveyError::PageNotAssigned`]（要件 3.5）
pub fn build(doc: &SnapshotDoc, assignment: &PageAssignment) -> Result<Catalog, SurveyError> {
    let mut entries: BTreeMap<EntryId, CatalogEntry> = BTreeMap::new();
    for raw in doc.entries.iter().filter(|raw| raw.source == UKADOC_SOURCE) {
        let entry = catalog_entry(raw)?;
        let id = entry.id.clone();
        // 表に入れるだけだと、同じ id の 2 件目が 1 件目を黙って上書きする。件数
        // だけが 1 つ減って形は整って見えるので、ここで止める（要件 1.8）。
        if let Some(previous) = entries.insert(id, entry) {
            return Err(SurveyError::SnapshotShape {
                detail: format!("同じ項目 id が 2 度現れる: {}", previous.id.as_str()),
            });
        }
    }

    let unassigned = assignment.unassigned(entries.values().map(|entry| &entry.page));
    if !unassigned.is_empty() {
        return Err(SurveyError::PageNotAssigned {
            pages: unassigned
                .iter()
                .map(PageName::as_str)
                .collect::<Vec<&str>>()
                .join(PAGE_SEPARATOR),
        });
    }

    Ok(Catalog {
        snapshot: SnapshotMeta {
            package: doc.package.clone(),
            package_version: doc.package_version.clone(),
            snapshot_version: doc.version,
            generated_at: doc.generated_at.clone(),
            // 全件数は出典を問わない件数、正典の件数は残した件数。別々に数える
            // （要件 1.6。実測では 2,983 と 1,749）。
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
/// 本文を使うのは版番号の抜き出しとハッシュの計算の 2 つだけで、写し終えた本文は
/// [`CatalogEntry`] のどの欄にも入らない（要件 1.3・9.4）。
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

/// 本文に現れる版番号のすべて。重複を除き、文字列として昇順（要件 1.2）。
///
/// 規則は設計「版番号の抽出規則」の逐語——「前後が数字でも小数点でもない
/// `数字+.数字+.数字+`」をすべて拾う。ここでは同じことを次の形で言い換えて実装する。
///
/// > 本文を「数字と小数点だけが連なる部分」に切り分け、その**塊まるごと**が
/// > `数字+.数字+.数字+` になっているものだけを採る。
///
/// 塊は数字でも小数点でもない文字で区切られるので、塊の直前・直後の文字は必ず
/// 「数字でも小数点でもない」。逆に塊の一部だけを採ると、その両隣のどちらかは
/// 必ず塊の中の文字＝数字か小数点になる。だから 2 つの言い方は同じものを指す
/// （実データ 1,749 件で両者の結果が 1 件も食い違わないことを確かめてある）。
///
/// 直前が英字のときは拾う。実データの `SSP2.3.00以降` がその形で、語の境界で
/// 切る規則にすると本文 4 件から版番号が落ちる。
///
/// 拾った値は語の形だけで選んだ候補である。`5.19.0` や `7.4.1` のように SSP の版で
/// ない値が 2 件混じることを承知の上でそのまま記録する——形で絞ると将来の 3 系を
/// 落とすため（設計 版番号の抽出規則）。
fn versions_in(content: &str) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    for run in content.split(|c: char| !is_version_char(c)) {
        if is_version(run) {
            found.insert(run.to_owned());
        }
    }
    // `BTreeSet<String>` の並びは文字列の byte 昇順。版番号はすべて ASCII なので、
    // これがそのまま「文字列として昇順」になる（数としての昇順ではない）。
    found.into_iter().collect()
}

/// 版番号の塊を作る文字か（半角数字と小数点だけ）。
///
/// 全角数字は数字として扱わない。ここを広げると塊の切れ目が変わり、規則の意味が
/// 変わってしまう。
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
