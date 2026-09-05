//! スナップショット JSON → `SnapshotDoc`。
//!
//! 場所は環境変数 `AREKA_UKADOC_SNAPSHOT` が最優先で、無ければ `APPDATA` から
//! 既定の場所を組み立てる（要件 1.7・9.7・設計 D-7）。
//!
//! # 常時走るテストはここを通らない（要件 6.2）
//!
//! このモジュールは `pub(crate)` である。常時走る整合検査（`tests/consistency.rs`）は
//! ライブラリの外にある別クレートなので、`pub(crate)` の中身へは**コンパイル時点で
//! 手が届かない**。「スナップショットの無い環境でも整合検査が赤にならない」ことを
//! 申し合わせではなく型検査で守るための形である（設計 Testing Strategy 19 の
//! 「`io::snapshot` を `cli` からだけ引く形にする」）。`cli` はライブラリの中にあるので
//! そのまま引ける。
//!
//! # JSON の読み方（設計 D-2）
//!
//! `serde` の派生機能は使わない。[`serde_json::Value`] を手で辿って値へ写す。
//! `serde` を直接の依存に加えずに済ませるためで、写しの規則がすべてこのファイルの
//! 中に見える形になる。最上位の鍵は `version` / `generatedAt` / `entries`、
//! 各 entry は `id` `title` `source` `category` `content` `url`（要件付録 B 手順 1）。
//!
//! # 黙って失敗しない（要件 1.8）
//!
//! 読めない（存在しない・壊れている）ときは [`SurveyError::SnapshotUnreadable`] を、
//! JSON としては読めるが形が違うときは [`SurveyError::SnapshotShape`] を返す。
//! 前者には**探した絶対パス**と理由を、後者にはどの鍵のどこが違うかを必ず載せる。
//! どちらの場合も既存のカタログには 1 バイトも触れない——このモジュールは読むだけで、
//! 書き出しは呼び出し側が成功した値を受け取ってから始める。
//!
//! `source` が `ukadoc` 以外の entry もそのまま持って返す。絞り込みは正典カタログを
//! 建てる側の判断であり（要件 1.4）、入出力層は判断を持たない（設計 Architecture）。

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::SurveyError;

/// スナップショットの場所を指す環境変数（要件 1.7・9.7 の `AREKA_` 冠）。
pub const SNAPSHOT_ENV: &str = "AREKA_UKADOC_SNAPSHOT";

/// 既定の場所を組み立てる元になる環境変数。
const APPDATA_ENV: &str = "APPDATA";

/// 既定の場所の、`%APPDATA%` から先の綴り（設計 D-7）。
const DEFAULT_TAIL: [&str; 5] = [
    "npm",
    "node_modules",
    "ukagaka-doc-mcp",
    "data",
    "index.json",
];

/// 提供パッケージの名前や版が読めなかったときに記録する値（設計 D-7）。
pub const UNKNOWN: &str = "unknown";

/// スナップショット 1 つ分。カタログ冒頭に記録する情報（要件 1.6）を含む。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDoc {
    /// スナップショットの版（最上位の `version`）。
    pub version: i64,
    /// 生成日時（最上位の `generatedAt`）。綴りが JSON と違う唯一の欄である。
    pub generated_at: String,
    /// 全 entry。`source` による絞り込みはここでは行わない（要件 1.4）。
    pub entries: Vec<RawEntry>,
    /// 提供パッケージの名前（`package.json` の `name`）。
    pub package: String,
    /// 提供パッケージの版（`package.json` の `version`）。読めなければ [`UNKNOWN`]。
    pub package_version: String,
}

/// スナップショットの entry 1 件。写しであって、判断は一切入っていない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub id: String,
    pub title: String,
    pub source: String,
    pub category: String,
    pub content: String,
    pub url: String,
}

/// entry が持つ欄の綴り（要件付録 B 手順 1）。
const ENTRY_FIELDS: [&str; 6] = ["id", "title", "source", "category", "content", "url"];

