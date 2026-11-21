//! `catalog.toml` の本文 → カタログ、およびカタログからの 3 つの読み出し。
//!
//! 読み取りは `toml` に任せる（書き出しだけが自前）。純粋層で、**ファイルには触らない**。
//! 本文の文字列を受け取るだけである。
//!
//! # 繕わない
//!
//! 形が違う本文は黙って直さずに落とす。落とすのは次の場合で、いずれも**どこが**
//! おかしいかを本文に載せる。
//!
//! - TOML として読めない／`[snapshot]`・`[entry]` が無い
//! - 冒頭の情報の欄が欠けている・型が違う・件数が負である
//! - 根・`[snapshot]`・項目に知らない鍵がある（形の版が違うカタログを黙って読み飛ばさないため）
//! - 項目 id が 2 形のどちらでもない
//! - `page` の欄が id のページと食い違う
//! - 同じ URL の 2 項目がある／別のページが同じページ URL に畳まれる
//!
//! `page` の食い違いを「書かれた値を捨てて id から作り直す」ことは採らない。作り直すと
//! 壊れたカタログが黙って通り、しかも読んで書き戻した本文が元と変わってしまう
//! （`write(read(t)) == t` が破れる）。カタログは機械生成の文書なので、食い違いは
//! 再生成すれば消える。
//!
//! # 失敗に添える場所
//!
//! [`read`] は本文だけを受け取り、どのファイルを読んでいるかを知らない。それでも
//! [`SurveyError::TomlParse`] は場所を求めるので、カタログの決まった置き場
//! （`io::paths::CATALOG_FILE`）を添える。`io::paths::catalog_path` と同じ層が持つ
//! 綴りなので、実際に読んでいたファイルと食い違うことはない。

use std::collections::BTreeMap;

use super::{Catalog, CatalogEntry, ENTRY_COLUMNS, SNAPSHOT_FIELDS, SnapshotMeta};
use crate::documents::fields::{
    field, malformed, reject_unknown_keys, required_table, string_array_field, string_field,
    usize_field,
};
use crate::error::SurveyError;
use crate::io::paths::CATALOG_FILE;
use crate::model::{EntryId, PageName};

/// 冒頭の情報を置く表の名前。
const SNAPSHOT_TABLE: &str = "snapshot";

/// 項目を置く表の名前。
const ENTRY_TABLE: &str = "entry";

/// URL のフラグメントの始まりの印。
const FRAGMENT_MARK: char = '#';

/// `catalog.toml` の本文を読む。
///
/// [`crate::catalog::write::write`] が組み立てた本文を渡せば、値は 1 つも欠けずに
/// 戻り、書き戻すと元の本文に 1 バイトも違わず一致する。
///
/// 形の版（`catalog_format`）はそのまま写すだけで、ここでは判断しない。古い形の
/// カタログを見分けるのは検査の仕事で、読み取りが値を握り潰すと見分けようがなくなる。
pub fn read(text: &str) -> Result<Catalog, SurveyError> {
    let root: toml::Table = text
        .parse()
        .map_err(|err| malformed(CATALOG_FILE, format!("TOML として読めない: {err}")))?;
    let snapshot = read_snapshot(&root)?;
    let entries = read_entries(&root)?;
    // 表が欠けたときはその表の名で落ちるよう、根の確かめは 2 つの表の後に置く。
    reject_unknown_keys(&root, &[SNAPSHOT_TABLE, ENTRY_TABLE], "根", CATALOG_FILE)?;
    Ok(Catalog { snapshot, entries })
}

impl Catalog {
    /// URL から項目 id を引く逆引き。
    ///
    /// **完全一致だけ**を扱う。フラグメントを外した綴りは [`Self::page_urls`] が受け持つ。
    /// 実データの URL はすべて相異なり、ある URL が別の URL の先頭部分になっている例も
    /// 無いので、完全一致で 1 件に定まる。同じ URL の 2 項目はカタログの組み立て
    /// （[`crate::catalog::build::build`]）と読み取り（[`read`]）が拒む。
    pub fn by_url(&self) -> BTreeMap<&str, &EntryId> {
        self.entries
            .iter()
            .map(|(id, entry)| (entry.url.as_str(), id))
            .collect()
    }

    /// フラグメントを外したページ URL の一覧。
    ///
    /// 最初の `#` より前を採る。同じページの項目は同じ綴りに畳まれるので、返る件数は
    /// 項目数ではなくページ数になる。別のページが同じページ URL に畳まれるカタログは
    /// 組み立てと読み取りが拒むので、1 つのページ URL は 1 つのページに決まる。
    pub fn page_urls(&self) -> BTreeMap<String, PageName> {
        self.entries
            .values()
            .map(|entry| (page_url(&entry.url).to_owned(), entry.page.clone()))
            .collect()
    }

    /// 1 ページ分の見出しの一覧。
    ///
    /// 並びは **id の昇順**（`entries` が `BTreeMap` なので走査順がそのまま出る）。
    /// 該当が無ければ空。
    pub fn titles_of_page(&self, page: &PageName) -> Vec<(&EntryId, &str)> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.page == *page)
            .map(|(id, entry)| (id, entry.title.as_str()))
            .collect()
    }
}

