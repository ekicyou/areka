//! `catalog.toml` の本文 → カタログ、およびカタログからの 3 つの読み出し。
//!
//! 読み取りは `toml` に任せる（設計「境界の要点」1 つ目。書き出しだけが自前）。
//! ここは純粋層で、**ファイルには触らない**——本文の文字列を受け取るだけである
//! （要件 6.2）。
//!
//! # 繕わない
//!
//! 形が違う本文は黙って直さずに落とす（設計 Error Handling・要件 1.8）。落とすのは
//! 次の場合で、いずれも**どこが**おかしいかを本文に載せる。
//!
//! - TOML として読めない／`[snapshot]`・`[entry]` が無い
//! - 冒頭の情報の欄が欠けている・型が違う・件数が負である
//! - 知らない欄がある（形の版が違うカタログを黙って読み飛ばさないため）
//! - 項目 id が 2 形のどちらでもない（要件 1.9）
//! - `page` の欄が id のページと食い違う
//!
//! 最後の 1 つは「書かれた値を捨てて id から作り直す」ことも考えられるが、採らない。
//! 作り直すと壊れたカタログが黙って通り、しかも読んで書き戻した本文が元と変わって
//! しまう（設計 catalog の事後条件 `write(read(t)) == t` が破れる）。カタログは
//! 機械生成の文書なので、食い違いは再生成すれば消える。
//!
//! # 失敗に添える場所
//!
//! 設計の [`read`] は本文だけを受け取り、どのファイルを読んでいるかを知らない。
//! それでも [`SurveyError::TomlParse`] は場所を求めるので、カタログの決まった置き場
//! （[`CATALOG_FILE`]）を添える。カタログの置き場は `io::paths::catalog_path` が
//! 決める 1 か所きりなので、これで実際に読んでいたファイルと食い違うことはない。

use std::collections::BTreeMap;

use super::{Catalog, CatalogEntry, SnapshotMeta};
use crate::error::SurveyError;
use crate::model::{EntryId, PageName};

/// 失敗の本文に添えるカタログの置き場（`io::paths::catalog_path` と同じ場所）。
const CATALOG_FILE: &str = "doc/ukadoc-coverage/catalog.toml";

/// 冒頭の情報を置く表の名前。
const SNAPSHOT_TABLE: &str = "snapshot";

/// 項目を置く表の名前。
const ENTRY_TABLE: &str = "entry";

/// 冒頭の情報に置いてよい欄のすべて（要件 1.6・設計 D-9）。
const SNAPSHOT_FIELDS: [&str; 8] = [
    "package",
    "package_version",
    "snapshot_version",
    "generated_at",
    "total_entries",
    "ukadoc_entries",
    "catalog_format",
    "hash_algorithm",
];

/// 項目に置いてよい欄のすべて（設計「Data Models」の列表）。
const ENTRY_COLUMNS: [&str; 6] = ["page", "title", "category", "versions", "hash", "url"];

/// URL のフラグメントの始まりの印。
const FRAGMENT_MARK: char = '#';

/// `catalog.toml` の本文を読む（要件 1.1・1.6）。
///
/// [`crate::catalog::write::write`] が組み立てた本文を渡せば、値は 1 つも欠けずに
/// 戻り、書き戻すと元の本文に 1 バイトも違わず一致する（設計 catalog の事後条件）。
///
/// 形の版（`catalog_format`）はそのまま写すだけで、ここでは判断しない。古い形の
/// カタログを見分けるのは検査の仕事で、読み取りが値を握り潰すと見分けようがなくなる。
pub fn read(text: &str) -> Result<Catalog, SurveyError> {
    let root: toml::Table = text
        .parse()
        .map_err(|err| malformed(format!("TOML として読めない: {err}")))?;
    Ok(Catalog {
        snapshot: read_snapshot(&root)?,
        entries: read_entries(&root)?,
    })
}

impl Catalog {
    /// URL から項目 id を引く逆引き（設計 D-4 の 1 段目）。
    ///
    /// **完全一致だけ**を扱う。フラグメントを外した綴りはここでは引けず、
    /// [`Self::page_urls`] が 2 段目を受け持つ。実測で 1,749 件の URL はすべて
    /// 相異なり、ある URL が別の URL の先頭部分になっている例も 0 件なので、
    /// 完全一致で曖昧さなく 1 件に定まる（設計 D-4）。
    ///
    /// 同じ URL の項目が 2 つある壊れたカタログでは、id の大きい方だけが残る。
    pub fn by_url(&self) -> BTreeMap<&str, &EntryId> {
        self.entries
            .iter()
            .map(|(id, entry)| (entry.url.as_str(), id))
            .collect()
    }

    /// フラグメントを外したページ URL の一覧（設計 D-4 の 2 段目・実測 38 種）。
    ///
    /// 最初の `#` より前を採る。同じページの項目は同じ綴りに畳まれるので、返る件数は
    /// 項目数ではなくページ数になる。アンカーを持たない 19 件は `#` を含まないので
    /// URL がそのまま鍵になる。
    pub fn page_urls(&self) -> BTreeMap<String, PageName> {
        self.entries
            .values()
            .map(|entry| {
                let url = match entry.url.split_once(FRAGMENT_MARK) {
                    Some((head, _)) => head.to_owned(),
                    None => entry.url.clone(),
                };
                (url, entry.page.clone())
            })
            .collect()
    }

    /// 1 ページ分の見出しの一覧（設計 D-5 の名前の突き合わせが使う）。
    ///
    /// 並びは **id の昇順**——`entries` が `BTreeMap` なので、その走査順がそのまま
    /// 出る。同じカタログを何度引いても同じ並びで返る。該当が無ければ空。
    pub fn titles_of_page(&self, page: &PageName) -> Vec<(&EntryId, &str)> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.page == *page)
            .map(|(id, entry)| (id, entry.title.as_str()))
            .collect()
    }
}

