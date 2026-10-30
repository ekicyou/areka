//! calls: 働きのコマを予定の時刻の順に当てる純粋な係（spec: seriko-pattern-calls 要件 3.1–3.9・
//! 4.1–4.4・5.2・7.2・7.3・8.1）。
//!
//! 再生中の各 animation の、まだ当てていない働きのコマのうち予定の時刻（開始 ＋ 累積の待ち・
//! `always` は周ごと）が今以下のものを、時刻・animation・pattern の昇順の待ち行列で 1 つずつ当てる。
//! 再生の表は [`CallPlays`] の口だけで見る（一番上は面の段の再生、部品は部品の時計・5.2/5.3）。
//! 時刻は丸めない（遅れた刻みでも、呼ばれた側は呼び出しの予定の時刻で始まる）。

use crate::table::{FrameAct, LoopAnimation, LoopTrigger};
use crate::timeline::LoopRng;
use std::collections::{BTreeSet, HashSet};

/// 再生中の表を Calls から見る口（一番上は面の段の再生と欄、部品は部品の時計と欄）。
pub(crate) trait CallPlays {
    /// 再生中なら (開始の時刻, 働きのコマを当て終えた時刻)。終わって最後のコマを残したものは数えない。
    fn playing(&self, animation: u32) -> Option<(u64, Option<u64>)>;
    /// 働きのコマをこの時刻まで当て終えた印を付ける。
    fn set_acts_through(&mut self, animation: u32, through_ms: u64);
    /// 開始の時刻 `at_ms` で頭から始める（今の再生は置き換え、当て終えた印は無しに戻す）。
    fn start(&mut self, animation: u32, at_ms: u64);
    /// 再生と欄を外す（`-1` と同じ）。
    /// 止めた `always` の印を付けるのは実装する側の役目（タスク 5.2）。
    fn stop(&mut self, animation: u32);
    /// 再生中の animation の番号。
    fn playing_ids(&self) -> Vec<u32>;
}

/// 1 回の刻みで当てた働きのコマの結果。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CallOutcome {
    /// この刻みで最後に当たった move（時刻・x・y）。無ければ `None`。
    pub shift: Option<(u64, i64, i64)>,
}

/// 待ち行列の 1 件: (予定の時刻, animation, pattern の位置, その再生の開始の時刻)。
/// 組の比べ方がそのまま当てる順（時刻・animation・pattern の昇順）になる。
type Due = (u64, u32, usize, u64);

/// 前の刻みから `now_ms` までに来た働きのコマを予定の時刻の順に当てる。
///
/// 事前条件: `plays` の各再生の開始の時刻は `now_ms` 以下。事後条件: 予定の時刻が `now_ms` 以下の
/// 働きのコマはすべて当て終え、再生中の全部の当て終えた印が `now_ms` になる。選びは `pick` だけを引く。
pub(crate) fn run_calls<'a>(
    plays: &mut impl CallPlays,
    animation_of: impl Fn(u32) -> Option<&'a LoopAnimation>,
    now_ms: u64,
    pick: &mut LoopRng,
) -> CallOutcome {
    let mut queue = BTreeSet::new();
    for id in plays.playing_ids() {
        if let (Some((start, through)), Some(anim)) = (plays.playing(id), animation_of(id)) {
            push_due(&mut queue, anim, start, through, now_ms);
        }
    }
    let mut outcome = CallOutcome::default();
    // 1 つの時刻の中で呼び出しが始めた animation（巡回の止め・5.2）。
    let mut instant = None;
    let mut started: HashSet<u32> = HashSet::new();
    while let Some((at, id, k, start)) = queue.pop_first() {
        // 止められた・やり直された再生の出来事は捨てる。
        if plays.playing(id).map(|(s, _)| s) != Some(start) {
            continue;
        }
        let Some(frame) = animation_of(id).and_then(|a| a.frames.get(k)) else {
            continue;
        };
        if instant != Some(at) {
            instant = Some(at);
            started.clear();
        }
        match &frame.act {
            FrameAct::Start { targets, pick_one } => {
                for target in chosen(targets, *pick_one, pick) {
                    // 表に無い相手は何もしない（記録は読み込みのときに 1 度出し済み・5.1）。
                    let Some(anim) = animation_of(target) else {
                        continue;
                    };
                    // 抽選・always・きっかけでこの時刻に始まった再生も 2 度目に数える。
                    let already = plays.playing(target).map(|(s, _)| s) == Some(at);
                    if already || !started.insert(target) {
                        tracing::debug!(
                            animation = target,
                            at_ms = at,
                            "seriko calls: 同じ時刻に 2 度目の開始ゆえ何もしない（要件 5.2）"
                        );
                        continue;
                    }
                    plays.start(target, at);
                    push_due(&mut queue, anim, at, None, now_ms);
                }
            }
            FrameAct::Stop { targets, pick_one } => {
                for target in chosen(targets, *pick_one, pick) {
                    // 再生していない相手は何もしない（4.4・M2）。
                    if plays.playing(target).is_some() {
                        plays.stop(target);
                    }
                }
            }
            // ずれは元の表示位置からの絶対値で、後の時刻が勝つ（7.2・7.3）。
            FrameAct::Move => outcome.shift = Some((at, frame.x, frame.y)),
            FrameAct::Draw | FrameAct::Inert => {}
        }
    }
    for id in plays.playing_ids() {
        plays.set_acts_through(id, now_ms);
    }
    outcome
}

