//! CueSheet — 自己完結した絶対時刻台本（絶対開始時刻＋相対時刻コマンド列）と
//! compile_sheet 関数。

use super::command::{ActorKey, Cue, CueCommand, CuePayload, TalkCue};
use super::schedule::{Entry, TimedSchedule};
use serde::{Deserialize, Serialize};

/// 自己完結した絶対時刻の演出台本。
///
/// 台本は「絶対開始時刻（アンカー）＋ 相対時刻の Cue 列」の 2 層で時間を表現する:
///
/// - `absolute_start_time` — talk が再生を開始する壁時計の絶対秒。dispatch 時に刻印される。
/// - `Cue::start_time` — 台本先頭を 0 とする相対秒（上流 compile が焼き込み済み）。
/// - `Cue::duration` — 当該 cue の presentation 占有秒（cue は区間
///   `[start_time, start_time + duration)`）。
///
/// この 2 層により、**台本 1 枚のみから**各 cue の絶対発火時刻
/// （[`absolute_fire_time`](Self::absolute_fire_time)）と talk の絶対終了時刻
/// （[`absolute_end_time`](Self::absolute_end_time)）が復元できる。broadcast された同一台本を
/// 受け取った任意の表現者（プロセス境界を跨ぐ場合を含む）が、互いに協調せずとも同一の
/// 絶対時刻を独立に算出できる——これが dola の「同一台本を同一絶対時刻で」の物理的な形。
///
/// `absolute_start_time` は `#[serde(default)]`（未刻印は 0.0）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CueSheet {
    /// talk が再生を開始する絶対秒（dispatch 時に刻印されるアンカー）。
    #[serde(default)]
    absolute_start_time: f64,
    /// 相対時刻の Cue 列（start_time 昇順）。
    cues: Vec<Cue>,
}

