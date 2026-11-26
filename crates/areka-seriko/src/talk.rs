//! talk: 文字が現れる時刻の写し。
//!
//! `talk,数値` の引き金は「数値分の文字がバルーンに現れた時刻」で鳴る。その時刻を、文字の層と同じ式で
//! seriko の時計の上に写す。時計は持たず、渡された時計の読みから見積もるだけ。
//!
//! - [`TalkEpoch`]: 台本の 0 秒が seriko の時計のどの時刻に当たるか（起点）の見積もり。届いた cue ごとに
//!   「今 − cue の時刻」の最大を取る（文字の層の時刻源 `TalkClock::observe_cue` と同じ式）。
//! - [`TalkFeed`]: スコープ 1 つの「i 文字目が現れる台本の時刻」の列。塊（文字の cue 1 件）ごとに
//!   文字の層と同じ 1 本の式（[`chunk_reveal_offsets`]）で積む。序数は塊をまたいで続き、消去は現れな
//!   かった文字を捨てる。
//!
//! 文字の時刻は台本の時刻（`TimeSpan`）のまま持ち、壁時刻へは読むときに起点を足して写す。

use std::collections::VecDeque;

use areka_sakura::cluster::cluster_count;
use areka_sakura::reveal::chunk_reveal_offsets;
use dola::{TimePoint, TimeSpan};

use crate::timeline::elapsed_since;

/// 台本の 0 秒に当たる seriko の時計の読み（起点）の見積もり。プロセスに 1 つ。
#[derive(Default)]
pub(crate) struct TalkEpoch {
    /// 起点。cue が 1 つも届いていなければ `None`。
    epoch: Option<TimePoint>,
}

impl TalkEpoch {
    /// 届いた cue ごと（種類を問わない）に呼ぶ: `起点 = max(起点, 今 − cue の時刻)`。
    ///
    /// 新しいトークは届く時刻が前へ飛ぶので起点も前へ飛び、同じトークの中で早めに届いた cue
    /// （より小さい「今 − cue の時刻」）は最大に負けて起点を戻さない。
    pub(crate) fn observe(&mut self, at: TimeSpan, now: TimePoint) {
        let candidate = now - at;
        self.epoch = Some(self.epoch.map_or(candidate, |epoch| epoch.max(candidate)));
    }

    /// 今の台本の時刻（起点より前は 0・起点が無ければ `None`）。
    pub(crate) fn talk_time(&self, now: TimePoint) -> Option<TimeSpan> {
        self.epoch.map(|epoch| elapsed_since(now, epoch))
    }

    /// 台本の時刻を壁時刻に写す。起点が無ければ `None`。
    pub(crate) fn wall(&self, r: TimeSpan) -> Option<TimePoint> {
        self.epoch.map(|epoch| epoch + r)
    }
}

/// スコープ 1 つの文字の時刻の列（序数は塊をまたいで連続）。
#[derive(Default)]
pub(crate) struct TalkFeed {
    /// `times` の先頭の文字の序数（刈り込んだ分だけ進む）。
    base: u64,
    /// 文字が現れる台本の時刻（単調非減少）。
    times: VecDeque<TimeSpan>,
    /// 直前の文字の時刻（消去で `None` に戻る＝次の塊は塊の頭から）。刈り込みでは変わらない。
    last: Option<TimeSpan>,
}

impl TalkFeed {
    /// 文字の cue 1 件（台詞の文字・選択肢の文字）を、書記素クラスタの数だけ積む。
    pub(crate) fn push_text(&mut self, at: TimeSpan, duration: TimeSpan, text: &str) {
        self.push_chunk(at, duration, cluster_count(text));
    }

    /// 塊 1 つ（頭 `at`・再生時間 `duration`・`count` 文字）を積む。
    ///
    /// 文字の時刻は文字の層と同じ [`chunk_reveal_offsets`] で求める（前の文字は直前の塊の最後の文字）。
    /// `count == 0` は何も積まない。
    pub(crate) fn push_chunk(&mut self, at: TimeSpan, duration: TimeSpan, count: usize) {
        let prev = self.last.map(|last| last - at);
        for offset in chunk_reveal_offsets(prev, duration, count) {
            let r = at + offset;
            self.times.push_back(r);
            self.last = Some(r);
        }
    }

    /// 消去の知らせ（台本の時刻 `at`）: その時刻までに現れていない文字を捨て、次の塊を塊の頭から
    /// 始めさせる。
    ///
    /// 文字の層は消去で未表示の文字ごと列を空にするので、現れなかった文字で口は動かない。残る文字は
    /// `at` 以前、次の塊の頭は `at` 以後なので、列は単調のまま（新しい台詞の頭の全消去は
    /// 台本の 0 秒で届き、中断された前の台詞の残りをここで捨てる）。
    pub(crate) fn restart_chain(&mut self, at: TimeSpan) {
        self.times
            .truncate(self.times.partition_point(|&r| r <= at));
        self.last = None;
    }

    /// 台本の時刻 `t` までに現れた文字の数（序数で数える・ちょうどの時刻の文字は現れている）。
    pub(crate) fn revealed_until(&self, t: TimeSpan) -> u64 {
        self.base + self.times.partition_point(|&r| r <= t) as u64
    }

    /// 序数 `g` の文字が現れる台本の時刻（刈り込み済み・まだ無い序数は `None`）。
    pub(crate) fn time_of(&self, g: u64) -> Option<TimeSpan> {
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
#[path = "talk_tests.rs"]
mod tests;
