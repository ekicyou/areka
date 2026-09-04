//! 台帳の本文 → 台帳（欄と語彙の検証つき）。
//!
//! 読み取りは `toml` に任せる（設計「境界の要点」1 つ目。書き出しだけが自前）。
//! ここは純粋層で、**ファイルには触らない**——本文の文字列と、どのドメインの台帳を
//! 読んでいるかだけを受け取る（要件 6.2）。
//!
//! # 繕わない
//!
//! 台帳は人が手で書く文書なので、形が違えば**黙って直さずに落とす**（設計 Error
//! Handling の「契約違反（データ）」）。落とすのは次の場合で、いずれもどのファイルの
//! どの項目かを本文に載せる（要件 6.10）。
//!
//! - 状態の綴りが 7 語彙のいずれでもない（要件 2.2）
//! - テーマ名の綴りが 8 テーマのいずれでもない（要件 4.4）
//! - 関連の種別が 6 種のいずれでもない（要件 4.3）
//! - `alias_of` が `alias` 以外の行にある／`alias` の行に無い（要件 2.4・付録 A.2）
//! - 付録 A.2 が必須とする欄が無い・型が違う
//! - **知らない欄がある**
//! - 項目 id が 2 形のどちらでもない（要件 1.9）
//! - 前置きのドメイン名が読んでいるファイルのドメインと食い違う
//!
//! 「知らない欄」を落とすのは要件 2.3 と 6.9 がここに載っているからである。台帳に
//! 証拠の欄は無く、未分類件数を宣言する欄も無い。黙って読み飛ばすと、手で書いた
//! `evidence = […]` や `unclassified_count = 42` が**書いたつもりのまま**検査を
//! 素通りし、持ち主は数が合わない理由に永久に気づけない。
//!
//! # 本文に現れた順
//!
//! `entries` は `BTreeMap` なので id の byte 昇順に並び替わり、**本文の順は失われる**。
//! それを知っているのは行を見て切り分ける [`blocks::split`] だけなので、読み取りは
//! 必ずそこを通し、返った塊の並びを `file_order` にする（設計 D-12）。ついでに
//! 「切り分けた id の集合が `toml` の鍵の集合と一致する」較正も相続する。
//!
//! # 失敗に添える場所
//!
//! ドメインが分かるので、置き場は `<ドメイン>` を伏せずに綴れる（[`ledger_file`]）。
//! `io::paths::ledger_path` は呼ばない——あれはワークスペース根から組み立てた絶対
//! パスを返すので、失敗の本文が計算機ごとに変わってしまう（要件 6.1 の決定性）。
//! 純粋層が入出力層に触らない形も保てる。綴りの正本は `Domain::as_key` 1 つきりで、
//! そこは両者で共有している。

use std::collections::BTreeMap;

use super::blocks;
use super::{Ledger, LedgerEntry};
use crate::error::SurveyError;
use crate::model::{Domain, EntryId, Link, LinkKind, PageName, Status, parse_theme};

/// 台帳を置くディレクトリ（`io::paths::ledger_path` と同じ場所・ワークスペース根から）。
const LEDGER_DIR: &str = "doc/ukadoc-coverage/ledger";

/// 前置きの表の名前。
const LEDGER_TABLE: &str = "ledger";

/// 項目を置く表の名前。
const ENTRY_TABLE: &str = "entry";

/// 前置きに置いてよい欄のすべて（要件付録 A.1）。
///
/// ここに `unclassified_count` のような件数の欄は**無い**。未分類の件数は報告側の
/// 分布を正とし、台帳に宣言値を持たせない（要件 6.9）。
const LEDGER_FIELDS: [&str; 2] = ["domain", "pages"];

/// 項目に置いてよい欄のすべて（要件付録 A.2 の表）。
///
/// ここに `evidence` は**無い**。実装済みの根拠はソース側の doc コメントにあり、
/// 台帳は人手だけが書く文書に保つ（要件 2.3）。
const ENTRY_COLUMNS: [&str; 9] = [
    "status",
    "introduced",
    "alias_of",
    "supersedes",
    "owner",
    "priority",
    "values",
    "links",
    "note",
];

