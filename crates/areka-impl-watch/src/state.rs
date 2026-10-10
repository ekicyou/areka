//! 状態の型（版 1）・空の状態・参加者の状態の列挙。
//!
//! 状態ファイル `state.json` はこの [`State`] 1 つをそのまま書いたもの（参加者・リポジトリごとの
//! マージの机・負荷テストの机・走っている待ち・最近の出来事が全部ここに在る）。時刻は全部 UNIX 秒。
//!
//! 不変（守るのは `plan`）: 1 つの識別は、1 つのリポジトリのマージの待ち行列に高々 1 回、
//! 負荷テストの待ち行列に高々 1 回。持ち主と待ち行列に同時には居ない。`waits` は
//! `(id, kind)` で高々 1 件。
//!
//! 項目には `#[serde(default)]` が付いていて、同じ版の中で項目が増えても古いファイルが読める。
//! 例外は記録の種類（[`WaitRecord::kind`]・[`Recent::kind`]）で、欠けたときに入れる中立の値が
//! 無いので、欠けていれば形の合わないファイルとして読まない。

// 使い手（plan・store・status・wait）が載るまで、本番のビルドではここが未使用になる。
// 全部が使われるとこの行が「満たされない expect」の警告になるので、そのとき外す。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "使い手のモジュールは後のタスクで載る")
)]

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// この実行ファイルの知っている状態ファイルの版。項目の意味を変える・消すときに上げる。
pub const VERSION: u32 = 1;

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
    pub load: LoadDesk,
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadDesk {
    pub holder: Option<LoadHolder>,
    pub queue: Vec<LoadRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadRequest {
    pub id: String,
    pub purpose: String,
    pub requested: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadHolder {
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
    Load,
    Resume,
}

/// 最近の出来事（回収・状態ファイルの復旧・`clear`）。
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
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
