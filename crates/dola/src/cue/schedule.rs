//! `TimedSchedule<T>` — 起点と相対の時刻から、届いた項目を 1 件ずつ取り出す時刻の表。
//!
//! `Entry<T>` により Payload / Barrier を型レベルで 2 種分離する。取り出し口は
//! [`TimedSchedule::pop_due`] の 1 本で、溜めを持たない。起点は再生の途中で置き直せる。

use super::command::BarrierKind;
use crate::time::{TimeDelta, TimePoint, TimeSpan};

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

/// 時刻の表から取り出した 1 件。
#[derive(Clone, Debug, PartialEq)]
pub enum Due<T> {
    /// 中身と、取り出した時の起点で決めた絶対の発火時刻と、項目の相対の時刻（台本の時刻）。
    Payload {
        fire_time: TimePoint,
        at: TimeSpan,
        payload: T,
    },
    /// 区切りに着いた（時刻の表は、外から解かれるまで止まる）。
    Barrier(BarrierKind),
}

/// 起点の知らせ。「`effective_at` から先、台本の時刻 0 は `origin` である」。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginNotice {
    /// この起点が効き始める時刻。置き直しでは、呼び手が渡した現在の時刻。最初の起点では、その起点。
    pub effective_at: TimePoint,
    /// 新しい起点。
    pub origin: TimePoint,
    /// 前の起点。最初の起点の知らせでは `None`。
    pub previous: Option<TimePoint>,
}

impl OriginNotice {
    /// 最初の起点の知らせ（`effective_at` は `origin`・`previous` は `None`）。
    pub fn first(origin: TimePoint) -> Self {
        Self {
            effective_at: origin,
            origin,
            previous: None,
        }
    }

    /// 動いた長さ（`origin − previous`）。負は前へ（台本の時刻は進む）。最初の起点では `None`。
    pub fn delta(&self) -> Option<TimeDelta> {
        self.previous.map(|previous| self.origin - previous)
    }
}

/// 起点と相対の時刻から、届いた項目を 1 件ずつ取り出す時刻の表。
///
/// 届いたかの判定は「起点＋相対 ≤ 今」。完了も占有の終わりも、その時の起点で答える。
#[derive(Debug)]
pub struct TimedSchedule<T> {
    /// 起点（台本の時刻 0 の絶対の時刻・100ns）。
    ///
    /// `TimePoint` の下の端より前へ置き直されても比べの結果が変わらないよう、128 ビットで持つ。
    origin: i128,
    /// 降順ソート（0 ベース相対オフセット）。末尾 pop で O(1) 消費。
    entries: Vec<Entry<T>>,
    /// 現在停止中のバリア
    current_barrier: Option<BarrierKind>,
    /// 占有の終わり（起点からの隔たり）。台本の全 cue の「開始＋長さ」の最大。
    ///
    /// cue は点でなく区間ゆえ、配り終えただけでは完了でなく、今がここに届いて初めて完了とみなす。
    horizon: TimeSpan,
}

impl<T> TimedSchedule<T> {
    /// 起点 `origin` でスケジュールを構築（占有 horizon は 0）。
    #[cfg(test)]
    pub(crate) fn new(origin: TimePoint) -> Self {
        Self::with_horizon(origin, TimeSpan::ZERO)
    }

    /// 起点 `origin` と占有 horizon（起点からの隔たり）を指定してスケジュールを構築。
    pub fn with_horizon(origin: TimePoint, horizon: TimeSpan) -> Self {
        Self {
            origin: i128::from(origin.as_ticks()),
            entries: Vec::new(),
            current_barrier: None,
            horizon,
        }
    }

