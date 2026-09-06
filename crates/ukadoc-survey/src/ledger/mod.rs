//! 台帳（areka の判定）の型と束ね。
//!
//! 台帳は**人が手で書き、機械が検査する**文書である（要件 2.1）。カタログが正典の
//! 写し（機械生成）であるのに対し、台帳は「areka がその項目をどう扱っているか」を
//! 人が記入する側で、両者は別ファイル・別責務に保つ（要件 2.7）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。本文の
//! 文字列を受け取り、値を返す。読み取りは [`read`]、塊への切り分けは [`blocks`]、
//! 初期生成と差し込みは [`write`]。
//!
//! # 欄の一覧は付録 A.2 が正本
//!
//! [`LedgerEntry`] の欄は要件付録 A.2 の表そのままで、増やしも減らしもしない
//! （要件 2.6 により変更には要件の改訂を要する）。**無い欄が 2 つある**——どちらも
//! 書き忘れではなく要件そのものである。
//!
//! - **証拠の欄は無い**（要件 2.3）。実装済みの根拠はソース側の doc コメントに置き、
//!   検査が集める。台帳に持たせるとソースの整理でファイルが動くたびに台帳を書き
//!   直すことになる。
//! - **未分類の件数を宣言する欄は無い**（要件 6.9）。件数は報告側の分布を正とし、
//!   同じ数を 2 か所で持たない。
//!
//! どちらも「知らない欄は落とす」（[`read`]）ことで型の形として守られる。

use std::collections::BTreeMap;

use crate::model::{Domain, EntryId, Link, PageName, Status};

pub mod blocks;
pub mod read;
pub mod write;

/// 台帳の 1 項目（要件付録 A.2 の欄）。
///
/// 任意の欄は 2 つだけで、いずれも「書かれていなければ空」として持つ
/// （[`Self::alias_of`] は `None`、[`Self::supersedes`] は空の配列）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// 項目 id。表の鍵に書かれた綴りをそのまま持つ。
    pub id: EntryId,
    /// 状態（要件 2.2 の 7 語彙のいずれか）。
    pub status: Status,
    /// 登場した SSP 版番号。不明なら空文字（＝世代不明。最古とは決めつけない・要件 4.2）。
    pub introduced: String,
    /// 正典側の id。[`Status::Alias`] の項目にだけ書ける（要件 2.4・付録 A.2）。
    ///
    /// 別名の行が持つのは「写像があるか否か」だけで、実装状態の判定は写像先の正典行に
    /// 委ねる（要件 2.4）。指す先が `alias` でないことの検査は整合検査の担当（要件 6.7）。
    pub alias_of: Option<EntryId>,
    /// この項目が置き換えた旧 id の一覧（`alias_of` の逆向き）。書かれていなければ空。
    pub supersedes: Vec<EntryId>,
    /// 担当 spec 名。未設定は空文字。
    pub owner: String,
    /// 優先度（段階 1 文字＋数値）。未設定は空文字。
    pub priority: String,
    /// 伺からしさのテーマ（要件 4.4 の 8 つのいずれか・0 個以上）。
    pub values: Vec<String>,
    /// 関連（種別と相手 id の対・要件 4.3 の 6 種）。
    pub links: Vec<Link>,
    /// 備考。複数行を許す。
    pub note: String,
}

/// 台帳 1 ファイル分。
///
/// 項目は **2 通りの並び**で持つ。どちらも要るのは、両者が食い違うことがそれ自体
/// 意味を持つからである。
///
/// - [`Self::entries`] は id を鍵にした表なので **id の byte 昇順**。値を引く側は
///   こちらを使う。
/// - [`Self::file_order`] は **本文に現れた順**。付録 A は「id の文字順に並べる」を
///   必須にしており、この 2 つが食い違えば台帳の並びが取り決めを破っている
///   （設計 D-12。判定そのものは差し込みと整合検査の担当）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    /// この台帳の担当ドメイン。
    pub domain: Domain,
    /// 前置きに書かれた担当ページ（要件 3.1）。
    ///
    /// 割り当ての正本は `assignment::canonical()` であって、ここではない。前置きと
    /// 正本が集合として一致するかは整合検査が確かめる。
    pub pages: Vec<PageName>,
    /// 項目（id の byte 昇順）。
    pub entries: BTreeMap<EntryId, LedgerEntry>,
    /// 本文に現れた順の項目 id（並び順の検査に使う・設計 D-12）。
    pub file_order: Vec<EntryId>,
}
