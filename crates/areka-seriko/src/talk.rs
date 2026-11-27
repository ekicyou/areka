//! talk: 文字が現れる時刻の列。
//!
//! `talk,数値` の引き金は「数値分の文字がバルーンに現れた時刻」で鳴る。その時刻を、文字の層と同じ式で
//! 絶対の時刻として持つ。時計も起点も持たない。
//!
//! [`TalkFeed`]: スコープ 1 つの「i 文字目が現れる時刻」の列。塊（文字の cue 1 件）ごとに、合図の発火
//! 時刻を塊の頭として、文字の層と同じ 1 本の式（[`chunk_reveal_offsets`]）で積む。序数は塊をまたいで
//! 続き、消去は現れなかった文字を捨てる。

use std::collections::VecDeque;

use areka_sakura::cluster::cluster_count;
use areka_sakura::reveal::chunk_reveal_offsets;
use dola::{TimeDelta, TimePoint, TimeSpan};

/// スコープ 1 つの文字の時刻の列（序数は塊をまたいで連続）。
#[derive(Default)]
pub(crate) struct TalkFeed {
    /// `times` の先頭の文字の序数（刈り込んだ分だけ進む）。
    base: u64,
    /// 文字が現れる時刻（単調非減少）。
    times: VecDeque<TimePoint>,
    /// 直前の文字の時刻（消去で `None` に戻る＝次の塊は塊の頭から）。刈り込みでは変わらない。
    last: Option<TimePoint>,
}

impl TalkFeed {
    /// 文字の cue 1 件（台詞の文字・選択肢の文字）を、書記素クラスタの数だけ積む。
    pub(crate) fn push_text(&mut self, fire_time: TimePoint, duration: TimeSpan, text: &str) {
        self.push_chunk(fire_time, duration, cluster_count(text));
    }

    /// 塊 1 つ（頭＝合図の発火時刻 `fire_time`・再生時間 `duration`・`count` 文字）を積む。
    ///
    /// 文字の時刻は文字の層と同じ [`chunk_reveal_offsets`] で求める（前の文字は直前の塊の最後の文字）。
    /// `count == 0` は何も積まない。
    pub(crate) fn push_chunk(&mut self, fire_time: TimePoint, duration: TimeSpan, count: usize) {
        let prev = self.last.map(|last| last - fire_time);
        for offset in chunk_reveal_offsets(prev, duration, count) {
            let r = fire_time + offset;
            self.times.push_back(r);
            self.last = Some(r);
        }
    }

    /// 消去の合図（発火時刻 `at`）: その時刻より後に現れる文字を捨て、次の塊を塊の頭から始めさせる。
    ///
    /// 文字の層は消去で未表示の文字ごと列を空にするので、現れなかった文字で口は動かない。残る文字は
    /// `at` 以前、次の塊の頭は `at` 以後なので、列は単調のまま（中断された前の台詞の残りは、新しい
    /// 台詞の頭の全消去でここで捨てる）。
    pub(crate) fn restart_chain(&mut self, at: TimePoint) {
        self.times
            .truncate(self.times.partition_point(|&r| r <= at));
        self.last = None;
    }

    /// 置き直しの知らせ: 効く時刻 `effective_at` より後の時刻（列の文字と「前の字の時刻」の欄）を
    /// `delta` だけ動かし、効く時刻より前になる物は効く時刻にする。効く時刻以前の時刻は書き換えない
    /// （列は単調のまま・刈り込んだ文字の序数はそのまま）。
    pub(crate) fn shift_after(&mut self, effective_at: TimePoint, delta: TimeDelta) {
        let shift = |t: TimePoint| {
            if t > effective_at {
                (t + delta).max(effective_at)
            } else {
                t
            }
        };
        for t in &mut self.times {
            *t = shift(*t);
        }
        self.last = self.last.map(shift);
    }

    /// 時刻 `t` までに現れた文字の数（序数で数える・ちょうどの時刻の文字は現れている）。
    pub(crate) fn revealed_until(&self, t: TimePoint) -> u64 {
        self.base + self.times.partition_point(|&r| r <= t) as u64
    }

    /// 序数 `g` の文字が現れる時刻（刈り込み済み・まだ無い序数は `None`）。
    pub(crate) fn time_of(&self, g: u64) -> Option<TimePoint> {
        let index = usize::try_from(g.checked_sub(self.base)?).ok()?;
        self.times.get(index).copied()
    }

    /// 序数 `keep_from` より前の文字を捨てる。残る文字・後から届く文字の序数は変わらない
    /// （在る文字の数より先へは進めない）。
    pub(crate) fn prune_before(&mut self, keep_from: u64) {
        let count = usize::try_from(keep_from.saturating_sub(self.base))
            .map_or(self.times.len(), |n| n.min(self.times.len()));
        self.times.drain(..count);
        self.base += count as u64;
    }
}

#[cfg(test)]
#[path = "talk_origin_tests.rs"]
mod origin_tests;

#[cfg(test)]
#[path = "talk_tests.rs"]
mod tests;
