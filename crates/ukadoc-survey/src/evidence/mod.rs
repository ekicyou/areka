//! 証拠（ソースに置かれた正典 URL）の型と束ね。
//!
//! 「実装済み」と書かれた台帳の行の根拠は、台帳の中ではなく **areka のソース側**に
//! 置く（要件 2.3・5.1）。置き方は「定義箇所に 1 項目 1 行の doc コメント」だけで、
//! 行番号も内部 ID も使わない。整理や作り替えで行が上下に動いても根拠は壊れない
//! （要件 6.11）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。ソースの
//! 本文は入出力層の [`crate::io::sources::walk`] が読んで渡す。
//!
//! 仕事は 3 つに分かれる。
//!
//! - 取り出し（[`extract`]）— 1 ファイルの本文から、正典 URL の行だけを拾う。
//! - 解決（[`resolve`]）— 拾った URL をカタログの項目 id へ結ぶ。ページ URL は
//!   語彙表の目印として扱い、表の要素名とカタログの見出しを名前で突き合わせる。
//! - 候補（[`candidates`]）— まだ URL が置かれていない既存コードから、URL を置く
//!   作業の手掛かりを拾う。**これは証拠ではない**（要件 5.9）。
//!
//! 未実装の項目についてはソース側に何も書かせない（要件 5.7）。未対応であることは
//! 台帳が持ち、ソースは「実装したものに URL を 1 行足す」だけを担う。だから証拠の
//! 型はどれも「無いこと」を表す欄を持たない——無いとは、単に出てこないことである。

use std::collections::BTreeMap;

use crate::model::EntryId;

pub mod candidates;
pub mod extract;
pub mod resolve;

/// ソースの 1 行から取り出した正典 URL 1 件（要件 5.1）。
///
/// **行番号を持たない**。これは書き忘れではなく要件 5.1・6.11 そのもので、証拠が
/// 行の位置に依存しないことを型の形で守っている。欄は [`Self::path`] と
/// [`Self::url`] の 2 つだけで、この 2 つ以外を足してはいけない。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UrlHit {
    /// ワークスペース根からの相対パス（区切りは `/`）。
    pub path: String,
    /// 行から取り出した 1 語。カタログに実在するかはここでは見ない（設計 D-4）。
    pub url: String,
}

/// 証拠の索引（要件 5.5）。台帳には書き込まず、検査の出力に並べるだけの値である。
///
/// 3 つの欄は役目が違う。[`Self::by_id`] だけが証拠で、残り 2 つは「証拠にできな
/// かったもの」の置き場である（要件 5.9 のとおり判定は人手に委ねる）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceIndex {
    /// 項目 id → その URL が現れたファイルパス。
    ///
    /// 並びの契約は **重複を除いた名前順**（設計 D-4）。同じ URL が複数のファイルに
    /// 現れても赤にしない——要件 5.2 の「定義箇所だけ」は人が守る規約で機械には
    /// 判定できず、要件 6.11 は整理で壊れないことを求めているからである。
    /// この並びを作るのは [`resolve`] の役目。
    pub by_id: BTreeMap<EntryId, Vec<String>>,
    /// カタログの項目 URL にもページ URL にも一致しなかった URL（設計 D-4 の 3 段目）。
    pub unresolved: Vec<UnresolvedUrl>,
    /// 語彙表の要素で、カタログの見出しと 1 件に定まらなかったもの（設計 D-5）。
    pub unmatched_names: Vec<UnmatchedName>,
}

/// カタログに無い URL（要件 6.5・6.10 の「綴りが違う」）。
///
/// `http` と `https` の別・全角文字の混入・末尾の余計な文字は、すべてここへ落ちる。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnresolvedUrl {
    /// URL が書かれていたファイルパス。
    pub path: String,
    /// 書かれていた綴りそのまま。直す人がそのまま検索できるように加工しない。
    pub url: String,
}

/// 語彙表の要素で対応が付かなかったもの（設計 D-5・要件 5.4）。
///
/// 赤にはしない。0 件でも 2 件以上でも、どちらへ寄せるかは人が決める。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnmatchedName {
    /// 語彙表が書かれていたファイルパス。
    pub path: String,
    /// 目印として置かれていたページ URL（綴りそのまま）。
    pub page_url: String,
    /// 対応が付かなかった理由と、対象の要素の文字列。
    pub reason: NameMatchFailure,
}

/// 語彙表の名前が証拠にならなかった理由（設計 D-5）。
///
/// 要素の文字列は**理由の側が持つ**。表そのものが続かなかったときには要素が 1 つも
/// 無いので、別の欄に `Option` で持たせると「`None` のときどうするか」を誰も強制
/// されない。理由と対象を 1 つの値にすれば、対応は構造で保たれる。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NameMatchFailure {
    /// 正規化しても同じ見出しがそのページに 1 つも無い（要素の文字列を持つ）。
    NoMatch(String),
    /// 同じ見出しがそのページに 2 つ以上あって 1 件に定まらない（要素の文字列を持つ）。
    Ambiguous(String),
    /// ページ URL の行の後にスライス定数が始まらない（「目印だが表が続かない」）。
    TableMissing,
}

/// 正典 URL を置く作業の手掛かり（要件 5.8）。
///
/// **証拠ではない**（要件 5.9）。[`EvidenceIndex`] には 1 件も入れず、別の値として
/// 返す。状態の判定は調査 spec の人手に委ねる。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Candidate {
    /// 手掛かりが見つかったファイルパス。証拠と同じく行番号は持たない（要件 5.1）。
    pub path: String,
    /// 手掛かりの種類。
    pub kind: CandidateKind,
    /// 拾った文字列そのまま（イベント名・登録名・設定キー・ログ行の本文）。
    pub text: String,
}

/// 手掛かりの 4 種（要件 5.8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateKind {
    /// 許可表の要素文字列（送出イベント名・リソース名など）。
    AllowListElement,
    /// `\![...]` の消費側の登録名。
    BangCommandConsumer,
    /// 設定キーの表。
    ConfigKey,
    /// 「縮退」「無視」「未知」などを含むログ行。
    LogLine,
}
