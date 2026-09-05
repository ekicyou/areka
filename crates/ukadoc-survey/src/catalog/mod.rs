//! カタログ（正典の写し）の型と束ね。
//!
//! カタログは ukadoc の項目を 1 項目 1 行で写した機械生成の文書である（要件 1.1）。
//! 写すのは **項目 id・ページ名・見出し・カテゴリ・本文に現れた版番号・本文のハッシュ・
//! 正典 URL** の 7 つだけで、**本文そのものは持たない**（要件 1.3・9.4）。本文が
//! 変わったかどうかはハッシュの比較だけで判じる（要件 8.2）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。値を受け取り、
//! 値を返す。組み立ては [`build`]、本文への書き出しと読み戻しは [`write`]・[`read`]。

use std::collections::BTreeMap;

use crate::model::{EntryId, PageName};

pub mod build;
pub mod read;
pub mod write;

/// カタログの形の版（設計 D-9）。
///
/// 列を増やしたり並びの規則を変えたりしたら 1 つ繰り上げる。冒頭に書いておくことで、
/// 古い形のカタログを読んだときに「読めるが意味が違う」状態を見分けられる。
pub const CATALOG_FORMAT: u32 = 1;

/// カタログ冒頭に記録するスナップショットの情報（要件 1.6）。
///
/// [`Self::total_entries`] と [`Self::ukadoc_entries`] は別物である。前者は
/// スナップショットに入っていた全 entry の件数（実測 2,983 件。ukadoc 以外の出典も
/// 含む）、後者はそのうちカタログへ残した正典由来の件数（実測 1,749 件）。
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

/// カタログの 1 項目（要件 1.2 の列）。
///
/// **本文の欄は無い**。これは書き忘れではなく要件 1.3・9.4 そのもので、本文を
/// 持たせないことを型の形で守っている。本文から作るのは [`Self::versions`] と
/// [`Self::hash`] の 2 つだけで、作り終えた本文は捨てる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// 項目 id。アンカーの有無で形は 2 つあるが、収容の仕方は変わらない（要件 1.9）。
    pub id: EntryId,
    /// ページ名。id の 2 番目の区切りから取る（設計 D-11）。
    pub page: PageName,
    /// 見出し。
    pub title: String,
    /// カテゴリ。
    pub category: String,
    /// 本文に現れた版番号のすべて。重複を除き文字列として昇順（要件 1.2）。
    /// 1 つも無ければ空。**1 つに絞らない**——2 つ以上を持つ項目が実測 23 件ある。
    pub versions: Vec<String>,
    /// 本文のハッシュ（16 桁の 16 進小文字）。
    pub hash: String,
    /// 正典 URL。
    pub url: String,
}

/// カタログ 1 つ分。
///
/// 項目は id を鍵にした 1 つの表で持つ。並びは `EntryId` の順＝id の byte 昇順で、
/// 書き出しの並びもここから来る（設計 D-9）。同じ id が 2 つ入ることは表の形として
/// 起こり得ない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    /// 冒頭のスナップショット情報。
    pub snapshot: SnapshotMeta,
    /// 項目（id の byte 昇順）。
    pub entries: BTreeMap<EntryId, CatalogEntry>,
}