    /// エントリを時刻順ソート維持で挿入（起点からの隔たり）。
    /// 降順ソートを維持する。テストの手組み用（本番は `insert_each`）。
    #[cfg(test)]
    pub(crate) fn insert(&mut self, entry: Entry<T>) {
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

    /// `now` までに届いた次の 1 件。届いた物が無い・区切りで止まっている時は `None`。
    ///
    /// 発火時刻は、取り出した時の起点＋相対で決まる（`now` ではない）。区切りを返した後は、
    /// [`notify_barrier_resolved`](Self::notify_barrier_resolved) まで何も返さない。
    pub fn pop_due(&mut self, now: TimePoint) -> Option<Due<T>> {
        if self.current_barrier.is_some() {
            return None;
        }
        let origin = self.origin;
        let now = i128::from(now.as_ticks());
        // 選択待ちの `timeout` は上位層（kanade の期限の判定）へ運ぶ指令で、時刻の表はこれで
        // 区切りを飛ばしも解きもしない。
        match self
            .entries
            .pop_if(|entry| absolute(origin, entry.offset()) <= now)?
        {
            Entry::Payload(at, payload) => Some(Due::Payload {
                fire_time: clamp(absolute(origin, at)),
                at,
                payload,
            }),
            Entry::Barrier(_, kind) => {
                self.current_barrier = Some(kind.clone());
                Some(Due::Barrier(kind))
            }
        }
    }

    /// 「時刻 `at` が、台本の時刻 `script_time` である」ように起点を置く。
    ///
    /// `now` は呼び手の現在の時刻で、知らせの効く時刻になる（表は覚えない）。
    /// 起点が変わらなければ `None`。区切りで止まっている間も受け付ける。
    pub fn reanchor(
        &mut self,
        now: TimePoint,
        at: TimePoint,
        script_time: TimeSpan,
    ) -> Option<OriginNotice> {
        self.set_origin(
            now,
            i128::from(at.as_ticks()) - i128::from(script_time.as_ticks()),
        )
    }

    /// 起点を `delta` だけ動かす（負は前へ）。`now` と戻り値は [`reanchor`](Self::reanchor) と同じ。
    pub fn shift_origin(&mut self, now: TimePoint, delta: TimeDelta) -> Option<OriginNotice> {
        self.set_origin(
            now,
            self.origin.saturating_add(i128::from(delta.as_ticks())),
        )
    }

    fn set_origin(&mut self, now: TimePoint, origin: i128) -> Option<OriginNotice> {
        if origin == self.origin {
            return None;
        }
        let previous = std::mem::replace(&mut self.origin, origin);
        Some(OriginNotice {
            effective_at: now,
            origin: clamp(origin),
            previous: Some(clamp(previous)),
        })
    }

    /// 今の起点（`TimePoint` の範囲の外では、端へ張り付いた値）。
    pub fn origin(&self) -> TimePoint {
        clamp(self.origin)
    }

    /// 現在停止中のバリア種別を照会（UI 表示用）。
    pub fn current_barrier(&self) -> Option<&BarrierKind> {
        self.current_barrier.as_ref()
    }

    /// バリア解除プッシュ通知（外部イベント駆動）。非バリア時は no-op。
    pub fn notify_barrier_resolved(&mut self) {
        self.current_barrier = None;
    }

    /// 残りエントリ数
    pub fn remaining(&self) -> usize {
        self.entries.len()
    }

    /// まだ取り出していない中身（区切りを除く）を、配る順に
    /// 「今の起点での予定の発火時刻・相対の時刻・中身」で返す。
    pub(crate) fn pending_payloads(&self) -> impl Iterator<Item = (TimePoint, TimeSpan, &T)> {
        let origin = self.origin;
        self.entries
            .iter()
            .rev()
            .filter_map(move |entry| match entry {
                Entry::Payload(at, payload) => Some((clamp(absolute(origin, *at)), *at, payload)),
                Entry::Barrier(..) => None,
            })
    }

    /// 全部を取り出し、区切りが無く、`now` が占有の終わりに届いている。
    ///
    /// 状態として持たず、その時の起点で答える（起点を置き直すと答えも変わる）。
    pub fn is_completed_at(&self, now: TimePoint) -> bool {
        self.entries.is_empty()
            && self.current_barrier.is_none()
            && i128::from(now.as_ticks()) >= absolute(self.origin, self.horizon)
    }

    /// 占有の終わりの絶対時刻（起点＋占有の終わり）。起点を置き直すと動く。
    ///
    /// [`crate::cue::to_talk_schedule`] で組んだ表では、起点を置き直すまで台本の
    /// [`CueSheet::absolute_end_time`](crate::cue::CueSheet::absolute_end_time) と同じ値になる。
    pub fn occupancy_horizon(&self) -> TimePoint {
        clamp(absolute(self.origin, self.horizon))
    }
}

fn absolute(origin: i128, offset: TimeSpan) -> i128 {
    origin.saturating_add(i128::from(offset.as_ticks()))
}

fn clamp(ticks: i128) -> TimePoint {
    TimePoint::from_ticks(ticks.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reorigin_tests;
