//! CueSheet — 相対時刻コマンド列（演出台本）と compile_sheet 関数。

use super::command::{ActorKey, Cue, CueCommand, CuePayload};
use super::schedule::Entry;
use serde::{Deserialize, Serialize};

/// 相対時刻の演出台本。
///
/// CueSheet は CueQueue にとっての "ソースコード" に相当し、
/// `compile_sheet()` を経て 0 ベース相対オフセットの `CompiledCue` に変換される。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CueSheet(Vec<Cue>);

impl CueSheet {
    /// start_time 昇順でソートして構築（安定ソート）
    ///
    /// NOTE(D3-V): NaN の start_time は比較が Equal 扱い（`unwrap_or`）となるため
    /// panic はしないが、NaN を含む場合のソート結果の位置は規定されない（P25 参照）。
    pub fn new(mut cues: Vec<Cue>) -> Self {
        cues.sort_by(|a, b| {
            a.start_time
                .partial_cmp(&b.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self(cues)
    }

    /// 全 Cue のスライスを取得
    pub fn cues(&self) -> &[Cue] {
        &self.0
    }

    /// 特定演者のキューのみをフィルタリング
    pub fn filter_by_actor(&self, key: &ActorKey) -> Vec<&Cue> {
        self.0.iter().filter(|cue| &cue.actor == key).collect()
    }

    /// CueSheet 内の全演者を重複なしで取得
    pub fn actors(&self) -> Vec<&ActorKey> {
        let mut actors: Vec<&ActorKey> = Vec::new();
        for cue in &self.0 {
            if !actors.contains(&&cue.actor) {
                actors.push(&cue.actor);
            }
        }
        actors
    }

    /// CueSheet が空か
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// CueSheet 内の Cue 数
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// コンパイル済みの 0 ベース相対オフセットエントリ。
pub struct CompiledCue {
    /// 0 ベース相対オフセット
    pub offset: f64,
    /// 対象演者
    pub actor: ActorKey,
    /// エントリ（Payload / Barrier / Routing）
    pub entry: Entry<CueCommand>,
}

/// CuePayload を Entry<CueCommand> に変換する。
///
/// - `CuePayload::Command` → `Entry::Payload`
/// - `CuePayload::Barrier` → `Entry::Barrier`
/// - `CuePayload::Routing` → `Entry::Routing`
impl CuePayload {
    /// Entry<CueCommand> への変換（compile_sheet 内で使用）。
    pub fn into_entry(self, offset: f64) -> Entry<CueCommand> {
        match self {
            CuePayload::Command(cmd) => Entry::Payload(offset, cmd),
            CuePayload::Barrier(kind) => Entry::Barrier(offset, kind),
            CuePayload::Routing(routing) => Entry::Routing(offset, routing),
        }
    }
}

/// CueSheet を 0 ベース相対オフセットに正規化。
///
/// `Cue::start_time` の最小値を 0 基準にし、`CuePayload::into_entry()` で Entry に変換。
/// 絶対時刻への変換は `TimedSchedule::new(start_time)` が担当。
///
/// NOTE(D3-V): start_time の有限性は検証されない（CueSheet 経路には DolaDocument の
/// validate() に相当する検証層がない — P25 参照）。非有限値の縮退は次のとおり
/// （いずれも tests/cue/sheet_test.rs の特性化テストで固定済み）:
/// - NaN の start_time は `f64::min` が NaN を無視するため最小値計算から脱落し、
///   当該 Cue のオフセットは NaN となる（TimedSchedule 側の NaN ハザードへ接続）。
/// - 全 Cue が +inf の場合、min = +inf となり inf - inf = NaN オフセットを生成する。
/// - 一部の Cue のみ +inf の場合、オフセットは +inf となり TimedSchedule 上で
///   永遠に配信されず is_completed() が true にならない（liveness 喪失）。
pub fn compile_sheet(sheet: &CueSheet) -> Vec<CompiledCue> {
    if sheet.is_empty() {
        return Vec::new();
    }

    // 最小 start_time を 0 基準に正規化
    let min_time = sheet
        .cues()
        .iter()
        .map(|c| c.start_time)
        .fold(f64::INFINITY, f64::min);

    sheet
        .cues()
        .iter()
        .map(|cue| {
            let offset = cue.start_time - min_time;
            let entry = cue.payload.clone().into_entry(offset);
            CompiledCue {
                offset,
                actor: cue.actor.clone(),
                entry,
            }
        })
        .collect()
}
