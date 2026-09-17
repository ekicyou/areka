//! 3 文書の骨組みを読むときに使う共通の道具——欄の取り出し・語彙の検査・失敗の作り方。
//!
//! `parse.rs` から割った（タスク 1.2 の申し送り・設計「Testing Strategy → 1,000 行の
//! 番人」）。文書ごとの読み手 3 組だけで 740 行あり、そこへ共通の道具 270 行を
//! 同居させると 1 ファイルが 1,000 行を越える。
//! **中身は移しただけで、1 行も変えていない**——見える範囲を `documents` 木の中へ
//! 広げただけである。公開名 `document_file` は `parse.rs` が再輸出して保つ
//! （`derive.rs` が `super::parse::document_file` で引いている）。
//!
//! # 繕わない・行番号は添えない
//!
//! 落とす本文には**文書名と表の鍵**を添え、**行番号は添えない**（設計 Error
//! Handling）——囲みを連結した本文の行番号は、書き手が見ている Markdown の行と
//! 一致しない。

use crate::error::SurveyError;
use crate::model::{Domain, EntryId, parse_theme};

use super::parse::{DOCUMENT_DIR, toml_blocks};

/// 失敗の本文に添える文書の置き場。
pub(super) fn document_file(name: &str) -> String {
    format!("{DOCUMENT_DIR}/{name}")
}

/// 文書の形が違うことを告げる失敗。置き場を必ず添える。
pub(super) fn malformed(file: &str, reason: impl Into<String>) -> SurveyError {
    SurveyError::TomlParse {
        path: file.to_owned(),
        reason: reason.into(),
    }
}

/// 語彙に無い値。文書名・表の鍵・欄名・書かれていた綴りを添える。
pub(super) fn bad_vocabulary(
    file: &str,
    place: &str,
    field: &'static str,
    value: &str,
) -> SurveyError {
    SurveyError::BadVocabulary {
        file: file.to_owned(),
        id: place.to_owned(),
        field,
        value: value.to_owned(),
    }
}

/// 失敗の本文に写す該当箇所の長さの上限（文字数）。
pub(super) const SNIPPET_CHARS: usize = 80;

/// 囲みを連結して 1 度だけ読む。
///
/// 失敗の本文に `toml` の**行番号を写さない**——連結した本文の行番号は書き手の見て
/// いる Markdown の行と一致しない。代わりに該当箇所の綴りを添える。`toml` の言う
/// 「duplicate key」だけでは**どの鍵が重複したか**が分からないからである。
pub(super) fn read_blocks(markdown: &str, file: &str) -> Result<toml::Table, SurveyError> {
    let joined = toml_blocks(markdown).join("\n");
    joined.parse::<toml::Table>().map_err(|err| {
        let at = err
            .span()
            .and_then(|span| joined.get(span))
            .map(str::trim)
            .filter(|snippet| !snippet.is_empty())
            .map(|snippet| {
                let short: String = snippet.chars().take(SNIPPET_CHARS).collect();
                format!("・該当箇所 {short}")
            })
            .unwrap_or_default();
        malformed(
            file,
            format!("囲みを連結した TOML が読めない: {}{at}", err.message()),
        )
    })
}

/// 表として取り出す。
pub(super) fn as_table<'a>(
    value: &'a toml::Value,
    place: &str,
    file: &str,
) -> Result<&'a toml::Table, SurveyError> {
    value
        .as_table()
        .ok_or_else(|| malformed(file, format!("{place} が表でない")))
}

/// 必ず在る表を取り出す。無ければ表の鍵を名指して落ちる。
pub(super) fn required_table<'a>(
    parent: &'a toml::Table,
    key: &str,
    place: &str,
    file: &str,
) -> Result<&'a toml::Table, SurveyError> {
    let value = parent
        .get(key)
        .ok_or_else(|| malformed(file, format!("{place} が無い")))?;
    as_table(value, place, file)
}

