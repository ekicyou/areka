//! `TimedSchedule<T>` — 0 ベース相対オフセットの汎用配信エンジン。
//!
//! `Entry<T>` により Payload / Barrier を型レベルで 2 種分離する。
//! `tick()` + `ready()` の 2 フェーズ API で `DolaRuntime` の `tick()` + `last_result()` と対称。

use std::fmt::Debug;

use super::command::BarrierKind;
use crate::time::{TimePoint, TimeSpan};

// ============================================================================
// Entry<T> — エントリの型レベル 2 種分離
// ============================================================================

/// エントリの型レベル 2 種分離。
/// 長さ = スケジュールの起点からの隔たり。
#[derive(Clone, Debug)]
pub enum Entry<T> {
    /// 時刻付きデータペイロード（実行すべきコマンド）
    Payload(TimeSpan, T),
    /// 時刻付きバリア（進行停止点、TimedSchedule が消費）
    Barrier(TimeSpan, BarrierKind),
}

impl<T> Entry<T> {
    /// エントリの時刻オフセットを取得する。
    pub fn offset(&self) -> TimeSpan {
        match self {
            Entry::Payload(t, _) => *t,
            Entry::Barrier(t, _) => *t,
        }
    }
}

// ============================================================================
// TimedSchedule<T>
// ============================================================================

/// 0 ベース相対オフセットの汎用配信エンジン。
///
/// `Entry<T>` により Payload / Barrier を型レベルで 2 種分離する。
/// ジェネリクス T は Payload(TimeSpan, T) のみに適用される。
/// Barrier(BarrierKind) は T に関わらず固定型。
#[derive(Debug)]
pub struct TimedSchedule<T> {
    /// 起点（台本の時刻 0 の絶対の時刻）
    start_time: TimePoint,
    /// 降順ソート（0 ベース相対オフセット）。末尾 pop で O(1) 消費。
    entries: Vec<Entry<T>>,
    /// tick() で収集した Payload
    ready_buffer: Vec<T>,
    /// 現在停止中のバリア
    current_barrier: Option<BarrierKind>,
    /// 前回 tick の絶対時刻（構築の直後はアンカー `start_time`）。
    ///
    /// 到達・完了の判定はすべて「起点＋相対」の整数の足し算で作った絶対時刻とこの値を
    /// 比べる（絶対の発火時刻の定義 [`CueSheet::absolute_fire_time`](crate::cue::CueSheet::absolute_fire_time)
    /// と同じ形）。ちょうどその時刻の tick で届く。
    current_time: TimePoint,
    /// 占有終了 horizon（相対オフセット・0 ベース）。台本の全 cue の
    /// `max(start_time + duration)` を canonical 変換が焼き込む。
    ///
    /// cue は点でなく区間 `[start, start+duration)` ゆえ、**entries を配り終えた**（＝
    /// 最後の cue の**配送時刻**に達した）だけでは talk は完了でなく、**現在時刻が
    /// この horizon に達して初めて**占有終了＝完了とみなす（末尾 Wait・最終 Text の
    /// duration を終端で落とさない・早期終了しない）。
    ///
    /// テストの手組みの `new` の既定は 0。`current_time` は常に起点以上（`tick` の手前ガード）
    /// ゆえ horizon=0 は `current_time >= start_time + horizon` を常に満たし、
    /// [`is_completed`](Self::is_completed) は「entries 枯渇かつ barrier なし」と一致する。
    horizon: TimeSpan,
}

impl<T: Clone + Debug> TimedSchedule<T> {
    /// 起点 `origin` でスケジュールを構築（占有 horizon は 0）。
    #[cfg(test)]
    pub(crate) fn new(origin: TimePoint) -> Self {
        Self::with_horizon(origin, TimeSpan::ZERO)
    }

    /// 起点 `origin` と占有 horizon（起点からの隔たり）を指定してスケジュールを構築。
    ///
    /// canonical 変換（[`crate::cue::to_talk_schedule`]）が台本の占有 horizon を焼き込むために
    /// 使う。手組みスケジュール（テストの手組みの `new`）は horizon=0。
    pub fn with_horizon(origin: TimePoint, horizon: TimeSpan) -> Self {
        Self {
            start_time: origin,
            entries: Vec::new(),
            ready_buffer: Vec::new(),
            current_barrier: None,
            current_time: origin,
            horizon,
        }
    }

    /// エントリを時刻順ソート維持で挿入（起点からの隔たり）。
    /// 降順ソートを維持する。テストの手組み用（本番は `insert_each`）。
    #[cfg(test)]
    pub(crate) fn insert(&mut self, entry: Entry<T>) {
        // 降順ソート: 大きいオフセットが先頭、小さいオフセットが末尾
        let offset = entry.offset();
        let pos = self.entries.partition_point(|e| e.offset() > offset);
        self.entries.insert(pos, entry);
    }

    /// `entries` を順に `insert` したのと同じ並びを、まとめて作る（同一オフセットは
    /// 後に入れたものが前＝先に入れたものから配る FIFO）。`insert` を 1 件ずつ呼ぶと昇順の台本で
    /// 毎回先頭へ挿すので全体が台本の長さの 2 乗になる。ここは逆順に積んで安定ソートする
    /// （昇順の台本なら既に降順の 1 本の並びで、ソートは 1 回の走査で済む）。
    pub fn insert_each(&mut self, entries: impl IntoIterator<Item = Entry<T>>) {
        let mut all: Vec<Entry<T>> = entries.into_iter().collect();
        all.reverse();
        all.append(&mut self.entries);
        all.sort_by_key(|entry| std::cmp::Reverse(entry.offset()));
        self.entries = all;
    }