/// URL からフラグメントを外したページ URL（最初の `#` より前・無ければ URL そのもの）。
pub(super) fn page_url(url: &str) -> &str {
    url.split_once(FRAGMENT_MARK).map_or(url, |(head, _)| head)
}

/// 冒頭の情報を読む。
fn read_snapshot(root: &toml::Table) -> Result<SnapshotMeta, SurveyError> {
    let place = "[snapshot]";
    let table = required_table(root, SNAPSHOT_TABLE, place, CATALOG_FILE)?;
    reject_unknown_keys(table, &SNAPSHOT_FIELDS, place, CATALOG_FILE)?;
    Ok(SnapshotMeta {
        package: string_field(table, place, "package", CATALOG_FILE)?,
        package_version: string_field(table, place, "package_version", CATALOG_FILE)?,
        snapshot_version: integer_field(table, place, "snapshot_version")?,
        generated_at: string_field(table, place, "generated_at", CATALOG_FILE)?,
        total_entries: usize_field(table, place, "total_entries", CATALOG_FILE)?,
        ukadoc_entries: usize_field(table, place, "ukadoc_entries", CATALOG_FILE)?,
        catalog_format: format_field(table, place, "catalog_format")?,
        hash_algorithm: string_field(table, place, "hash_algorithm", CATALOG_FILE)?,
    })
}

/// 項目を読む。
fn read_entries(root: &toml::Table) -> Result<BTreeMap<EntryId, CatalogEntry>, SurveyError> {
    let table = required_table(root, ENTRY_TABLE, "[entry]", CATALOG_FILE)?;
    let mut entries: BTreeMap<EntryId, CatalogEntry> = BTreeMap::new();
    // 2 つの逆引き（`Catalog::by_url`・`Catalog::page_urls`）は 1 対 1 が前提。
    let mut id_of_url: BTreeMap<String, &str> = BTreeMap::new();
    let mut page_of_url: BTreeMap<String, PageName> = BTreeMap::new();
    for (raw_id, value) in table {
        // 綴りが 2 形のどちらでもなければ、置き場つきで落とす。素の id の失敗は綴りしか
        // 持たないので外へ出さない。
        let id = EntryId::parse(raw_id)
            .map_err(|_| malformed(CATALOG_FILE, format!("項目 id の形が違う: {raw_id}")))?;
        let place = format!("[entry] の {raw_id}");
        let item = value.as_table().ok_or_else(|| {
            malformed(
                CATALOG_FILE,
                format!("{place}: 1 行のインラインテーブルでない"),
            )
        })?;
        reject_unknown_keys(item, &ENTRY_COLUMNS, &place, CATALOG_FILE)?;

        let written_page = string_field(item, &place, "page", CATALOG_FILE)?;
        let page = id.page();
        if written_page != page.as_str() {
            return Err(malformed(
                CATALOG_FILE,
                format!(
                    "{place}: page の値 {written_page} が id のページ {} と食い違う",
                    page.as_str()
                ),
            ));
        }

        let entry = CatalogEntry {
            id: id.clone(),
            page,
            title: string_field(item, &place, "title", CATALOG_FILE)?,
            category: string_field(item, &place, "category", CATALOG_FILE)?,
            versions: string_array_field(item, &place, "versions", CATALOG_FILE)?,
            hash: string_field(item, &place, "hash", CATALOG_FILE)?,
            url: string_field(item, &place, "url", CATALOG_FILE)?,
        };
        if let Some(previous) = id_of_url.insert(entry.url.clone(), raw_id) {
            return Err(malformed(
                CATALOG_FILE,
                format!(
                    "同じ URL が 2 つの項目 id に現れる: {}（{previous}・{raw_id}）",
                    entry.url
                ),
            ));
        }
        let page_url = page_url(&entry.url).to_owned();
        if let Some(previous) = page_of_url.insert(page_url.clone(), entry.page.clone())
            && previous != entry.page
        {
            return Err(malformed(
                CATALOG_FILE,
                format!(
                    "別のページが同じページ URL に畳まれる: {page_url}（{}・{}）",
                    previous.as_str(),
                    entry.page.as_str()
                ),
            ));
        }
        entries.insert(id, entry);
    }
    Ok(entries)
}

/// 整数の欄。
fn integer_field(table: &toml::Table, place: &str, key: &str) -> Result<i64, SurveyError> {
    field(table, place, key, CATALOG_FILE)?
        .as_integer()
        .ok_or_else(|| malformed(CATALOG_FILE, format!("{place}: 欄 {key} が整数でない")))
}

/// 形の版の欄。負の数も大きすぎる数も入らない。
fn format_field(table: &toml::Table, place: &str, key: &str) -> Result<u32, SurveyError> {
    let value = integer_field(table, place, key)?;
    u32::try_from(value).map_err(|_| {
        malformed(
            CATALOG_FILE,
            format!("{place}: 欄 {key} の値 {value} は形の版にならない"),
        )
    })
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
