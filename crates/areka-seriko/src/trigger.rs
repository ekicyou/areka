//! trigger: 引き金の判定（spec: areka-P0-seriko-trigger-intervals 要件 2・3・5.3・5.9・6.1・7.5）。
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
//!   起点になる。`runonce` の印は残る（開き直しでは鳴らない・要件 3.2・5.9）。
//! - `talk`: 文字の区切りの判定はまだ無い（[`Armed::poll`] は何も返さない）。

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
}

/// 1 回の判定で `talk` に渡す文字の窓（面ごとに刻み 1 回作る・部品も同じ窓を使う）。
#[expect(
    dead_code,
    reason = "組むのも欄を読むのも文字の区切りの判定（タスク 3.2）を入れるとき"
)]
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
    /// `_revealed` は `talk` の数え始めの序数（文字の区切りの判定が読む・今は読まない）。
    pub(crate) fn arm(at_ms: Option<u64>, open: bool, _revealed: Option<u64>) -> Armed {
        Armed {
            visible_since: at_ms.filter(|_| open),
            fired: BTreeSet::new(),
            last_lap: BTreeMap::new(),
        }
    }

    /// 窓が閉じた: 起点を消す（`runonce` の印は残す）。
    pub(crate) fn hide(&mut self) {
        self.visible_since = None;
    }

    /// 窓が開いた: 起点を置き直す（`periodic` の周は 0 から）。
    ///
    /// `_revealed` は [`Armed::arm`] と同じ。
    pub(crate) fn show(&mut self, at_ms: u64, _revealed: Option<u64>) {
        self.visible_since = Some(at_ms);
        self.last_lap.clear();
    }

    /// 今の判定で `anim` を始めるなら、開始の時刻を返す。
    ///
    /// 隠れている間・`playing`（同じ animation が再生中）の間は始めない。`periodic` は再生中に来た周も
    /// 数えだけ進めるので、飛ばした周が後から鳴ることは無い（要件 3.3）。抽選の引き金と `always` は
    /// ここへ来ない前提で、来ても何も返さない。
    pub(crate) fn poll(
        &mut self,
        anim: &LoopAnimation,
        now_ms: u64,
        playing: bool,
        _talk: Option<&TalkWindow<'_>>,
    ) -> Option<u64> {
        let since = self.visible_since?;
        match anim.trigger {
            LoopTrigger::Runonce => (!playing && self.fired.insert(anim.id)).then_some(since),
            LoopTrigger::Periodic { period_ms } => {
                let (lap, _) = lap_of(now_ms.saturating_sub(since), period_ms);
                if lap <= self.last_lap.get(&anim.id).copied().unwrap_or(0) {
                    return None;
                }
                self.last_lap.insert(anim.id, lap);
                // 周の数は経過を周期で割った商なので、掛け戻しは今の時刻を超えない。
                (!playing).then(|| since + lap * period_ms.get())
            }
            LoopTrigger::Talk { .. }
            | LoopTrigger::Random { .. }
            | LoopTrigger::BindRandom { .. }
            | LoopTrigger::Always { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod tests;