/// スナップショットの場所。環境変数 `AREKA_UKADOC_SNAPSHOT` が既定より優先する。
///
/// 環境変数が無いときは `APPDATA` から既定の場所を組み立てる。`APPDATA` も無ければ
/// 組み立てようが無いので、どの環境変数が足りないかを告げて失敗する（設計 D-7）。
pub fn default_path() -> Result<PathBuf, SurveyError> {
    if let Some(given) = std::env::var_os(SNAPSHOT_ENV)
        && !given.is_empty()
    {
        return Ok(PathBuf::from(given));
    }
    match std::env::var_os(APPDATA_ENV) {
        Some(appdata) if !appdata.is_empty() => Ok(default_path_from_appdata(Path::new(&appdata))),
        _ => Err(SurveyError::MissingEnv { name: APPDATA_ENV }),
    }
}

/// 与えられた `%APPDATA%` から既定の場所を組み立てる（設計 D-7）。
///
/// 環境変数を読まないので、テストから逐語で確かめられる。
fn default_path_from_appdata(appdata: &Path) -> PathBuf {
    let mut path = appdata.to_path_buf();
    for part in DEFAULT_TAIL {
        path.push(part);
    }
    path
}

/// スナップショットを読んで [`SnapshotDoc`] にする。
///
/// 読めないときの本文には**探した絶対パス**を載せる（要件 1.8）。相対パスを渡されても
/// 本文に出るのは絶対パスで、どこを見に行ったのかが後から辿れる。
pub fn load(path: &Path) -> Result<SnapshotDoc, SurveyError> {
    let shown = absolutize(path, std::env::current_dir().ok().as_deref());
    let text = std::fs::read_to_string(path).map_err(|err| unreadable(&shown, &err.to_string()))?;
    let (package, package_version) = package_info(path);
    parse(&text, &shown, &package, &package_version)
}

/// 本文に載せる絶対パスを組み立てる。
///
/// すでに絶対パスならその綴りのまま。相対パスなら作業ディレクトリに繋ぐ。作業
/// ディレクトリすら取れないときは、渡された綴りをそのまま出す——絶対パスに直せない
/// ことを黙って隠すより、見に行った綴りが分かるほうがよい。
fn absolutize(path: &Path, current_dir: Option<&Path>) -> String {
    if path.is_absolute() {
        return path.display().to_string();
    }
    match current_dir {
        Some(dir) => dir.join(path).display().to_string(),
        None => path.display().to_string(),
    }
}

/// 提供パッケージの名前と版を読む（要件 1.6・設計 D-7）。
///
/// 読むのはスナップショットの **2 つ上**にある `package.json`。読めないときは
/// 名前も版も [`UNKNOWN`] にして、その旨を標準エラーへ 1 行出すだけで先へ進む。
/// 環境変数で任意の場所を指したときに再生成そのものを止めないためである。
fn package_info(snapshot: &Path) -> (String, String) {
    let unknown = || (UNKNOWN.to_owned(), UNKNOWN.to_owned());
    let Some(manifest) = package_json_path(snapshot) else {
        warn_unknown_package(
            &snapshot.display().to_string(),
            "2 つ上のディレクトリが無い",
        );
        return unknown();
    };
    let shown = manifest.display().to_string();
    let text = match std::fs::read_to_string(&manifest) {
        Ok(text) => text,
        Err(err) => {
            warn_unknown_package(&shown, &err.to_string());
            return unknown();
        }
    };
    match package_fields(&text) {
        Some(pair) => pair,
        None => {
            warn_unknown_package(&shown, "name か version の欄が読めない");
            unknown()
        }
    }
}

/// スナップショットの 2 つ上にある `package.json` の場所。
fn package_json_path(snapshot: &Path) -> Option<PathBuf> {
    snapshot
        .parent()
        .and_then(Path::parent)
        .map(|dir| dir.join("package.json"))
}

/// `package.json` の本文から名前と版を取り出す。
///
/// 版が無い（または文字列でない）ときは諦める。名前だけが無いときは [`UNKNOWN`] を
/// 名前にして版は活かす——記録したいのは版だからである（要件 1.6）。
fn package_fields(text: &str) -> Option<(String, String)> {
    let value: Value = serde_json::from_str(text).ok()?;
    let table = value.as_object()?;
    let version = table.get("version")?.as_str()?.to_owned();
    let name = table
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(UNKNOWN)
        .to_owned();
    Some((name, version))
}