impl CueSheet {
    /// start_time 昇順でソートして構築（安定ソート）
    ///
    /// 絶対開始時刻は未刻印（0.0）。dispatch 時に
    /// [`with_absolute_start_time`](Self::with_absolute_start_time) で刻印する。
    ///
    /// NOTE(D3-V): NaN の start_time は比較が Equal 扱い（`unwrap_or`）となるため
    /// panic はしないが、NaN を含む場合のソート結果の位置は規定されない（P25 参照）。
    pub fn new(mut cues: Vec<Cue>) -> Self {
        cues.sort_by(|a, b| {
            a.start_time
                .partial_cmp(&b.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self {
            absolute_start_time: 0.0,
            cues,
        }
    }

    /// 絶対開始時刻（アンカー）を刻印した台本を返す（dispatch 時に呼ぶ）。
    ///
    /// 刻印は各 Cue の**相対**時刻（`start_time`/`duration`）を書き換えない——
    /// 台本内の整列は上流 compile が焼き込み済みで、アンカーはその上に載る。
    pub fn with_absolute_start_time(mut self, absolute_start_time: f64) -> Self {
        self.absolute_start_time = absolute_start_time;
        self
    }

    /// 台本の絶対開始時刻（未刻印なら 0.0）。
    pub fn absolute_start_time(&self) -> f64 {
        self.absolute_start_time
    }

    /// 指定 Cue の絶対発火時刻 = `absolute_start_time + start_time`。
    ///
    /// dola は時刻を導出・変換せず、この和のみが絶対時刻の定義（配送時に別の時刻を
    /// 算出すると表現者ごとに独立計算となり desync するため）。
    pub fn absolute_fire_time(&self, cue: &Cue) -> f64 {
        self.absolute_start_time + cue.start_time
    }

    /// talk の絶対終了時刻 = `absolute_start_time + max(start_time + duration)`。
    ///
    /// cue は点でなく区間 `[start_time, start_time + duration)` ゆえ、終了時刻は
    /// 「最後に発火する cue の時刻」でなく**占有区間の最大端**。末尾 Wait・最終 Text の
    /// duration もここに含まれる（配り終えた ≠ 再生し終えた）。空台本は時間を占有せず
    /// アンカーそのものを返す（相対 horizon の下限は台本先頭 0.0）。
    ///
    /// NOTE: duration の非有限（NaN/±inf）・負値の clamp は台本→スケジュールの
    /// canonical 変換（dola ingress の単一権威）が担う。本メソッドは検証済みの値を
    /// 前提に和と max を取るのみ（`f64::max` は NaN を読み飛ばす）。
    pub fn absolute_end_time(&self) -> f64 {
        let relative_horizon = self
            .cues
            .iter()
            .map(|cue| cue.start_time + cue.duration)
            .fold(0.0_f64, f64::max);
        self.absolute_start_time + relative_horizon
    }

    /// 全 Cue のスライスを取得
    pub fn cues(&self) -> &[Cue] {
        &self.cues
    }

    /// 特定演者のキューのみをフィルタリング
    pub fn filter_by_actor(&self, key: &ActorKey) -> Vec<&Cue> {
        self.cues.iter().filter(|cue| &cue.actor == key).collect()
    }

    /// CueSheet 内の全演者を重複なしで取得
    pub fn actors(&self) -> Vec<&ActorKey> {
        let mut actors: Vec<&ActorKey> = Vec::new();
        for cue in &self.cues {
            if !actors.contains(&&cue.actor) {
                actors.push(&cue.actor);
            }
        }
        actors
    }

    /// CueSheet が空か
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }

    /// CueSheet 内の Cue 数
    pub fn len(&self) -> usize {
        self.cues.len()
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

/// duration を「有限かつ非負」へ丸める（dola ingress の**単一権威**・R1.8）。
///
/// 非有限（NaN/±inf）・負値はすべて 0（瞬時）とみなす。+inf も畳み込む——horizon が ∞ に
/// なると talk が永遠に終わらない同種の病理ゆえ（開発者決裁）。この clamp を通した値のみが
/// 配送エンベロープ [`TalkCue::duration`] と占有 horizon の両方へ流れるため、検証されない
/// 台本（プロセス跨ぎの自作 cue を含む）でも horizon 完了判定と下流 reveal の可視グリフ数
/// 計算が全域で well-defined になる。
fn clamp_duration(duration: f64) -> f64 {
    if duration.is_finite() && duration >= 0.0 {
        duration
    } else {
        0.0
    }
}

/// 台本 [`CueSheet`] を時刻駆動可能な [`TimedSchedule`]`<`[`TalkCue`]`>` へ変換する
/// **唯一の canonical 変換**（R11.1・dola ingress の単一権威）。
///
/// 旧 [`compile_sheet`]（min 正規化＝**先頭待ちを食う**）と、旧 sakura `to_schedule`（独自実装）
/// の二重を廃した 1 本。以下を同時に成立させる:
///
/// - **絶対アンカーの継承**: スケジュールの `start_time` に [`CueSheet::absolute_start_time`]
///   を用いる（dispatch 刻印前は 0.0）。各 cue の相対 `start_time` はエントリオフセットとして
///   **そのまま**保存する（min 正規化しない＝先頭・単独の待ちが消えない）。
/// - **同一 at の FIFO**: [`CueSheet::cues`] の記述順に 1 件ずつ [`TimedSchedule::insert`] で
///   挿入する（`extend` の一括再ソートは同一オフセット群を逆順化するため使わない）。
/// - **duration の clamp**: 各 cue の duration を [`clamp_duration`] で有限・非負へ丸め、
///   その**同一値**を配送エンベロープ [`TalkCue::duration`] と占有 horizon の両方へ用いる。
/// - **占有 horizon の算出**: 全 cue の `max(start_time + clamp(duration))`（空台本は 0.0）を
///   相対 horizon としてスケジュールへ保持し、[`TimedSchedule::is_completed`] が配送完了でなく
///   **占有終了**で真になるようにする（末尾待ちの duration を終端で落とさない・R2.5/D6）。
///
/// ペイロード写像: [`CuePayload::Command`] → [`Entry::Payload`]（[`TalkCue`] を組む）、
/// [`CuePayload::Barrier`] → [`Entry::Barrier`]、[`CuePayload::Routing`] → [`Entry::Routing`]。
/// Barrier/Routing は presentation でなく（R1.6）envelope duration を持たないが、その
/// `start_time` は horizon の fold に一律に寄与する（duration 相当は 0）。
pub fn to_talk_schedule(sheet: &CueSheet) -> TimedSchedule<TalkCue> {
    // 相対占有 horizon = max(start_time + clamp(duration))・空台本は 0.0。
    // fold の種を 0.0 とするため、NaN の start_time を持つ cue は f64::max が読み飛ばす。
    let horizon = sheet
        .cues()
        .iter()
        .map(|cue| cue.start_time + clamp_duration(cue.duration))
        .fold(0.0_f64, f64::max);

    let mut schedule = TimedSchedule::with_horizon(sheet.absolute_start_time(), horizon);

    // per-cue insert（extend 禁止）: 同一 at 群を記述順（FIFO）で保つ。
    for cue in sheet.cues() {
        match &cue.payload {
            CuePayload::Command(command) => {
                let talk_cue = TalkCue {
                    at: cue.start_time,
                    actor: cue.actor.clone(),
                    command: command.clone(),
                    duration: clamp_duration(cue.duration),
                };
                schedule.insert(Entry::Payload(cue.start_time, talk_cue));
            }
            CuePayload::Barrier(kind) => {
                schedule.insert(Entry::Barrier(cue.start_time, kind.clone()));
            }
            CuePayload::Routing(routing) => {
                schedule.insert(Entry::Routing(cue.start_time, routing.clone()));
            }
        }
    }

    schedule
}