/// カタログの形が違うことを告げる失敗。
fn malformed(reason: impl Into<String>) -> SurveyError {
    SurveyError::TomlParse {
        path: CATALOG_FILE.to_owned(),
        reason: reason.into(),
    }
}

/// 冒頭の情報を読む（要件 1.6）。
fn read_snapshot(root: &toml::Table) -> Result<SnapshotMeta, SurveyError> {
    let table = sub_table(root, SNAPSHOT_TABLE)?;
    let place = "[snapshot]";
    reject_unknown_keys(table, &SNAPSHOT_FIELDS, place)?;
    Ok(SnapshotMeta {
        package: string_field(table, place, "package")?,
        package_version: string_field(table, place, "package_version")?,
        snapshot_version: integer_field(table, place, "snapshot_version")?,
        generated_at: string_field(table, place, "generated_at")?,
        total_entries: count_field(table, place, "total_entries")?,
        ukadoc_entries: count_field(table, place, "ukadoc_entries")?,
        catalog_format: format_field(table, place, "catalog_format")?,
        hash_algorithm: string_field(table, place, "hash_algorithm")?,
    })
}

/// 項目を読む（要件 1.1・1.9）。
fn read_entries(root: &toml::Table) -> Result<BTreeMap<EntryId, CatalogEntry>, SurveyError> {
    let table = sub_table(root, ENTRY_TABLE)?;
    let mut entries: BTreeMap<EntryId, CatalogEntry> = BTreeMap::new();
    for (raw_id, value) in table {
        // 綴りが 2 形のどちらでもなければ、ここで `BadEntryId` になる（要件 1.9）。
        let id = EntryId::parse(raw_id)?;
        let place = format!("[entry] の {raw_id}");
        let item = value
            .as_table()
            .ok_or_else(|| malformed(format!("{place}: 1 行のインラインテーブルでない")))?;
        reject_unknown_keys(item, &ENTRY_COLUMNS, &place)?;

        let written_page = string_field(item, &place, "page")?;
        let page = id.page();
        if written_page != page.as_str() {
            return Err(malformed(format!(
                "{place}: page の値 {written_page} が id のページ {} と食い違う",
                page.as_str()
            )));
        }

        let entry = CatalogEntry {
            id: id.clone(),
            page,
            title: string_field(item, &place, "title")?,
            category: string_field(item, &place, "category")?,
            versions: string_array_field(item, &place, "versions")?,
            hash: string_field(item, &place, "hash")?,
            url: string_field(item, &place, "url")?,
        };
        entries.insert(id, entry);
    }
    Ok(entries)
}

/// 最上位の表の 1 つを取り出す。
fn sub_table<'a>(root: &'a toml::Table, name: &str) -> Result<&'a toml::Table, SurveyError> {
    root.get(name)
        .ok_or_else(|| malformed(format!("[{name}] が無い")))?
        .as_table()
        .ok_or_else(|| malformed(format!("[{name}] が表でない")))
}

/// 知らない欄が混じっていないことを確かめる。
fn reject_unknown_keys(
    table: &toml::Table,
    allowed: &[&str],
    place: &str,
) -> Result<(), SurveyError> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(malformed(format!("{place}: 知らない欄 {key}")));
        }
    }
    Ok(())
}

/// 欄を 1 つ取り出す。無ければ落ちる。
fn field<'a>(
    table: &'a toml::Table,
    place: &str,
    key: &str,
) -> Result<&'a toml::Value, SurveyError> {
    table
        .get(key)
        .ok_or_else(|| malformed(format!("{place}: 欄 {key} が無い")))
}

/// 文字列の欄。
fn string_field(table: &toml::Table, place: &str, key: &str) -> Result<String, SurveyError> {
    field(table, place, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| malformed(format!("{place}: 欄 {key} が文字列でない")))
}

/// 整数の欄。
fn integer_field(table: &toml::Table, place: &str, key: &str) -> Result<i64, SurveyError> {
    field(table, place, key)?
        .as_integer()
        .ok_or_else(|| malformed(format!("{place}: 欄 {key} が整数でない")))
}

/// 件数の欄。負の数は入らない。
fn count_field(table: &toml::Table, place: &str, key: &str) -> Result<usize, SurveyError> {
    let value = integer_field(table, place, key)?;
    usize::try_from(value)
        .map_err(|_| malformed(format!("{place}: 欄 {key} の値 {value} は件数にならない")))
}

/// 形の版の欄。負の数も大きすぎる数も入らない。
fn format_field(table: &toml::Table, place: &str, key: &str) -> Result<u32, SurveyError> {
    let value = integer_field(table, place, key)?;
    u32::try_from(value)
        .map_err(|_| malformed(format!("{place}: 欄 {key} の値 {value} は形の版にならない")))
}

/// 文字列の配列の欄。要素の 1 つでも文字列でなければ落ちる。
fn string_array_field(
    table: &toml::Table,
    place: &str,
    key: &str,
) -> Result<Vec<String>, SurveyError> {
    let array = field(table, place, key)?
        .as_array()
        .ok_or_else(|| malformed(format!("{place}: 欄 {key} が配列でない")))?;
    let mut values = Vec::with_capacity(array.len());
    for element in array {
        let value = element
            .as_str()
            .ok_or_else(|| malformed(format!("{place}: 欄 {key} に文字列でない要素がある")))?;
        values.push(value.to_owned());
    }
    Ok(values)
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
