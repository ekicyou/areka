//! 3 文書（`linkage.md`・`briefing.md`・`roadmap-draft.md`）の骨組みの型。
//!
//! 3 文書は「人が読む本文」と「機械が読む ```toml の囲み」を同じファイルに置く
//! （設計 D-1）。ここに並ぶのはその囲みの中身の型で、読み手は [`parse`] 1 つだけ
//! である——読み方が判定と副手続きで割れないようにするためで、欄を増やすときは
//! ここと [`parse`] の 2 か所だけを直す。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（設計「境界」）。
//!
//! # 欄の一覧は設計 D-2 が正本
//!
//! 欄名と語彙は設計「Logical Data Model（3 文書の骨組み・D-2）」の表そのままで、
//! 増やしも減らしもしない。知らない欄は読み手が落とす（台帳の読み手と同じ流儀）。

use std::collections::BTreeMap;

use crate::model::{Domain, EntryId, PageName};

pub mod parse;

// ---------------------------------------------------------------------------
// 共通の語彙
// ---------------------------------------------------------------------------

/// 製品品質の段階（要件 5）。台帳の `priority` の頭 1 文字でもある。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    A,
    B,
    C,
    D,
    E,
}

impl Stage {
    /// 5 つの段階（A が最も先）。
    pub const ALL: [Self; 5] = [Self::A, Self::B, Self::C, Self::D, Self::E];

    /// 文書と台帳に書く 1 文字。
    pub fn as_key(&self) -> &'static str {
        // 既定の腕を置かない。段階を増やしたらここが赤くなる。
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
        }
    }

    /// 綴りから段階を引く。5 つのいずれでもなければ `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.as_key() == raw)
    }
}

/// 束が欠けたときの壊れ方の最悪値（要件 4.1 ⑺）。
///
/// 「該当なし」は全構成 id が `implemented` の束にだけ許されるが、その突き合わせは
/// 台帳を見る判定の担当で、読み手は語彙かどうかだけを見る（設計 D-2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Breakage {
    /// 黙って壊れる。
    Silent,
    /// 明示エラー。
    Explicit,
    /// 見た目の差。
    Visual,
    /// 該当なし。
    NotApplicable,
}

impl Breakage {
    /// 4 つの語彙（悪いほうから並べる。順位の第 1 根拠の序列でもある・要件 6.1）。
    pub const ALL: [Self; 4] = [
        Self::Silent,
        Self::Explicit,
        Self::Visual,
        Self::NotApplicable,
    ];

    /// 文書に書く綴り。
    pub fn as_key(&self) -> &'static str {
        // 既定の腕を置かない。語彙を増やしたらここが赤くなる。
        match self {
            Self::Silent => "黙って壊れる",
            Self::Explicit => "明示エラー",
            Self::Visual => "見た目の差",
            Self::NotApplicable => "該当なし",
        }
    }

    /// 綴りから壊れ方を引く。4 つのいずれでもなければ `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|breakage| breakage.as_key() == raw)
    }
}

/// 束への参照（`roadmap-draft.md` の spec 表と予約群）。
///
/// 「どの束にも属さない」を空欄で表さず、`none = true` と理由で表す（要件 11.5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleRef {
    /// `linkage.md` の束名。
    Named(String),
    /// どの束にも属さない。理由を必ず持つ。
    None { reason: String },
}

// ---------------------------------------------------------------------------
// linkage.md
// ---------------------------------------------------------------------------

/// `linkage.md` の骨組み（帰属の正本・要件 4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linkage {
    /// 束（名前付き束と単独項目）。鍵は束名で、束名の文字順に並ぶ。
    pub bundles: BTreeMap<String, NamedBundle>,
    /// 件数の宣言。判定が数え直す。
    pub tally: Tally,
}

