//! trigger: 引き金の判定（spec: areka-P0-seriko-trigger-intervals 要件 2・3・4.1・4.5〜4.7・5.3・5.9・
//! 6.1・7.5）。
//!
//! `runonce`・`periodic,数値`・`talk,数値` の animation を「今の判定で始めるか・始めるなら何時に
//! 始まったことにするか」だけを決める。始まった後の進み方（コマの順・`-1` の停止・末尾の保持）は
//! 抽選の animation と同じ経路が受け持つので、ここは持たない。
//!
//! 状態は [`Armed`] 1 つ。面（一番上の面、または見えている部品 1 つ）が見え始めた時刻を起点に持ち、
//! 一番上でも部品でも同じ型・同じ関数を使う（要件 5.4）。乱数は読まない。始めない経路（隠れている・
//! 再生中・周の境目の前）は正常なので記録を出さない（要件 7.5）。
//!
//! - `runonce`: 見えている間に 1 回だけ。開始の時刻は見え始めの時刻（要件 2.1・2.2）。
//! - `periodic`: 見え始めからの経過を周期で割り（[`lap_of`]・丸めない）、周が進んだ判定で鳴る。
//!   開始の時刻は最新の周の境目そのもの（判定の時刻ではない・要件 3.1・3.4・3.5・6.1）。
//! - 隠す・現す（バルーンの窓の閉じ・開き）: 隠れている間は鳴らず、現れた時刻が `periodic` の新しい
//!   起点になる。`runonce` の印は残る（開き直しでは鳴らない・要件 3.2・5.9）。`talk` の数えは隠すと
//!   捨て、現すときに渡された数から数え直す。
//! - `talk`: 数え始めから数値分の文字ごとの区切りを、刻み 1 回分の文字の窓（[`TalkWindow`]）が
//!   越えた判定で鳴る。開始の時刻は区切りの文字が現れた時刻そのもの（判定の時刻ではない）。
//!   1 回の窓で区切りを 2 つ以上越えても最新の 1 つだけ（要件 4.1・4.5・4.6）。数えは判定の後に
//!   [`Armed::advance_talk`] で進め、再生中に越えた区切りは後から鳴らない（要件 4.7）。

use std::collections::{BTreeMap, BTreeSet};

use crate::table::{LoopAnimation, LoopTrigger};
use crate::timeline::lap_of;

/// 面（一番上の面、または見えている部品 1 つ）が見え始めてからの引き金の状態。
pub(crate) struct Armed {
    /// 見えている間の起点（ms）。隠れている（バルーンの窓が閉じている）間は `None`。
    visible_since: Option<u64>,
    /// `runonce` を鳴らした animation の番号（この状態が在る間は 2 度と鳴らさない）。
    fired: BTreeSet<u32>,
    /// `periodic` の animation ごとに、最後に判定した周（載っていなければ 0＝まだ）。
    last_lap: BTreeMap<u32, u64>,
    /// `talk` の数え（一番上の面の状態だけが持つ・部品は面の窓を借りるので `None`）。
    talk: Option<TalkCursor>,
}

/// スコープの文字の序数の数え。
pub(crate) struct TalkCursor {
    /// 構えた時点で現れていた文字の序数（ここから 0 と数える）。
    base: u64,
    /// 最後の判定までに数えた序数（単調非減少）。
    seen: u64,
}

/// 1 回の判定で `talk` に渡す文字の窓（面ごとに刻み 1 回作る・部品も同じ窓を使う）。
pub(crate) struct TalkWindow<'a> {
    /// 数え始めの序数（ここから 0 と数える）。
    pub(crate) base: u64,
    /// 前の判定までに数えた序数。
    pub(crate) prev_seen: u64,
    /// 今見えている文字の序数。
    pub(crate) now_seen: u64,
    /// 序数 → 壁時刻（ms・最も近い 1 ms へ丸める）。
    pub(crate) wall_ms: &'a dyn Fn(u64) -> u64,
}

impl Armed {
    /// 見え始めの時刻で構える（`open` が偽なら隠れた状態で構える）。
    ///
    /// `revealed` は今現れている文字の序数で、`talk` はここから 0 と数える（`None` なら数えを
    /// 持たない＝表に `talk` が無い面・窓が閉じている面・部品）。
    pub(crate) fn arm(at_ms: Option<u64>, open: bool, revealed: Option<u64>) -> Armed {
        Armed {
            visible_since: at_ms.filter(|_| open),
            fired: BTreeSet::new(),
            last_lap: BTreeMap::new(),
            talk: revealed.map(TalkCursor::starting_at),
        }
    }

    /// 窓が閉じた: 起点と `talk` の数えを消す（`runonce` の印は残す。`periodic` の周は現すときに
    /// 置き直す）。隠れている間は文字を数えないので、数え終えた文字の刈り込みを止めない。
    pub(crate) fn hide(&mut self) {
        self.visible_since = None;
        self.talk = None;
    }

    /// 窓が開いた: 起点を置き直す（`periodic` の周は 0 から・`talk` は `revealed` を 0 と数え直す）。
    ///
    /// `revealed` は [`Armed::arm`] と同じ。
    pub(crate) fn show(&mut self, at_ms: u64, revealed: Option<u64>) {
        self.visible_since = Some(at_ms);
        self.last_lap.clear();
        self.talk = revealed.map(TalkCursor::starting_at);
    }