/// 提供パッケージの版が読めなかったことを標準エラーへ 1 行だけ告げる（設計 D-7）。
///
/// `tracing` は使わない。この道具は調査用の実行ファイルで、記録の宛先は標準エラーに
/// 一本化してある（設計 Error Handling）。
fn warn_unknown_package(shown: &str, reason: &str) {
    eprintln!("警告: 提供パッケージの版が読めないので {UNKNOWN} と記録する: {shown}（{reason}）");
}

/// JSON の本文を [`SnapshotDoc`] へ写す（設計 D-2）。
///
/// `shown` は本文に載せる**探した絶対パス**。提供パッケージの名前と版は呼び出し側が
/// 決めて渡す——ここはファイルに触らない純粋な写しなので、テストから文字列だけで
/// 確かめられる。
fn parse(
    text: &str,
    shown: &str,
    package: &str,
    package_version: &str,
) -> Result<SnapshotDoc, SurveyError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|err| unreadable(shown, &format!("JSON として読めない: {err}")))?;
    let root = value
        .as_object()
        .ok_or_else(|| shape(format!("最上位が表ではない（{}）", kind(&value))))?;

    let version = match root.get("version") {
        None => return Err(shape("最上位に version が無い")),
        Some(found) => found.as_i64().ok_or_else(|| {
            shape(format!(
                "最上位の version が整数ではない（{}）",
                kind(found)
            ))
        })?,
    };
    let generated_at = root_string(root, "generatedAt")?;
    let entries_value = root
        .get("entries")
        .ok_or_else(|| shape("最上位に entries が無い"))?;
    let list = entries_value.as_array().ok_or_else(|| {
        shape(format!(
            "最上位の entries が配列ではない（{}）",
            kind(entries_value)
        ))
    })?;

    let mut entries = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        entries.push(raw_entry(item, index)?);
    }

    Ok(SnapshotDoc {
        version,
        generated_at,
        entries,
        package: package.to_owned(),
        package_version: package_version.to_owned(),
    })
}

/// 最上位の文字列の欄を取り出す。
fn root_string(root: &Map<String, Value>, key: &str) -> Result<String, SurveyError> {
    match root.get(key) {
        None => Err(shape(format!("最上位に {key} が無い"))),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(other) => Err(shape(format!(
            "最上位の {key} が文字列ではない（{}）",
            kind(other)
        ))),
    }
}

/// entry 1 件を写す。どの位置のどの欄が違うかを本文に載せる。
fn raw_entry(item: &Value, index: usize) -> Result<RawEntry, SurveyError> {
    let table = item
        .as_object()
        .ok_or_else(|| shape(format!("entries[{index}] が表ではない（{}）", kind(item))))?;
    Ok(RawEntry {
        id: entry_string(table, index, ENTRY_FIELDS[0])?,
        title: entry_string(table, index, ENTRY_FIELDS[1])?,
        source: entry_string(table, index, ENTRY_FIELDS[2])?,
        category: entry_string(table, index, ENTRY_FIELDS[3])?,
        content: entry_string(table, index, ENTRY_FIELDS[4])?,
        url: entry_string(table, index, ENTRY_FIELDS[5])?,
    })
}

/// entry の文字列の欄を取り出す。
fn entry_string(
    table: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<String, SurveyError> {
    match table.get(key) {
        None => Err(shape(format!("entries[{index}] に {key} が無い"))),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(other) => Err(shape(format!(
            "entries[{index}] の {key} が文字列ではない（{}）",
            kind(other)
        ))),
    }
}

/// 値の種類を日本語で言う（形の違いの本文に添える）。
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "空",
        Value::Bool(_) => "真偽値",
        Value::Number(_) => "数",
        Value::String(_) => "文字列",
        Value::Array(_) => "配列",
        Value::Object(_) => "表",
    }
}

/// 探した絶対パスと理由を載せた「読めない」失敗（要件 1.8）。
fn unreadable(shown: &str, reason: &str) -> SurveyError {
    SurveyError::SnapshotUnreadable {
        path: shown.to_owned(),
        reason: reason.to_owned(),
    }
}

/// どこがどう違うかを載せた「形が違う」失敗（要件 1.8）。
fn shape(detail: impl Into<String>) -> SurveyError {
    SurveyError::SnapshotShape {
        detail: detail.into(),
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