/// 束 1 つ分（`[bundle."名前"]`・要件 4.1 の 8 項目）。
///
/// 単独項目も同じ型で持つ（[`Self::single`] が真）。型を 1 つにしてあるのは、分割の
/// 判定も順位の行も同じ規則で扱えるようにするためである（設計 D-2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedBundle {
    /// ⑴ 束の名前（表の鍵と同じ綴り）。
    pub name: String,
    /// 単独項目なら真。
    pub single: bool,
    /// ⑵ 由来する機械の束 id（0 個以上）。単独項目は持たない。
    pub machine: Vec<EntryId>,
    /// ⑶ 構成 id の全列挙。単独項目はちょうど 1 つ。
    pub members: Vec<EntryId>,
    /// ⑶ のうち人手で足した id（[`Self::members`] の部分集合）。単独項目は持たない。
    pub hand: Vec<EntryId>,
    /// ⑷ 跨ぐドメイン。判定が数え直す。
    pub domains: Vec<Domain>,
    /// ⑸ 成立に要る最小の基盤の見出し。単独項目は空。
    pub foundation: String,
    /// ⑺ 壊れ方の最悪値。
    pub breakage: Breakage,
    /// ⑻ テーマの集合。判定が数え直す。
    pub themes: Vec<String>,
    /// 単独項目に落とした理由（要件 4.4）。名前付き束は空。
    pub reason: String,
}

/// `[tally]`（要件 4.2 の 3 つの数と除外の数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tally {
    /// 対象 4 状態の全数。
    pub target: usize,
    /// 単独でない束の `members` ∖ `hand` の総数。
    pub from_machine: usize,
    /// 単独でない束の `hand` の総数。
    pub by_hand: usize,
    /// 単独項目の数。
    pub singles: usize,
    /// 束から除いた `alias` の数。
    pub alias_excluded: usize,
    /// 束から除いた `not-applicable` の数。
    pub not_applicable_excluded: usize,
    /// 単独項目のドメイン別内訳。4 ドメインすべてが揃う（0 も省略できない・要件 11.5）。
    pub singles_by_domain: BTreeMap<Domain, usize>,
}

// ---------------------------------------------------------------------------
// briefing.md
// ---------------------------------------------------------------------------

/// `briefing.md` の骨組み（段階と順位・要件 5・6・8）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Briefing {
    /// 順位の行。**本文に現れた順**（順位の主張を見るのは導出の担当）。
    pub ranks: Vec<RankRow>,
    /// 段階ごとの数。A〜E の 5 つが必ず揃う。
    pub stages: BTreeMap<Stage, StageCount>,
    /// 段階 A の主障壁（ページ別の状態分布・要件 8.1 ⑷）。
    pub barriers: Vec<Barrier>,
    /// 書き戻した後のドメイン別段階分布（要件 7.3）。
    pub afters: Vec<After>,
    /// 優先度を空欄にする項目の数（要件 7.2）。
    pub priority_blank: PriorityBlank,
    /// 完了済み spec を `owner` に残した宛先（要件 7.4 ⑵）。
    pub owner_completed: Vec<OwnerCompleted>,
    /// 参照した標準テンプレート辞書（要件 6.3）。
    pub templates: Vec<Template>,
}

/// 順位の行 1 つ（`[[rank]]`・要件 6.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankRow {
    /// 段階。
    pub stage: Stage,
    /// 段階の中での順位（同順位は同じ数・要件 6.7）。
    pub rank: usize,
    /// 順位を付ける相手（束 1 つか、同順位の単独項目の並び）。
    pub target: RankTarget,
    /// ⑶ 影響する既存資産の広さ。判定が数え直す。書かなければ 0。
    pub assets: usize,
    /// ⑷ 依存基盤の共有度。判定が数え直す。書かなければ 0。
    pub shared: usize,
    /// 4 つの根拠の順序から外す理由（toml の欄名は `override`）。
    pub exception: Option<RankOverride>,
    /// 退路（要件 6.8）で `assets` が決められない束。順序の主張から外す。
    pub insufficient: bool,
}

/// 順位を付ける相手。`bundle` と `singles` のちょうど一方（設計 `documents::parse`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RankTarget {
    /// `linkage.md` の束名。
    Bundle(String),
    /// 同順位の単独項目の id。
    Singles(Vec<EntryId>),
}

/// 4 つの根拠の順序から外す理由（`override`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankOverride {
    /// 外す根拠の種別。
    pub kind: OverrideKind,
    /// 根拠の指し先（toml の欄名は `ref`）。
    pub reference: String,
}