    /// 見えているか（隠れている＝バルーンの窓が閉じている間は偽）。窓の知らせを写す側が、隠す・現すを
    /// 1 回ずつ呼ぶために読む。
    pub(crate) fn is_visible(&self) -> bool {
        self.visible_since.is_some()
    }

    /// 今の判定で `anim` を始めるなら、開始の時刻を返す。
    ///
    /// 隠れている間と、同じ animation が再生中の間は始めない。「再生中か」は判定の時刻 `now_ms` でなく、
    /// 返すことになる開始の時刻で `playing_at` に尋ねる（`runonce` は見え始め・`periodic` は周の境目・
    /// `talk` は区切りの文字が現れた時刻）。その時刻ちょうどに終わった再生は再生中でないので、長さが
    /// 周期ちょうどの animation も毎周鳴り、遅れた刻みが見た境目も、その時刻に再生中だったなら飛ばす
    /// （要件 3.1・3.3・3.5・4.7）。`periodic` は再生中に来た周も数えだけ進めるので、飛ばした周が後から
    /// 鳴ることは無い。`talk` は `talk`（文字の窓）が無ければ始めず、数えはここでは進めない（窓 1 つを
    /// 一番上と部品の何本もの animation が読むので、進めるのは全部の判定の後の
    /// [`Armed::advance_talk`]）。抽選の引き金と `always` はここへ来ない前提で、来ても何も返さない。
    pub(crate) fn poll(
        &mut self,
        anim: &LoopAnimation,
        now_ms: u64,
        playing_at: impl Fn(u64) -> bool,
        talk: Option<&TalkWindow<'_>>,
    ) -> Option<u64> {
        let since = self.visible_since?;
        match anim.trigger {
            LoopTrigger::Runonce => {
                (!playing_at(since) && self.fired.insert(anim.id)).then_some(since)
            }
            LoopTrigger::Periodic { period_ms } => {
                let (lap, _) = lap_of(now_ms.saturating_sub(since), period_ms);
                if lap <= self.last_lap.get(&anim.id).copied().unwrap_or(0) {
                    return None;
                }
                self.last_lap.insert(anim.id, lap);
                // 周の数は経過を周期で割った商なので、掛け戻しは今の時刻を超えない。
                Some(since + lap * period_ms.get()).filter(|&at| !playing_at(at))
            }
            LoopTrigger::Talk { every } => {
                let window = talk?;
                let every = u64::from(every.get());
                // 今見えている数が数え始めより手前（消去で切り詰められた刻み）なら、越えた区切りは 0。
                let crossed = window.now_seen.saturating_sub(window.base) / every;
                if crossed == 0 {
                    return None;
                }
                // 越えた区切りのうち最新の文字の序数（今見えている数を超えない）。前の判定までに
                // 数え済みなら鳴らし済みか見送り済み。今見えている数が減った刻みも、ここで外れる。
                let glyph = window.base + crossed * every - 1;
                (glyph >= window.prev_seen)
                    .then(|| (window.wall_ms)(glyph))
                    .filter(|&at| !playing_at(at))
            }
            LoopTrigger::Random { .. }
            | LoopTrigger::BindRandom { .. }
            | LoopTrigger::Always { .. } => None,
        }
    }

    /// `talk` の判定の後に、数えた序数を進める（面の窓 1 つにつき刻み 1 回）。
    ///
    /// 数えは単調: 今見えている数が減った刻み（消去で切り詰められた・起点が前へ飛んだ）は進めない。
    /// 数えを持たない状態（部品・表に `talk` が無い面）では何もしない。
    pub(crate) fn advance_talk(&mut self, now_seen: u64) {
        if let Some(cursor) = &mut self.talk {
            cursor.seen = cursor.seen.max(now_seen);
        }
    }

    /// 消去の知らせの後に呼ぶ: 文字の列が `total` 文字まで切り詰められた（捨てられた序数は次に届く
    /// 文字に振り直される）。数え済みの序数が `total` を超えていたら、数えた文字の数（数え済み −
    /// 数え始め）を保ったまま数え済みを `total` に揃える。揃えないと、次に届く文字が「数え済み」に
    /// 埋もれて区切りを越えても鳴らない。超えていなければ何もしない。
    pub(crate) fn realign_talk(&mut self, total: u64) {
        if let Some(cursor) = &mut self.talk {
            let dropped = cursor.seen.saturating_sub(total);
            cursor.seen -= dropped;
            // ponytail: 数え始めの序数が捨てられた数より小さいと 0 で止まり、数えた数が減る（次の区切りが
            // 遅れる）。数え済みが列を超えるのは刻みの間に構えた直後（数えた数 0）なので、まず届かない。
            // 届くようなら数え始めを符号つきにする。
            cursor.base = cursor.base.saturating_sub(dropped);
        }
    }

    /// 文字の窓を組むための（数え始めの序数, 数え済みの序数）。数えを持たなければ `None`。
    pub(crate) fn talk_window_bounds(&self) -> Option<(u64, u64)> {
        self.talk.as_ref().map(|cursor| (cursor.base, cursor.seen))
    }
}

impl TalkCursor {
    /// `revealed` 文字が現れている時点から数え始める。
    fn starting_at(revealed: u64) -> TalkCursor {
        TalkCursor {
            base: revealed,
            seen: revealed,
        }
    }
}

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod tests;