/// 配列表の各行。書かれていなければ 0 行として扱う。
///
/// TOML は配列表の見出しだけを空で置けないので（`rank = []` と書くと後続の
/// `[[rank]]` が「値を上書きできない」で落ちる）、**欄の欠落＝0 行**とする。
pub(super) fn array_of_tables<'a>(
    root: &'a toml::Table,
    key: &str,
    file: &str,
) -> Result<Vec<&'a toml::Table>, SurveyError> {
    let place = format!("[[{key}]]");
    let Some(value) = root.get(key) else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| malformed(file, format!("{place} が配列でない")))?;
    let mut tables = Vec::with_capacity(array.len());
    for element in array {
        tables.push(as_table(element, &place, file)?);
    }
    Ok(tables)
}

/// 知らない欄が混じっていないことを確かめる。
pub(super) fn reject_unknown_keys(
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

/// この形では書けない欄が書かれていないことを確かめる。
pub(super) fn reject_present(
    table: &toml::Table,
    place: &str,
    file: &str,
    forbidden: &[&str],
) -> Result<(), SurveyError> {
    for key in forbidden {
        if table.contains_key(*key) {
            return Err(malformed(
                file,
                format!("{place}: この形の行に欄 {key} は書けない"),
            ));
        }
    }
    Ok(())
}

/// 欄を 1 つ取り出す。無ければ落ちる。
pub(super) fn field<'a>(
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
pub(super) fn string_field(
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

/// 真偽の欄。書かれていなければ偽。
pub(super) fn bool_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<bool, SurveyError> {
    match table.get(key) {
        None => Ok(false),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| malformed(file, format!("{place}: 欄 {key} が真偽でない"))),
    }
}

/// 0 以上の整数の欄。
pub(super) fn usize_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<usize, SurveyError> {
    let raw = field(table, place, key, file)?
        .as_integer()
        .ok_or_else(|| malformed(file, format!("{place}: 欄 {key} が整数でない")))?;
    usize::try_from(raw).map_err(|_| malformed(file, format!("{place}: 欄 {key} が負")))
}

/// 0 以上の整数の欄。書かれていなければ 0（判定が数え直す欄にだけ使う）。
pub(super) fn optional_usize_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<usize, SurveyError> {
    if table.contains_key(key) {
        usize_field(table, place, key, file)
    } else {
        Ok(0)
    }
}

/// 文字列の配列の欄。要素の 1 つでも文字列でなければ落ちる。
pub(super) fn string_array_field(
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

/// 項目 id の配列の欄。
pub(super) fn id_array_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<Vec<EntryId>, SurveyError> {
    let raws = string_array_field(table, place, key, file)?;
    let mut ids = Vec::with_capacity(raws.len());
    for raw in &raws {
        let id = EntryId::parse(raw).map_err(|err| {
            malformed(
                file,
                format!("{place}: 欄 {key} が項目 id の形でない（{err}）"),
            )
        })?;
        ids.push(id);
    }
    Ok(ids)
}

/// ドメインの配列の欄。
pub(super) fn domain_array_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<Vec<Domain>, SurveyError> {
    let raws = string_array_field(table, place, key, file)?;
    let mut domains = Vec::with_capacity(raws.len());
    for raw in &raws {
        domains.push(Domain::parse(raw).map_err(|err| err.at(file, place))?);
    }
    Ok(domains)
}

/// テーマの配列の欄（要件 4.4 の 8 テーマ）。
pub(super) fn theme_array_field(
    table: &toml::Table,
    place: &str,
    key: &str,
    file: &str,
) -> Result<Vec<String>, SurveyError> {
    let raws = string_array_field(table, place, key, file)?;
    let mut themes = Vec::with_capacity(raws.len());
    for raw in &raws {
        let theme = parse_theme(raw).map_err(|err| err.at(file, place))?;
        themes.push(theme.to_owned());
    }
    Ok(themes)
}
