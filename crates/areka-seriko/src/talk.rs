//! talk: 文字が現れる時刻の写し（spec: areka-P0-seriko-trigger-intervals 要件 4.2・4.3・4.10・6.3・6.4）。
//!
//! `talk,数値` の引き金は「数値分の文字がバルーンに現れた時刻」で鳴る。その時刻を、文字の層と同じ式で
//! seriko の時計の上に写す。時計は持たず、渡された時計の読み（ms）から見積もるだけ（要件 6.3）。
//!
//! - [`TalkEpoch`]: 台本の 0 秒が seriko の時計の何 ms に当たるか（起点）の見積もり。届いた cue ごとに
//!   「今 − cue の時刻」の最大を取る（文字の層の時刻源 `TalkClock::observe_cue` と同じ式）。
//! - [`TalkFeed`]: スコープ 1 つの「i 文字目が現れる台本の秒」の列。塊（文字の cue 1 件）ごとに
//!   `r_i = max(r_{i−1} + 1 字の間隔, 塊の頭)` で積む（文字の層の `RevealSchedule::extend_chunk` と
//!   同じ式・同じ計算の順）。序数は塊をまたいで続き、消去は現れなかった文字を捨てる。
//!
//! 文字の時刻は台本の秒（`f64`）のまま持ち、壁時刻（ms）へは読むときに写す（1 字の間隔の端数を
//! 失わない）。文字の層の式が変わったら、ここも同じに変える（写しであって、共有ではない）。

use std::collections::VecDeque;

use areka_sakura::cluster::cluster_count;

/// 台本の 0 秒に当たる seriko の時計の読み（起点・ms）の見積もり。プロセスに 1 つ。
#[derive(Default)]
pub(crate) struct TalkEpoch {
    /// 起点（ms・端数つき）。cue が 1 つも届いていなければ `None`。
    epoch_ms: Option<f64>,
}

impl TalkEpoch {
    /// 届いた cue ごと（種類を問わない）に呼ぶ: `起点 = max(起点, 今 − cue の時刻)`。
    ///
    /// 新しいトークは届く時刻が前へ飛ぶので起点も前へ飛び、同じトークの中で早めに届いた cue
    /// （より小さい「今 − cue の時刻」）は最大に負けて起点を戻さない。
    pub(crate) fn observe(&mut self, at_s: f64, now_ms: u64) {
        let candidate = now_ms as f64 - at_s * 1000.0;
        self.epoch_ms = Some(
            self.epoch_ms
                .map_or(candidate, |epoch| epoch.max(candidate)),
        );
    }

    /// 今の台本の秒（起点より前は 0・起点が無ければ `None`）。
    pub(crate) fn talk_time(&self, now_ms: u64) -> Option<f64> {
        self.epoch_ms
            .map(|epoch| ((now_ms as f64 - epoch) / 1000.0).max(0.0))
    }

    /// 台本の秒を壁時刻（ms）に写す。最も近い 1 ms へ丸める（時計の分解能より細かい値を持てない
    /// ための写しで、刻みの境目への丸めではない。切り捨てないので、浮動小数の誤差で 1 ms 手前に
    /// ならない）。起点が無ければ `None`。
    pub(crate) fn wall_ms(&self, r_s: f64) -> Option<u64> {
        self.epoch_ms
            .map(|epoch| (epoch + r_s * 1000.0).round() as u64)
    }
}

/// スコープ 1 つの文字の時刻の列（序数は塊をまたいで連続）。
#[derive(Default)]
pub(crate) struct TalkFeed {
    /// `times` の先頭の文字の序数（刈り込んだ分だけ進む）。
    base: u64,
    /// 文字が現れる台本の秒（単調非減少）。
    times: VecDeque<f64>,
    /// 直前の文字の時刻（消去で `None` に戻る＝次の塊は塊の頭から）。刈り込みでは変わらない。
    last: Option<f64>,
}

impl TalkFeed {
    /// 文字の cue 1 件（台詞の文字・選択肢の文字）を、書記素クラスタの数だけ積む（要件 4.2）。
    pub(crate) fn push_text(&mut self, at_s: f64, duration_s: f64, text: &str) {
        self.push_chunk(at_s, duration_s, cluster_count(text));
    }

    /// 塊 1 つ（頭 `at_s`・再生時間 `duration_s`・`count` 文字）を積む。
    ///
    /// 1 字の間隔は `duration_s / count`、文字の時刻は「前の文字＋間隔」と塊の頭の大きい方
    /// （前の文字が無ければ塊の頭）。`count == 0` は何もしない（割らない）。`duration_s` は cue の
    /// 入口で有限・非負に揃えてある前提で、列は単調のまま。
    pub(crate) fn push_chunk(&mut self, at_s: f64, duration_s: f64, count: usize) {
        if count == 0 {
            return;
        }
        let interval = duration_s / count as f64;
        for _ in 0..count {
            let r = self.last.map_or(at_s, |prev| (prev + interval).max(at_s));
            self.times.push_back(r);
            self.last = Some(r);
        }
    }

    /// 消去の知らせ（台本の秒 `at_s`）: その時刻までに現れていない文字を捨て、次の塊を塊の頭から
    /// 始めさせる。
    ///
    /// 文字の層は消去で未表示の文字ごと列を空にするので、現れなかった文字で口は動かない。残る文字は
    /// `at_s` 以前、次の塊の頭は `at_s` 以後なので、列は単調のまま（新しい台詞の頭の全消去は
    /// 台本の 0 秒で届き、中断された前の台詞の残りをここで捨てる）。
    pub(crate) fn restart_chain(&mut self, at_s: f64) {
        self.times
            .truncate(self.times.partition_point(|&r| r <= at_s));
        self.last = None;
    }

    /// 台本の秒 `t_s` までに現れた文字の数（序数で数える・ちょうどの時刻の文字は現れている）。
    pub(crate) fn revealed_until(&self, t_s: f64) -> u64 {
        self.base + self.times.partition_point(|&r| r <= t_s) as u64
    }

    /// 序数 `g` の文字が現れる台本の秒（刈り込み済み・まだ無い序数は `None`）。
    pub(crate) fn time_of(&self, g: u64) -> Option<f64> {
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