/// 関連の 1 要素に置いてよい欄のすべて（要件付録 A.2）。
const LINK_FIELDS: [&str; 2] = ["kind", "to"];

/// 前置きの失敗に添える場所。項目 id が無いので、代わりに表の名前を書く。
const PROLOGUE_PLACE: &str = "[ledger]";

/// 台帳の本文を読む（要件 2.1〜2.4・6.9）。
///
/// `domain` は読んでいるファイルのドメイン（＝ファイル名）である。前置きの
/// `domain` がこれと食い違えば、どちらの綴りも添えて落とす。
pub fn read(text: &str, domain: Domain) -> Result<Ledger, SurveyError> {
    let file = ledger_file(domain);

    // 先に本文全体を読む。読めない本文はここで、ドメインまで綴った置き場を添えて
    // 落とす（切り分けはドメインを知らないので `<ドメイン>` としか書けない）。
    let root: toml::Table = text
        .parse()
        .map_err(|err| malformed(&file, format!("TOML として読めない: {err}")))?;

    // 本文に現れた順は `toml` の表からは取れない（鍵で並び替えられている）。それを
    // 知っているのは切り分けだけなので、必ずここを通す（設計 D-12）。
    let (_, blocks) = blocks::split(text)?;
    let file_order: Vec<EntryId> = blocks.into_iter().map(|block| block.id).collect();

    let (declared, pages) = read_prologue(&root, &file)?;
    if declared != domain {
        return Err(malformed(
            &file,
            format!(
                "{PROLOGUE_PLACE}: domain の値 {} がファイルのドメイン {} と食い違う",
                declared.as_key(),
                domain.as_key()
            ),
        ));
    }

    Ok(Ledger {
        domain,
        pages,
        entries: read_entries(&root, &file)?,
        file_order,
    })
}

/// 失敗の本文に添える台帳の置き場。
fn ledger_file(domain: Domain) -> String {
    format!("{LEDGER_DIR}/{}.toml", domain.as_key())
}

/// 台帳の形が違うことを告げる失敗。置き場を必ず添える（要件 6.12）。
fn malformed(file: &str, reason: impl Into<String>) -> SurveyError {
    SurveyError::TomlParse {
        path: file.to_owned(),
        reason: reason.into(),
    }
}

/// 前置きを読む（要件付録 A.1）。
fn read_prologue(root: &toml::Table, file: &str) -> Result<(Domain, Vec<PageName>), SurveyError> {
    let table = root
        .get(LEDGER_TABLE)
        .ok_or_else(|| malformed(file, format!("{PROLOGUE_PLACE} が無い")))?
        .as_table()
        .ok_or_else(|| malformed(file, format!("{PROLOGUE_PLACE} が表でない")))?;
    reject_unknown_keys(table, &LEDGER_FIELDS, PROLOGUE_PLACE, file)?;

    let raw = string_field(table, PROLOGUE_PLACE, "domain", file)?;
    // 語彙の失敗には必ず場所を添える（要件 6.10。包み忘れるとどの台帳かが消える）。
    let domain = Domain::parse(&raw).map_err(|err| err.at(file, PROLOGUE_PLACE))?;

    let pages = string_array_field(table, PROLOGUE_PLACE, "pages", file)?
        .into_iter()
        .map(PageName::new)
        .collect();
    Ok((domain, pages))
}

