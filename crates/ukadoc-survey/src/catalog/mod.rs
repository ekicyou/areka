//! カタログ（正典の写し）の型と束ね。
//!
//! カタログは ukadoc の項目を 1 項目 1 行で写した機械生成の文書である。
//! 写すのは **項目 id・ページ名・見出し・カテゴリ・本文に現れた版番号・本文のハッシュ・
//! 正典 URL** の 7 つだけで、**本文そのものは持たない**。本文が変わったかどうかは
//! ハッシュの比較だけで判じる。
//!
//! 純粋層で、ファイルにもスナップショットにも触らない。組み立ては [`build`]、
//! 本文への書き出しと読み戻しは [`write`]・[`read`]。

use std::collections::BTreeMap;

use crate::model::{EntryId, PageName};

pub mod build;
pub mod read;
pub mod write;

/// カタログの形の版。
///
/// 列を増やしたり並びの規則を変えたりしたら 1 つ繰り上げる。冒頭に書いておくことで、
/// 古い形のカタログを読んだときに「読めるが意味が違う」状態を見分けられる。
pub const CATALOG_FORMAT: u32 = 1;

/// 冒頭の情報に置いてよい欄のすべて。読み手と書き手が共有する。
///
/// 並びは書き出しの並び。`total_entries` と `ukadoc_entries` を入れ替えても TOML
/// としては読めてしまうので、`write_tests.rs` の逐語テストがこの並びを守る。
pub(super) const SNAPSHOT_FIELDS: [&str; 8] = [
    "package",
    "package_version",
    "snapshot_version",
    "generated_at",
    "total_entries",
    "ukadoc_entries",
    "catalog_format",
    "hash_algorithm",
];

/// 項目 1 行の列の並び。読み手と書き手が共有する。
///
/// 並べ替えても TOML としては同じ表に読み戻るので、ここを触ったら `write_tests.rs` の
/// 逐語テストが赤になる。
pub(super) const ENTRY_COLUMNS: [&str; 6] =
    ["page", "title", "category", "versions", "hash", "url"];

/// カタログ冒頭に記録するスナップショットの情報。
///
/// [`Self::total_entries`] と [`Self::ukadoc_entries`] は別物である。前者は
/// スナップショットに入っていた全 entry の件数（ukadoc 以外の出典も含む）、後者は
/// そのうちカタログへ残した正典由来の件数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMeta {
    /// スナップショットを配るパッケージの名前。
    pub package: String,
    /// そのパッケージの版。
    pub package_version: String,
    /// スナップショット自身の版（JSON 最上位の `version`）。
    pub snapshot_version: i64,
    /// スナップショットの生成日時（JSON 最上位の `generatedAt`）。
    pub generated_at: String,
    /// スナップショットの全 entry 件数（出典を問わない）。
    pub total_entries: usize,
    /// うちカタログに残した正典由来の件数。
    pub ukadoc_entries: usize,
    /// カタログの形の版（[`CATALOG_FORMAT`]）。
    pub catalog_format: u32,
    /// 本文ハッシュの算法の名前（`crate::hash::HASH_ALGORITHM`）。
    pub hash_algorithm: String,
}

/// カタログの 1 項目。
///
/// **本文の欄は無い**。書き忘れではなく、本文を持たせないことを型の形で守っている。
/// 本文から作るのは [`Self::versions`] と [`Self::hash`] の 2 つだけで、作り終えた
/// 本文は捨てる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// 項目 id。アンカーの有無で形は 2 つある。
    pub id: EntryId,
    /// ページ名。id の 2 番目の区切りから取る。
    pub page: PageName,
    /// 見出し。
    pub title: String,
    /// カテゴリ。
    pub category: String,
    /// 本文に現れた版番号のすべて。重複を除き文字列として昇順。
    /// 1 つも無ければ空。**1 つに絞らない**——2 つ以上を持つ項目が実データにある。
    pub versions: Vec<String>,
    /// 本文のハッシュ（16 桁の 16 進小文字）。
    pub hash: String,
    /// 正典 URL。
    pub url: String,
}

/// カタログ 1 つ分。
///
/// 項目は id を鍵にした 1 つの表で持つ。並びは `EntryId` の順＝id の byte 昇順で、
/// 書き出しの並びもここから来る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    /// 冒頭のスナップショット情報。
    pub snapshot: SnapshotMeta,
    /// 項目（id の byte 昇順）。
    pub entries: BTreeMap<EntryId, CatalogEntry>,
}