/// `override` の種別（設計 D-2 の受け付け形）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OverrideKind {
    /// 第二段の実物（M1 完成の適合検証項目・持ち越し行）で動かした。
    SecondStage,
    /// 段階の規則の例外（要件 5.3）。
    StageRule,
}

impl OverrideKind {
    /// 2 つの語彙。
    pub const ALL: [Self; 2] = [Self::SecondStage, Self::StageRule];

    /// 文書に書く綴り。
    pub fn as_key(&self) -> &'static str {
        // 既定の腕を置かない。種別を増やしたらここが赤くなる。
        match self {
            Self::SecondStage => "second-stage",
            Self::StageRule => "stage-rule",
        }
    }

    /// 綴りから種別を引く。2 つのいずれでもなければ `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_key() == raw)
    }
}

/// 段階ごとの数（`[stage.X]`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageCount {
    /// 名前付き束の数。
    pub bundles: usize,
    /// 単独項目の数。
    pub singles: usize,
    /// 項目の数。
    pub items: usize,
}

/// 段階 A の主障壁 1 行（`[[barrier]]`）。ページ 1 つの状態分布。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Barrier {
    /// ukadoc のページ名。
    pub page: PageName,
    pub implemented: usize,
    pub vocabulary_only: usize,
    pub degraded: usize,
    pub absent: usize,
    pub alias: usize,
    pub not_applicable: usize,
}

/// 書き戻した後のドメイン別段階分布 1 行（`[[after]]`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct After {
    /// ドメイン。
    pub domain: Domain,
    /// 段階ごとの項目数。A〜E の 5 つが必ず揃う。
    pub stages: BTreeMap<Stage, usize>,
    /// 優先度が空欄の項目数。
    pub empty: usize,
}

/// 優先度を空欄にする項目の数（`[priority_blank]`・要件 7.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityBlank {
    pub alias: usize,
    pub not_applicable: usize,
}

/// 完了済み spec を `owner` に残した宛先 1 行（`[[owner_completed]]`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCompleted {
    /// spec 名。`completed/` に実在することは判定 ⑸-e が確かめる。
    pub spec: String,
    /// その名前を `owner` に持つ項目数。判定が数え直す。
    pub items: usize,
}

/// 参照した標準テンプレート辞書 1 本（`[[template]]`・要件 6.3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// テンプレートの名前。
    pub name: String,
    /// SHIORI の種別（里々・YAYA など）。
    pub shiori: String,
    /// 配布元の URL。退路を使ったときは取得できなかった配布元。
    pub url: String,
    /// 取得日。
    pub fetched_on: String,
    /// 読んだ辞書ファイル名。
    pub files: Vec<String>,
    /// id への写し方。
    pub mapping: String,
    /// 退路（要件 6.8）を使ったら真。
    pub fallback: bool,
    /// 辞書に現れた語彙を写した id。
    pub ids: Vec<EntryId>,
}

// ---------------------------------------------------------------------------
// roadmap-draft.md
// ---------------------------------------------------------------------------

/// `roadmap-draft.md` の骨組み（要件 10）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapDraft {
    /// brief 済み未完了 spec の数と、それを撮った日。
    pub briefs: Briefs,
    /// spec 表の行。**本文に現れた順**。
    pub specs: Vec<SpecRow>,
    /// M2 予約群（要件 10.8）。
    pub reserved: Vec<Reserved>,
}

/// `[briefs]`（着手時の写真・要件 10.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Briefs {
    /// `[[spec]]` の行数と一致すべき数。判定が数え直す。
    pub count: usize,
    /// 撮った日。
    pub snapshot_on: String,
}

/// spec 表の 1 行（`[[spec]]`・要件 10.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecRow {
    /// spec 名（ディレクトリ名）。
    pub name: String,
    /// roadmap.md のウェーブ（W13〜W17・保留）。
    pub wave: String,
    /// 段階。
    pub stage: Stage,
    /// 属する束。
    pub bundle: BundleRef,
    /// 台帳で `owner` にこの名前を持つ id の数。判定が数え直す。
    pub owner_count: usize,
}

/// M2 予約群の 1 行（`[[reserved]]`・要件 10.8）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reserved {
    /// 予約項目の名前。
    pub name: String,
    /// 写った束。
    pub bundle: BundleRef,
}
