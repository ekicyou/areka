//! 状態の型（版 2）・空の状態・参加者の状態の列挙。
//!
//! 状態ファイル `state.json` はこの [`State`] 1 つをそのまま書いたもの（参加者・リポジトリごとの
//! マージの机・排他テストの机とひとり占めの持ち主・机の申し込みの待ち行列・走っている待ち・
//! 最近の出来事が全部ここに在る）。実装の机は「作業中」の参加者の集まりで、別の項目は持たない。
//! 時刻は全部 UNIX 秒。
//!
//! 不変（守るのは `plan`）: 1 つの識別は、1 つのリポジトリのマージの待ち行列に高々 1 回、
//! 机の申し込みの待ち行列 `requests` に高々 1 回。マージの持ち主と待ち行列に同時には居ない。
//! `requests` に居る識別は `test`・`sole` を持たない。`sole` が走っている印つきなら `test` も
//! 同じ識別が持つ。`waits` は `(id, kind)` で高々 1 件。
//!
//! 項目には `#[serde(default)]` が付いていて、同じ版の中で項目が増えても古いファイルが読める。
//! 例外は記録の種類（[`WaitRecord::kind`]・[`Recent::kind`]）と申し込みの机の組
//! （[`DeskRequest::desks`]）で、欠けたときに入れる中立の値が無いので、欠けていれば形の合わない
//! ファイルとして読まない。

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// この実行ファイルの知っている状態ファイルの版。項目の意味を変える・消すときに上げる。
pub const VERSION: u32 = 2;

/// [`State::recent`] に残す件数の上限。
pub const RECENT_MAX: usize = 50;

/// `Default` は版 0（＝版の無いファイルを読んだ姿）。空の状態は [`State::empty`] で作る。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub version: u32,
    pub participants: BTreeMap<String, Participant>,
    /// リポジトリ名 → マージの机。
    pub merge: BTreeMap<String, MergeDesk>,
    /// 排他テストの机の持ち主。
    pub test: Option<TestHolder>,
    /// 実装の机のひとり占めの持ち主。
    pub sole: Option<SoleHolder>,
    /// 排他テストの机・ひとり占めの申し込みの待ち行列（1 本）。
    pub requests: Vec<DeskRequest>,
    pub waits: Vec<WaitRecord>,
    /// 新しい順・最大 [`RECENT_MAX`] 件。足すときは [`State::push_recent`] を通す。
    pub recent: Vec<Recent>,
}

impl State {
    /// 版だけを持つ空の状態。
    pub fn empty() -> Self {
        State {
            version: VERSION,
            ..State::default()
        }
    }

    /// `clear` の後の状態: 空の状態に「消した記録」1 件だけ。`backup` は退避したファイルの道筋。
    pub fn cleared(at: u64, backup: &Path) -> Self {
        let mut state = State::empty();
        state.push_recent(Recent {
            at,
            kind: RecentKind::Cleared,
            id: None,
            detail: backup.to_string_lossy().into_owned(),
        });
        state
    }

    /// 最近の出来事を先頭へ足し、上限を超えた古いものを落とす。
    pub fn push_recent(&mut self, recent: Recent) {
        self.recent.insert(0, recent);
        self.recent.truncate(RECENT_MAX);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub status: ParticipantStatus,
    pub since: u64,
    pub stop_reason: Option<StopReason>,
    pub watch: Option<WatchInfo>,
    /// 再開してから見張りを立て直すまでの印（在る間は回収しない）。
    pub awaiting_watch_since: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParticipantStatus {
    #[default]
    Working,
    StopRequested,
    Stopped,
}

impl ParticipantStatus {
    /// ASCII の名前。状態ファイルの綴りと同じで、`status` の表示に使う。
    pub const fn as_str(self) -> &'static str {
        match self {
            ParticipantStatus::Working => "working",
            ParticipantStatus::StopRequested => "stop-requested",
            ParticipantStatus::Stopped => "stopped",
        }
    }
}

/// 停止要請の理由: 誰の負荷テストか・その内容。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StopReason {
    pub by: String,
    pub purpose: String,
}

