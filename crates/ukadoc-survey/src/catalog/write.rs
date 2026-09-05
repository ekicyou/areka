//! カタログ → `catalog.toml` の本文。
//!
//! ここは純粋層である。**ファイルには触らない**——本文を組み立てて文字列で返すだけで、
//! どこへ書くかは入出力層（`io::files::write_lf`）と生成の副手続きの仕事である
//! （要件 6.2・設計 Architecture）。
//!
//! 形は設計「Data Models」の逐語で、次の 4 つの塊が必ずこの順で並ぶ。
//!
//! 1. 手で編集しないことと再生成の仕方を告げる注意書き 2 行。
//! 2. `[snapshot]` と冒頭の情報 8 欄（要件 1.6・設計 D-9）。
//! 3. `[entry]`。
//! 4. 項目 1 件につきちょうど 1 行のインラインテーブル（要件 1.1）、id の byte 昇順。
//!
//! # 1 バイトの安定（要件 1.5）
//!
//! 並びも区切りの空白も欄の順もここで決め打ちする。並びは `Catalog::entries` が
//! `BTreeMap` なので id の byte 昇順に定まり（設計 D-9。実測で id はすべて ASCII）、
//! 欄の順は [`ENTRY_COLUMNS`] の 1 か所だけが決める。同じカタログを 2 回書き出せば
//! 同じ本文になる——分岐も並べ替えも通らないからである。
//!
//! # 逃がし
//!
//! 値はすべて [`crate::tomlout`] を通す。`toml` の書き出しは使わない（設計
//! 「境界の要点」1 つ目。実測で `toml` 1.1.4 は逆斜線を含む文字列を単引用符の素の
//! 文字列で書き、要件付録 A.3 が凍結した書き方と一致しない）。

use super::{Catalog, CatalogEntry, SnapshotMeta};
use crate::model::EntryId;
use crate::tomlout::{basic_string, inline_table, string_array};

/// 冒頭の注意書き（設計「Data Models」の逐語）。
const HEADER_LINES: [&str; 2] = [
    "# 機械生成。手で編集しない。再生成: cargo run -p ukadoc-survey -- catalog",
    "# 形式の正本: .kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/design.md",
];

/// 冒頭の情報を置く表の見出し。
const SNAPSHOT_HEADER: &str = "[snapshot]";

/// 項目を置く表の見出し。
const ENTRY_HEADER: &str = "[entry]";

/// 項目 1 行の列の並び（設計「Data Models」の列表・D-9）。
///
/// 並べ替えても TOML としては同じ表に読み戻るので、要件 1.5 が守る対象そのものが
/// 変わってしまう。ここを触ったら `write_tests.rs` の逐語テストが赤になる。
const ENTRY_COLUMNS: [&str; 6] = ["page", "title", "category", "versions", "hash", "url"];

/// カタログを `catalog.toml` の本文にする（要件 1.1・1.5・1.6）。
///
/// 返る本文は改行だけで区切られ、必ず改行で終わる（設計 D-6）。同じカタログを
/// 2 回渡せば 1 バイトも違わない本文が返る。
pub fn write(catalog: &Catalog) -> String {
    let mut out = String::new();
    for line in HEADER_LINES {
        push_line(&mut out, line);
    }
    push_line(&mut out, "");

    push_line(&mut out, SNAPSHOT_HEADER);
    push_snapshot(&mut out, &catalog.snapshot);
    push_line(&mut out, "");

    push_line(&mut out, ENTRY_HEADER);
    // 表の鍵をそのまま使う。項目の側の `id` を使うと、鍵と中身が食い違ったカタログを
    // 黙って鍵の方だけ書き換えた本文が出てしまう。
    for (id, entry) in &catalog.entries {
        push_line(&mut out, &entry_line(id, entry));
    }
    out
}

/// 1 行を改行付きで押し込む。
fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

/// 冒頭の情報 8 欄を書く（要件 1.6・設計 D-9）。
///
/// 並びは設計「Data Models」の見本の逐語。[`SnapshotMeta::total_entries`]（出典を
/// 問わない全件数）と [`SnapshotMeta::ukadoc_entries`]（うち正典の件数）は別の欄で、
/// 入れ替えても TOML としては読めてしまうので、逐語テストがこの並びを守る。
fn push_snapshot(out: &mut String, meta: &SnapshotMeta) {
    push_field(out, "package", &basic_string(&meta.package));
    push_field(out, "package_version", &basic_string(&meta.package_version));
    push_field(out, "snapshot_version", &meta.snapshot_version.to_string());
    push_field(out, "generated_at", &basic_string(&meta.generated_at));
    push_field(out, "total_entries", &meta.total_entries.to_string());
    push_field(out, "ukadoc_entries", &meta.ukadoc_entries.to_string());
    push_field(out, "catalog_format", &meta.catalog_format.to_string());
    push_field(out, "hash_algorithm", &basic_string(&meta.hash_algorithm));
}

/// `名前 = 値` の 1 行。値は組み上がった TOML の値の断片を受け取る。
fn push_field(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(" = ");
    push_line(out, value);
}

/// 項目 1 件の 1 行（要件 1.1）。
///
/// 列は [`ENTRY_COLUMNS`] の順に並ぶ。`versions` は与えられた順のまま書く——昇順に
/// 整えるのは組み立て（`catalog::build`）の仕事で、ここで並べ替えると読み戻した
/// カタログを書き戻したときに本文が動く。
fn entry_line(id: &EntryId, entry: &CatalogEntry) -> String {
    let values = [
        basic_string(entry.page.as_str()),
        basic_string(&entry.title),
        basic_string(&entry.category),
        string_array(&entry.versions),
        basic_string(&entry.hash),
        basic_string(&entry.url),
    ];
    let pairs: Vec<(&str, String)> = ENTRY_COLUMNS
        .into_iter()
        .zip(values)
        .collect::<Vec<(&str, String)>>();
    format!("{} = {}", basic_string(id.as_str()), inline_table(&pairs))
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;