/// 相手の並びから当てる番号（`pick_one` なら並びの全部から等しく 1 つ・3.3・4.3）。
fn chosen(targets: &[u32], pick_one: bool, pick: &mut LoopRng) -> Vec<u32> {
    if !pick_one {
        return targets.to_vec();
    }
    let len = u32::try_from(targets.len()).unwrap_or(u32::MAX);
    targets
        .get(pick(len) as usize)
        .copied()
        .into_iter()
        .collect()
}

/// 1 つの再生の、予定の時刻が (`through`, `now`] に入る働きのコマを待ち行列へ入れる
/// （`through` が無ければ開始の時刻を含む）。`always` は周ごとに繰り返し、ほかは描くコマの負の番号
/// （`-1` で止まる）の後ろを入れない。
fn push_due(
    queue: &mut BTreeSet<Due>,
    anim: &LoopAnimation,
    start: u64,
    through: Option<u64>,
    now: u64,
) {
    let due = |at: u64| at <= now && through.is_none_or(|t| at > t);
    // 働きのコマ（何もしない Inert を除く）の位置と、開始からの累積の待ち。
    let mut acts = Vec::new();
    let mut cum: u64 = 0;
    let repeats = matches!(anim.trigger, LoopTrigger::Always { .. });
    for (k, f) in anim.frames.iter().enumerate() {
        cum = cum.saturating_add(u64::from(f.wait_ms));
        match f.act {
            FrameAct::Draw if f.surface_id < 0 && !repeats => break,
            FrameAct::Draw | FrameAct::Inert => {}
            _ => acts.push((k, cum)),
        }
    }
    let LoopTrigger::Always { period_ms, laps } = anim.trigger else {
        for (k, cum) in acts {
            let at = start.saturating_add(cum);
            if due(at) {
                queue.insert((at, anim.id, k, start));
            }
        }
        return;
    };
    let period = period_ms.get();
    let lo = through.unwrap_or(start).saturating_sub(start);
    // 累積の待ちは周の長さ以下なので、1 つ前の周の終わりのコマも窓に入りうる。
    let first = (lo / period).saturating_sub(1);
    let mut last = now.saturating_sub(start) / period;
    if let Some(n) = laps {
        last = last.min(u64::from(n.get()) - 1);
    }
    for lap in first..=last {
        for &(k, cum) in &acts {
            let at = start
                .saturating_add(lap.saturating_mul(period))
                .saturating_add(cum);
            if due(at) {
                queue.insert((at, anim.id, k, start));
            }
        }
    }
}

#[cfg(test)]
#[path = "calls_tests.rs"]
mod tests;
