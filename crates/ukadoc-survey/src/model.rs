//! 台帳とカタログで使う値の型と語彙（EntryId・PageName・Domain・Status・LinkKind・
//! テーマ・Link）を 1 か所で定義する。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない。文字列と値だけを受け取り、
//! 文字列と値だけを返す。
//!
//! 凍結された語彙は 4 つある（要件 2.6 により、変更には要件の改訂を要する）。
//!
//! - 状態 7 種（要件 2.2）
//! - 関連の種別 6 種（要件 4.3）
//! - 伺からしさのテーマ 8 種（要件 4.4。正本は `doc/ukadoc-coverage/values.md` で、
//!   ここの定数との一致は `tests/consistency/values_md.rs` が守る）
//! - 担当ドメイン 4 種（要件 3.1）
//!
//! どの語彙も文字列からの変換を失敗しうる操作にしてある。綴りが 1 文字違えば値は
//! 作れず、黙って既定値に落ちることはない（設計「実装上の注意」）。列挙から文字列を
//! 引く向きの `match` には既定の腕を置かない。語彙を 1 つ増やすと、対応表を書き足す
//! までコンパイルが通らなくなる。

use crate::error::SurveyError;

/// 語彙に無い綴りに出会ったこと。
///
/// この層は「どのファイルのどの項目を読んでいたか」を知らない。だから欄の名前と綴り
/// だけを持ち、`file` と `id` は台帳を読む段（`ledger::read`）が [`Self::at`] で添えて
/// [`SurveyError::BadVocabulary`] にする（設計 Error Handling・要件 6.10）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field} の値 {value} は語彙に無い")]
pub struct UnknownVocabulary {
    /// 台帳の欄の名前（`status` `links.kind` `values` `domain`）。
    pub field: &'static str,
    /// 実際に書かれていた綴り。
    pub value: String,
}

impl UnknownVocabulary {
    fn new(field: &'static str, value: &str) -> Self {
        Self {
            field,
            value: value.to_owned(),
        }
    }

    /// 読んでいたファイルと項目 id を添えて、この道具の失敗に仕立てる。
    pub fn at(self, file: impl Into<String>, id: impl Into<String>) -> SurveyError {
        SurveyError::BadVocabulary {
            file: file.into(),
            id: id.into(),
            field: self.field,
            value: self.value,
        }
    }
}

/// 項目 id。カタログと台帳の鍵で、`ukadoc:` で始まる 2 形だけを受け付ける（要件 1.9）。
///
/// 並びは中の文字列の byte 昇順（設計 D-9）。実測で id はすべて ASCII なので、これが
/// そのまま文字順になる。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(String);

/// ukadoc のページ名（`dev_bind` `list_sakura_script` など）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageName(String);

/// 台帳を分ける 4 つの担当ドメイン（要件 3.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    Shiori,
    Assets,
    SakuraScript,
    Property,
}

/// 項目の状態（要件 2.2 の 7 語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    Implemented,
    VocabularyOnly,
    Degraded,
    Absent,
    Alias,
    NotApplicable,
    Unclassified,
}

/// 項目どうしの関連の種別（要件 4.3 の 6 語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkKind {
    /// 旧→新。
    AliasOf,
    /// 新→旧。
    Supersedes,
    /// 操作・タグ→イベント。
    Triggers,
    /// 設定キー→挙動・タグ・イベント。
    Configures,
    /// タグ・イベント→プロパティ。
    Queries,
    /// 同じ機能の別の面。
    SameFeature,
}

/// 台帳の `links` の 1 要素（種別と相手 id の対）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub kind: LinkKind,
    pub to: EntryId,
}

/// 要件 4.4 の 8 テーマ。values.md の見出しと一致することをテストで守る。
pub const THEMES: [&str; 8] = [
    "気配",
    "触れ合い",
    "掛け合い",
    "装い",
    "記憶",
    "交わり",
    "気配り",
    "更新",
];

/// テーマ名を凍結された 8 つの綴りへ解決する（要件 4.4）。
///
/// 返すのは定数側の綴りそのもの。前後の空白を落としたり部分一致で拾ったりはしない
/// （「気配」と「気配り」は片方が他方の接頭辞なので、緩めると取り違える）。
pub fn parse_theme(raw: &str) -> Result<&'static str, UnknownVocabulary> {
    THEMES
        .into_iter()
        .find(|theme| *theme == raw)
        .ok_or_else(|| UnknownVocabulary::new("values", raw))
}

/// 項目 id の区切り。ページ名自身が下線を含むので、下線では割らない（要件付録 B 手順 2）。
const ID_SEPARATOR: char = ':';

/// 項目 id の先頭に必ず置かれる印。
const ID_PREFIX: &str = "ukadoc";