/// 項目をすべて読む。
///
/// `[entry]` が無い台帳は項目 0 件として扱う（初期生成の前の、前置きだけの台帳）。
fn read_entries(
    root: &toml::Table,
    file: &str,
) -> Result<BTreeMap<EntryId, LedgerEntry>, SurveyError> {
    let table = match root.get(ENTRY_TABLE) {
        None => return Ok(BTreeMap::new()),
        Some(value) => value
            .as_table()
            .ok_or_else(|| malformed(file, format!("[{ENTRY_TABLE}] が表でない")))?,
    };

    let mut entries: BTreeMap<EntryId, LedgerEntry> = BTreeMap::new();
    for (raw_id, value) in table {
        // 綴りが 2 形のどちらでもなければ、ここで `BadEntryId` になる（要件 1.9）。
        let id = EntryId::parse(raw_id)?;
        let place = place_of(raw_id);
        let item = value
            .as_table()
            .ok_or_else(|| malformed(file, format!("{place}: 欄を並べた表でない")))?;
        entries.insert(id.clone(), read_entry(id, item, &place, file)?);
    }
    Ok(entries)
}

/// 項目 1 つ分を読む（要件付録 A.2 の表）。
fn read_entry(
    id: EntryId,
    item: &toml::Table,
    place: &str,
    file: &str,
) -> Result<LedgerEntry, SurveyError> {
    reject_unknown_keys(item, &ENTRY_COLUMNS, place, file)?;

    let raw_status = string_field(item, place, "status", file)?;
    let status = Status::parse(&raw_status).map_err(|err| err.at(file, id.as_str()))?;
    let alias_of = read_alias_of(item, place, file)?;
    reject_alias_mismatch(status, alias_of.as_ref(), place, file)?;

    Ok(LedgerEntry {
        status,
        introduced: string_field(item, place, "introduced", file)?,
        alias_of,
        supersedes: read_supersedes(item, place, file)?,
        owner: string_field(item, place, "owner", file)?,
        priority: string_field(item, place, "priority", file)?,
        values: read_values(item, place, file, &id)?,
        links: read_links(item, place, file, &id)?,
        note: string_field(item, place, "note", file)?,
        id,
    })
}

/// 正典側の id（任意の欄）。書かれていなければ `None`。
fn read_alias_of(
    item: &toml::Table,
    place: &str,
    file: &str,
) -> Result<Option<EntryId>, SurveyError> {
    if !item.contains_key("alias_of") {
        return Ok(None);
    }
    let raw = string_field(item, place, "alias_of", file)?;
    Ok(Some(reference_id(&raw, place, "alias_of", file)?))
}

/// 別名の写像と状態の食い違いを落とす（要件 2.4・付録 A.2）。
///
/// 付録 A.2 は `alias_of` を「`status = "alias"` のとき必須・それ以外は書かない」と
/// 定める。どちらの向きも落とす——片側だけを守ると、別名でない行に書かれた写像先が
/// 黙って無視されるか、別名の行の写像先が黙って空になる。
fn reject_alias_mismatch(
    status: Status,
    alias_of: Option<&EntryId>,
    place: &str,
    file: &str,
) -> Result<(), SurveyError> {
    match (status, alias_of) {
        (Status::Alias, None) => Err(malformed(
            file,
            format!("{place}: status = \"alias\" の項目には alias_of が要る"),
        )),
        (other, Some(_)) if other != Status::Alias => Err(malformed(
            file,
            format!(
                "{place}: alias_of は status = \"alias\" の項目にだけ書ける（status = \"{}\"）",
                other.as_key()
            ),
        )),
        _ => Ok(()),
    }
}

/// 置き換えた旧 id の一覧（任意の欄）。書かれていなければ空。
fn read_supersedes(
    item: &toml::Table,
    place: &str,
    file: &str,
) -> Result<Vec<EntryId>, SurveyError> {
    if !item.contains_key("supersedes") {
        return Ok(Vec::new());
    }
    let raws = string_array_field(item, place, "supersedes", file)?;
    let mut ids = Vec::with_capacity(raws.len());
    for raw in &raws {
        ids.push(reference_id(raw, place, "supersedes", file)?);
    }
    Ok(ids)
}