    // ── 2 フェーズ API（DolaRuntime の tick/last_result と対称） ──

    /// Phase 1: 新たな時刻を入力としてランタイムの時刻を進める。
    ///
    /// `current_time` は絶対時刻。各エントリは「起点＋隔たり ≤ `current_time`」で届く
    /// （相対へ引き戻さない・`current_time` 欄の説明を参照）。
    ///
    /// 時刻到達済みの Payload を `ready_buffer` に蒐集しながら進行。Barrier 到達（外部解決が
    /// 必要）または末尾到達で停止。冪等（同一時刻の再呼び出し安全）。
    pub fn tick(&mut self, current_time: TimePoint) {
        if current_time < self.start_time {
            return;
        }

        if current_time <= self.current_time && !self.ready_buffer.is_empty() {
            return;
        }

        if self.current_barrier.is_some() {
            return;
        }

        self.current_time = current_time;
        self.ready_buffer.clear();

        // entries は降順ソート → 末尾から時刻到達を消費
        let start_time = self.start_time;
        while let Some(entry) = self
            .entries
            .pop_if(|e| start_time + e.offset() <= current_time)
        {
            match entry {
                Entry::Payload(_, payload) => {
                    self.ready_buffer.push(payload);
                }
                // 選択待ちの `timeout` は上位層（kanade の期限の判定）へ運ぶ指令で、再生層はこれで
                // 区切りを飛ばしも解きもしない。解けるのは外からの `notify_barrier_resolved` だけ。
                Entry::Barrier(_, kind) => {
                    self.current_barrier = Some(kind);
                    return;
                }
            }
        }
    }

    /// Phase 2: 直前の tick() で収集された Payload スライスを返す。
    /// 次の tick() まで何度でも参照可能（`notify_barrier_resolved` でも空になる）。
    pub fn ready(&self) -> &[T] {
        &self.ready_buffer
    }

    // ── バリア管理 ──

    /// 現在停止中のバリア種別を照会（UI 表示用）。
    pub fn current_barrier(&self) -> Option<&BarrierKind> {
        self.current_barrier.as_ref()
    }

    /// バリア解除プッシュ通知（外部イベント駆動）。非バリア時は no-op。
    pub fn notify_barrier_resolved(&mut self) {
        if self.current_barrier.is_none() {
            return; // 非バリア時は no-op
        }
        self.current_barrier = None;
        // 集めは区切りの tick で配り済み。空にして、同じ時刻の次の tick が冪等の守りに止められず
        // 区切りの後ろを集められるようにする（解決が次の Tick より先に来る形）。
        self.ready_buffer.clear();
    }

    // ── ユーティリティ ──

    /// 残りエントリ数
    pub fn remaining(&self) -> usize {
        self.entries.len()
    }

    /// まだ配っていない中身（区切りを除く）を配る順に返す。
    ///
    /// `entries` は降順（末尾から取り出す）ゆえ後ろから読む。
    pub(crate) fn pending_payloads(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().rev().filter_map(|entry| match entry {
            Entry::Payload(_, payload) => Some(payload),
            Entry::Barrier(..) => None,
        })
    }

    /// 占有終了（完了）判定: 全エントリ消費済み・バリア中でない・かつ現在時刻が
    /// 占有 horizon に達している。
    ///
    /// entries 枯渇（＝最後の cue の**配送時刻**到達）だけでは完了とせず、cue を区間
    /// `[start, start+duration)` と見て**占有終了 horizon**（`max(start+duration)`）到達まで
    /// 完了扱いしない（末尾 Wait・最終 Text の duration を終端で落とさない）。
    ///
    /// 閾値は [`occupancy_horizon`](Self::occupancy_horizon) と同じ `start_time` と `horizon` の和。
    ///
    /// horizon=0（テストの手組みの `new` 既定）では `current_time >= start_time` が常に真ゆえ
    /// 「entries 枯渇かつ barrier なし」と一致する。
    pub fn is_completed(&self) -> bool {
        self.entries.is_empty()
            && self.current_barrier.is_none()
            && self.current_time >= self.occupancy_horizon()
    }
}

/// ペイロード型 T に依存しない照会（エントリを触らないため `Clone + Debug` 境界を要さない）。
impl<T> TimedSchedule<T> {
    /// 占有終了 horizon の**絶対時刻** = 起点と相対 horizon の整数の和。
    ///
    /// [`is_completed`](Self::is_completed) が閾値として見ている占有区間の終端を、
    /// 上位層が絶対時刻として照会するための口。値は [`tick`](Self::tick) の
    /// 進行では変化しない。[`crate::cue::to_talk_schedule`] で組んだ予定では、台本の
    /// [`CueSheet::absolute_end_time`](crate::cue::CueSheet::absolute_end_time) と同じ値になる
    /// （両方が同じ [`CueSheet::relative_end`](crate::cue::CueSheet::relative_end) と同じ和を使う）。
    pub fn occupancy_horizon(&self) -> TimePoint {
        self.start_time + self.horizon
    }
}

#[cfg(test)]
mod tests;