impl EntryId {
    /// 2 形のいずれかであることを確かめて作る（要件 1.9）。
    ///
    /// 受け付ける形は次の 2 つだけ。区切りはコロンで、下線では割らない。
    ///
    /// - ページ全体: `ukadoc:<ページ>`（コロン 1 つ・4 欄でなく 2 欄）
    /// - アンカー付き: `ukadoc:<ページ>:<アンカー>:<連番>`（コロン 3 つ・4 欄）
    ///
    /// 欄の数で分ける素直な割り方にしてある。スナップショット実測（`source` が
    /// `ukadoc` の全 1,749 件）で id のコロンは 1 つが 19 件・3 つが 1,730 件で、
    /// 2 つや 4 つ以上は 1 件も無く、アンカーの中にコロンを含むものも無い。要件 1.9 の
    /// 「アンカー無し 19 件・アンカー付き 1,730 件」とも一致する。
    /// どの欄も空であってはならない。
    pub fn parse(raw: &str) -> Result<Self, SurveyError> {
        let bad = || SurveyError::BadEntryId {
            raw: raw.to_owned(),
        };
        let fields: Vec<&str> = raw.split(ID_SEPARATOR).collect();
        let shaped = match fields.as_slice() {
            [ID_PREFIX, page] => !page.is_empty(),
            [ID_PREFIX, page, anchor, seq] => {
                !page.is_empty() && !anchor.is_empty() && !seq.is_empty()
            }
            _ => false,
        };
        if shaped {
            Ok(Self(raw.to_owned()))
        } else {
            Err(bad())
        }
    }

    /// ページ名。2 番目の区切りから取る（設計 D-11）。
    pub fn page(&self) -> PageName {
        // [`Self::parse`] を通った値は必ず 2 欄以上あるので、2 番目は必ず存在する。
        let page = self
            .0
            .split(ID_SEPARATOR)
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        PageName(page)
    }

    /// アンカーを持つか。区切りが 3 つなら真。
    pub fn has_anchor(&self) -> bool {
        self.0.matches(ID_SEPARATOR).count() == 3
    }

    /// 元の綴り。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PageName {
    /// ページ名を作る。カタログの id から取り出した名前と、割り当て表に書いた名前の
    /// 双方をこの型で扱う。
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// 元の綴り。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Domain {
    /// 4 つのドメイン。台帳ファイル・報告ファイルを並べる順でもある（要件 3.1 の表）。
    pub const ALL: [Self; 4] = [
        Self::Shiori,
        Self::Assets,
        Self::SakuraScript,
        Self::Property,
    ];

    /// 台帳・報告のファイル名に使う綴り（要件 3.1）。
    pub fn as_key(&self) -> &'static str {
        // 既定の腕を置かない。ドメインを増やしたらここが赤くなる。
        match self {
            Self::Shiori => "shiori",
            Self::Assets => "assets",
            Self::SakuraScript => "sakura-script",
            Self::Property => "property",
        }
    }

    /// 綴りからドメインを引く。4 つのいずれでもなければ失敗する。
    pub fn parse(raw: &str) -> Result<Self, UnknownVocabulary> {
        Self::ALL
            .into_iter()
            .find(|domain| domain.as_key() == raw)
            .ok_or_else(|| UnknownVocabulary::new("domain", raw))
    }
}

impl Status {
    /// 7 つの状態。報告で分布を並べる順でもある（要件 2.2 の並び）。
    pub const ALL: [Self; 7] = [
        Self::Implemented,
        Self::VocabularyOnly,
        Self::Degraded,
        Self::Absent,
        Self::Alias,
        Self::NotApplicable,
        Self::Unclassified,
    ];

    /// 台帳に書く英字の綴り（要件 2.2）。
    pub fn as_key(&self) -> &'static str {
        // 既定の腕を置かない。状態を増やしたらここが赤くなる。
        match self {
            Self::Implemented => "implemented",
            Self::VocabularyOnly => "vocabulary-only",
            Self::Degraded => "degraded",
            Self::Absent => "absent",
            Self::Alias => "alias",
            Self::NotApplicable => "not-applicable",
            Self::Unclassified => "unclassified",
        }
    }

    /// 報告に出す平易な日本語の呼び名（要件 7.8）。台帳とカタログには常に英字の綴りが
    /// 入るので、こちらを使うのは報告だけ。
    pub fn as_japanese(&self) -> &'static str {
        // 既定の腕を置かない。状態を増やしたらここが赤くなる。
        match self {
            Self::Implemented => "実装済み",
            Self::VocabularyOnly => "語彙のみ",
            Self::Degraded => "縮退",
            Self::Absent => "未対応",
            Self::Alias => "別名",
            Self::NotApplicable => "対象外",
            Self::Unclassified => "未分類",
        }
    }

    /// 台帳の綴りから状態を引く。7 つのいずれでもなければ失敗する（要件 6.10）。
    pub fn parse(raw: &str) -> Result<Self, UnknownVocabulary> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_key() == raw)
            .ok_or_else(|| UnknownVocabulary::new("status", raw))
    }
}

impl LinkKind {
    /// 6 つの関連の種別（要件 4.3 の並び）。
    pub const ALL: [Self; 6] = [
        Self::AliasOf,
        Self::Supersedes,
        Self::Triggers,
        Self::Configures,
        Self::Queries,
        Self::SameFeature,
    ];

    /// 台帳に書く英字の綴り（要件 4.3）。下線と横棒が種別ごとに違うので、写し違えない。
    pub fn as_key(&self) -> &'static str {
        // 既定の腕を置かない。種別を増やしたらここが赤くなる。
        match self {
            Self::AliasOf => "alias_of",
            Self::Supersedes => "supersedes",
            Self::Triggers => "triggers",
            Self::Configures => "configures",
            Self::Queries => "queries",
            Self::SameFeature => "same-feature",
        }
    }

    /// 台帳の綴りから種別を引く。6 つのいずれでもなければ失敗する。
    pub fn parse(raw: &str) -> Result<Self, UnknownVocabulary> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_key() == raw)
            .ok_or_else(|| UnknownVocabulary::new("links.kind", raw))
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