/// 伺からしさのテーマ（要件 4.4 の 8 つ）。
///
/// 持つのは凍結された側の綴りである。前後の空白を落としたり近い綴りへ寄せたりは
/// しない——「気配」と「気配り」は片方が他方の接頭辞なので、緩めると取り違える。
fn read_values(
    item: &toml::Table,
    place: &str,
    file: &str,
    id: &EntryId,
) -> Result<Vec<String>, SurveyError> {
    let raws = string_array_field(item, place, "values", file)?;
    let mut themes = Vec::with_capacity(raws.len());
    for raw in &raws {
        let theme = parse_theme(raw).map_err(|err| err.at(file, id.as_str()))?;
        themes.push(theme.to_owned());
    }
    Ok(themes)
}

/// 関連（要件 4.3 の 6 種と相手 id の対）。
fn read_links(
    item: &toml::Table,
    place: &str,
    file: &str,
    id: &EntryId,
) -> Result<Vec<Link>, SurveyError> {
    let array = field(item, place, "links", file)?
        .as_array()
        .ok_or_else(|| malformed(file, format!("{place}: 欄 links が配列でない")))?;

    let mut links = Vec::with_capacity(array.len());
    for element in array {
        let table = element
            .as_table()
            .ok_or_else(|| malformed(file, format!("{place}: 欄 links に表でない要素がある")))?;
        let where_at = format!("{place} の links");
        reject_unknown_keys(table, &LINK_FIELDS, &where_at, file)?;

        let raw_kind = string_field(table, &where_at, "kind", file)?;
        let kind = LinkKind::parse(&raw_kind).map_err(|err| err.at(file, id.as_str()))?;
        let raw_to = string_field(table, &where_at, "to", file)?;
        links.push(Link {
            kind,
            to: reference_id(&raw_to, place, "to", file)?,
        });
    }
    Ok(links)
}

/// 欄に書かれた相手 id を読む（要件 1.9）。
///
/// 素の [`SurveyError::BadEntryId`] は綴りしか持たないので、**どの項目のどの欄**で
/// 見つけたかを添え直す（要件 6.10）。手書きの台帳では、綴りだけ告げられても
/// どの行を直せばよいか分からない。
fn reference_id(raw: &str, place: &str, key: &str, file: &str) -> Result<EntryId, SurveyError> {
    EntryId::parse(raw).map_err(|err| {
        malformed(
            file,
            format!("{place}: 欄 {key} が項目 id の形でない（{err}）"),
        )
    })
}

/// 項目の失敗に添える場所。項目 id を必ず含む（要件 6.10）。
fn place_of(raw_id: &str) -> String {
    format!("[{ENTRY_TABLE}.\"{raw_id}\"]")
}

/// 知らない欄が混じっていないことを確かめる（要件 2.3・6.9 はここに載っている）。
fn reject_unknown_keys(
    table: &toml::Table,
    allowed: &[&str],
    place: &str,
    file: &str,
) -> Result<(), SurveyError> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(malformed(file, format!("{place}: 知らない欄 {key}")));
        }
    }
    Ok(())
}

/// 欄を 1 つ取り出す。無ければ落ちる。
fn field<'a>(
    table: &'a toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<&'a toml::Value, SurveyError> {
    table
        .get(key)
        .ok_or_else(|| malformed(file, format!("{place}: 欄 {key} が無い")))
}

/// 文字列の欄。
fn string_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<String, SurveyError> {
    field(table, place, key, file)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| malformed(file, format!("{place}: 欄 {key} が文字列でない")))
}

/// 文字列の配列の欄。要素の 1 つでも文字列でなければ落ちる。
fn string_array_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<Vec<String>, SurveyError> {
    let array = field(table, place, key, file)?
        .as_array()
        .ok_or_else(|| malformed(file, format!("{place}: 欄 {key} が配列でない")))?;
    let mut values = Vec::with_capacity(array.len());
    for element in array {
        let value = element.as_str().ok_or_else(|| {
            malformed(file, format!("{place}: 欄 {key} に文字列でない要素がある"))
        })?;
        values.push(value.to_owned());
    }
    Ok(values)
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