/// 見張りの記録。人が読むためのもので、生死の判定には使わない。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WatchInfo {
    pub pid: u32,
    pub since: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MergeDesk {
    pub holder: Option<MergeHolder>,
    pub queue: Vec<MergeRequest>,
    pub last: Option<LastMerge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MergeRequest {
    pub id: String,
    pub spec: String,
    pub bug: bool,
    pub requested: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MergeHolder {
    pub id: String,
    pub spec: String,
    pub bug: bool,
    pub requested: u64,
    pub granted: u64,
}

/// そのリポジトリの直前のマージ。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LastMerge {
    pub pr: String,
    pub sha: String,
    pub spec: String,
    pub at: u64,
}

/// 申し込みに並べる机の組。空の組は作れない。綴りは `test`・`sole`・`test,sole`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeskSet {
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "sole")]
    Sole,
    #[serde(rename = "test,sole")]
    TestSole,
}

impl DeskSet {
    /// 排他テストの机を含むか。
    pub const fn has_test(self) -> bool {
        matches!(self, DeskSet::Test | DeskSet::TestSole)
    }

    /// ひとり占めを含むか。
    pub const fn has_sole(self) -> bool {
        matches!(self, DeskSet::Sole | DeskSet::TestSole)
    }

    /// ASCII の綴り。状態ファイルの綴りと同じで、端末の文に使う。
    pub const fn as_str(self) -> &'static str {
        match self {
            DeskSet::Test => "test",
            DeskSet::Sole => "sole",
            DeskSet::TestSole => "test,sole",
        }
    }

    /// 2 つの机の有無から組を作る。どちらも無ければ `None`。
    pub const fn from_parts(test: bool, sole: bool) -> Option<DeskSet> {
        match (test, sole) {
            (true, false) => Some(DeskSet::Test),
            (false, true) => Some(DeskSet::Sole),
            (true, true) => Some(DeskSet::TestSole),
            (false, false) => None,
        }
    }
}

/// 机の申し込み 1 つ。`desks` は欠けたら読まない（中立の値が無い）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeskRequest {
    #[serde(default)]
    pub id: String,
    pub desks: DeskSet,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub requested: u64,
}

/// 排他テストの机の持ち主。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TestHolder {
    pub id: String,
    pub purpose: String,
    pub requested: u64,
    pub granted: u64,
}

/// 実装の机のひとり占めの持ち主。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SoleHolder {
    pub id: String,
    pub purpose: String,
    pub requested: u64,
    pub granted: u64,
    /// 「すでに走っている負荷テスト」として記録された印（停止要請を出さない）。
    pub running: bool,
    /// 番が来たときに止まっていた識別。
    pub stopped: Vec<String>,
}

/// 走っている待ち・見張りの記録。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitRecord {
    #[serde(default)]
    pub id: String,
    pub kind: WaitKind,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub pid: u32,
    #[serde(default)]
    pub since: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WaitKind {
    Watch,
    Merge,
    Request,
    Resume,
}

impl WaitKind {
    /// ASCII の名前。状態ファイルの綴りと同じで、居る印のファイル名と失敗の文に使う。
    pub const fn as_str(self) -> &'static str {
        match self {
            WaitKind::Watch => "watch",
            WaitKind::Merge => "merge",
            WaitKind::Request => "request",
            WaitKind::Resume => "resume",
        }
    }
}

/// 最近の出来事（回収・状態ファイルの復旧・`clear`・排他テストの机の空け）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    #[serde(default)]
    pub at: u64,
    pub kind: RecentKind,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecentKind {
    Reclaimed,
    Recovered,
    Cleared,
    /// 排他テストの机が空いた。`detail` は訳（`returned`・`stopped`・`reclaimed`・`cancelled`・
    /// `left`・`merged`）。
    TestFreed,
}

impl RecentKind {
    /// ASCII の名前。状態ファイルの綴りと同じで、`status` の表示に使う。
    pub const fn as_str(self) -> &'static str {
        match self {
            RecentKind::Reclaimed => "reclaimed",
            RecentKind::Recovered => "recovered",
            RecentKind::Cleared => "cleared",
            RecentKind::TestFreed => "test-freed",
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
