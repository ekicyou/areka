//! CueSheet — 自己完結した台本（起点＋起点からの隔たりのコマンド列）と、
//! 時刻の表への canonical 変換 `to_talk_schedule`。

use super::command::{Cue, CuePayload, TalkCue};
use super::runtime::CueError;
use super::schedule::{Entry, TimedSchedule};
use crate::time::{TimePoint, TimeSpan};
use serde::{Deserialize, Serialize};

/// 自己完結した演出台本。
///
/// 台本は「起点（絶対の時刻）＋ 起点からの隔たりの Cue 列」の 2 層で時間を表す:
///
/// - `absolute_start_time` — 台本の時刻 0 の絶対の時刻。dispatch 時に押される。
/// - `Cue::start_time` — 起点からの隔たり（上流 compile が焼き込み済み）。
/// - `Cue::duration` — 当該 cue の presentation 占有の長さ（cue は区間
///   `[start_time, start_time + duration)`）。
///
/// 絶対の時刻は「起点＋隔たり」の整数の足し算だけで作る。broadcast された同一台本を
/// 受け取った任意の表現者が、互いに協調せずとも同一の絶対時刻を独立に算出できる。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CueSheet {
    /// 台本の起点（押されていなければ `None`）。
    #[serde(
        with = "crate::time::serde_ticks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    absolute_start_time: Option<TimePoint>,
    /// Cue 列（start_time 昇順）。
    cues: Vec<Cue>,
}

impl CueSheet {
    /// start_time 昇順に安定ソートして構築する（起点は押されていない）。
    ///
    /// 同じ瞬間の合図は台本の順のまま。
    pub fn new(mut cues: Vec<Cue>) -> Self {
        cues.sort_by_key(|cue| cue.start_time);
        Self {
            absolute_start_time: None,
            cues,
        }
    }

    /// 起点を押した台本を返す（dispatch 時に呼ぶ）。各 Cue の隔たりは書き換えない。
    pub fn with_absolute_start_time(mut self, origin: TimePoint) -> Self {
        self.absolute_start_time = Some(origin);
        self
    }

    /// 台本の起点（押されていなければ `None`）。
    pub fn absolute_start_time(&self) -> Option<TimePoint> {
        self.absolute_start_time
    }

    /// 指定 Cue の絶対の発火時刻（起点＋`start_time`）。起点が無ければ `None`。
    pub fn absolute_fire_time(&self, cue: &Cue) -> Option<TimePoint> {
        self.absolute_start_time
            .map(|origin| origin + cue.start_time)
    }

    /// 台本の絶対の終わり（起点＋[`relative_end`](Self::relative_end)）。起点が無ければ `None`。
    ///
    /// 変換した予定の [`TimedSchedule::occupancy_horizon`] と常に等しい。
    pub fn absolute_end_time(&self) -> Option<TimePoint> {
        self.absolute_start_time
            .map(|origin| origin + self.relative_end())
    }

    /// 占有の終わりの、起点からの隔たり（全部の合図の「開始＋長さ」の最大・空台本は 0）。
    ///
    /// cue は点でなく区間ゆえ、最後に発火する cue の時刻でなく占有区間の最大端である
    /// （配り終えた ≠ 再生し終えた）。
    pub fn relative_end(&self) -> TimeSpan {
        self.cues
            .iter()
            .map(|cue| cue.start_time + cue.duration)
            .max()
            .unwrap_or(TimeSpan::ZERO)
    }

    /// 全 Cue のスライスを取得
    pub fn cues(&self) -> &[Cue] {
        &self.cues
    }

    /// CueSheet が空か
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }
}

/// 台本 [`CueSheet`] を時刻の表 [`TimedSchedule`]`<`[`TalkCue`]`>` へ変換する
/// **唯一の canonical 変換**。起点の押されていない台本は `Err(CueError::OriginNotStamped)`。
///
/// - 表の起点は台本の起点。各 cue の `start_time` はエントリの隔たりとして**そのまま**
///   保存する（min 正規化しない＝先頭・単独の待ちが消えない）。
/// - 合図の発火時刻は「台本の起点＋`start_time`」（[`TalkCue::scheduled`]）。
/// - 同じ隔たりの合図は [`CueSheet::cues`] の記述順（FIFO）。
/// - 占有の終わりは [`CueSheet::relative_end`]。表の完了は配り終えた時でなく、占有の終わりで決まる。
///
/// ペイロード写像: [`CuePayload::Command`] → [`Entry::Payload`]、
/// [`CuePayload::Barrier`] → [`Entry::Barrier`]。
pub fn to_talk_schedule(sheet: &CueSheet) -> Result<TimedSchedule<TalkCue>, CueError> {
    let origin = sheet
        .absolute_start_time()
        .ok_or(CueError::OriginNotStamped)?;
    let mut schedule = TimedSchedule::with_horizon(origin, sheet.relative_end());

    // まとめて入れて台本の長さに比例する手間にする（1 件ずつの insert は長い台本で 2 乗になる）。
    schedule.insert_each(sheet.cues().iter().map(|cue| match &cue.payload {
        CuePayload::Command(command) => Entry::Payload(
            cue.start_time,
            TalkCue::scheduled(
                origin,
                cue.start_time,
                cue.actor.clone(),
                command.clone(),
                cue.duration,
            ),
        ),
        CuePayload::Barrier(kind) => Entry::Barrier(cue.start_time, kind.clone()),
    }));

    Ok(schedule)
}
