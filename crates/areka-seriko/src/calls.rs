//! calls: 働きのコマを予定の時刻の順に当てる純粋な係。
//!
//! 再生中の各 animation の、まだ当てていない働きのコマのうち予定の時刻（開始 ＋ 累積の待ち・
//! `always` は周ごと）が今以下のものを、時刻・animation・pattern の昇順の待ち行列で 1 つずつ当てる。
//! 再生の表は [`CallPlays`] の口だけで見る（一番上は面の段の再生、部品は部品の時計）。
//! 時刻は丸めない（遅れた刻みでも、呼ばれた側は呼び出しの予定の時刻で始まる）。

use crate::table::{FrameAct, LoopAnimation, LoopTrigger};
use crate::timeline::LoopRng;
use dola::{TimePoint, TimeSpan};
use std::collections::{BTreeSet, HashSet};

/// 再生中の表を Calls から見る口（一番上は面の段の再生と欄、部品は部品の時計と欄）。
pub(crate) trait CallPlays {
    /// 再生中なら (開始の時刻, 働きのコマを当て終えた時刻)。終わって最後のコマを残したものは数えない。
    fn playing(&self, animation: u32) -> Option<(TimePoint, Option<TimePoint>)>;
    /// 当て終えた印をこの値にする（戻さない決まりは [`run_calls`] が受け持つ）。
    fn set_acts_through(&mut self, animation: u32, through: TimePoint);
    /// 開始の時刻 `at` で頭から始める（今の再生は置き換え、当て終えた印は無しに戻す）。
    fn start(&mut self, animation: u32, at: TimePoint);
    /// 再生と欄を外す（`-1` と同じ）。`always` が真なら止めた `always` の印を付ける。
    fn stop(&mut self, animation: u32, always: bool);
    /// 再生中の animation の番号。
    fn playing_ids(&self) -> Vec<u32>;
}

/// 1 回の刻みで当てた働きのコマの結果。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CallOutcome {
    /// この刻みで最後に当たった move（時刻・x・y）。無ければ `None`。
    pub shift: Option<(TimePoint, i64, i64)>,
}

impl CallOutcome {
    /// 後に当てた側 `next` と合わせる: ずれは時刻が後のものが勝つ（同じ時刻なら `next`）。
    pub(crate) fn then(self, next: CallOutcome) -> CallOutcome {
        match (self.shift, next.shift) {
            (Some((a, ..)), Some((b, ..))) if a > b => self,
            (_, None) => self,
            _ => next,
        }
    }
}

/// 待ち行列の 1 件: (予定の時刻, animation, pattern の位置, その再生の開始の時刻)。
/// 組の比べ方がそのまま当てる順（時刻・animation・pattern の昇順）になる。
type Due = (TimePoint, u32, usize, TimePoint);

/// 前の刻みから `now` までに来た働きのコマを予定の時刻の順に当てる。
///
/// `now` より後に始まった再生（合図の時刻で始めた再生は刻みより僅かに後になりうる）は、何も当てず
/// 次の刻みを待つ。事後条件: 予定の時刻が `now` 以下の
/// 働きのコマはすべて当て終え、再生中の全部の当て終えた印が `max(前の印, now)` になる（合図の時刻が
/// 前の刻みより前でも印を戻さない＝同じコマを 2 度当てない）。選びは `pick` だけを引く。
pub(crate) fn run_calls<'a>(
    plays: &mut impl CallPlays,
    animation_of: impl Fn(u32) -> Option<&'a LoopAnimation>,
    now: TimePoint,
    pick: &mut LoopRng,
) -> CallOutcome {
    // 再生ごとに次の 1 件だけを入れ、取り出したらその次を入れる（入れる数は当てる数と同じ）。
    let mut queue = BTreeSet::new();
    for id in plays.playing_ids() {
        push_next(&mut queue, &*plays, &animation_of, id, None, now);
    }
    let mut outcome = CallOutcome::default();
    // 1 つの時刻の中で呼び出しが始めた animation（巡回の止め）。
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
                    // 表に無い相手は何もしない（記録は読み込みのときに 1 度出し済み）。
                    if animation_of(target).is_none() {
                        continue;
                    }
                    // 抽選・always・きっかけでこの時刻に始まった再生も 2 度目に数える。
                    let already = plays.playing(target).map(|(s, _)| s) == Some(at);
                    if already || !started.insert(target) {
                        tracing::debug!(
                            animation = target,
                            %at,
                            "seriko calls: 同じ時刻に 2 度目の開始ゆえ何もしない（要件 5.2）"
                        );
                        continue;
                    }
                    plays.start(target, at);
                    push_next(&mut queue, &*plays, &animation_of, target, None, now);
                }
            }
            FrameAct::Stop { targets, pick_one } => {
                for target in chosen(targets, *pick_one, pick) {
                    // 再生していない相手は何もしない。
                    if plays.playing(target).is_some() {
                        let always = animation_of(target)
                            .is_some_and(|a| matches!(a.trigger, LoopTrigger::Always { .. }));
                        plays.stop(target, always);
                    }
                }
            }
            // ずれは元の表示位置からの絶対値で、後の時刻が勝つ。
            FrameAct::Move => outcome.shift = Some((at, frame.x, frame.y)),
            FrameAct::Draw | FrameAct::Inert => {}
        }
        // 止めた・やり直した再生の続きは入れない（やり直しは自分の頭を入れ済み）。
        if plays.playing(id).map(|(s, _)| s) == Some(start) {
            push_next(&mut queue, &*plays, &animation_of, id, Some((at, k)), now);
        }
    }
    for id in plays.playing_ids() {
        let through = plays
            .playing(id)
            .and_then(|(_, t)| t)
            .map_or(now, |t| t.max(now));
        plays.set_acts_through(id, through);
    }
    outcome
}

/// 相手の並びから当てる番号（`pick_one` なら並びの全部から等しく 1 つ）。
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

/// 再生中の `id` の、(時刻, 位置) が `after` より後で一番先の当て残しを待ち行列へ入れる。
fn push_next<'a>(
    queue: &mut BTreeSet<Due>,
    plays: &impl CallPlays,
    animation_of: &impl Fn(u32) -> Option<&'a LoopAnimation>,
    id: u32,
    after: Option<(TimePoint, usize)>,
    now: TimePoint,
) {
    if let (Some((start, through)), Some(anim)) = (plays.playing(id), animation_of(id))
        && let Some((at, k)) = next_due(anim, start, through, after, now)
    {
        queue.insert((at, id, k, start));
    }
}

/// 1 つの再生の、予定の時刻が (`through`, `now`] に入る働きのコマのうち、(時刻, 位置) が `after` より後で
/// 一番先のもの（`through` が無ければ開始の時刻を含む）。`always` は周ごとに繰り返し、ほかは描くコマの
/// 負の番号（`-1` で止まる）の後ろを数えない。周は数え上げず、コマごとに条件を満たす最初の周を割り算で
/// 出す（長く空いても仕事はコマの数だけ・時刻は丸めない）。
fn next_due(
    anim: &LoopAnimation,
    start: TimePoint,
    through: Option<TimePoint>,
    after: Option<(TimePoint, usize)>,
    now: TimePoint,
) -> Option<(TimePoint, usize)> {
    let (period, last_lap, repeats) = match anim.trigger {
        LoopTrigger::Always { period, laps } => (
            period,
            laps.map_or(u64::MAX, |n| u64::from(n.get()) - 1),
            true,
        ),
        _ => (TimeSpan::ZERO, 0, false),
    };
    // 繰り返さない再生は周 0 だけを取る（1 で割れば、開始より後の時刻は周 1 になって外れる）。
    let divisor = period.as_ticks().max(1) as u64;
    let mut best: Option<(TimePoint, usize)> = None;
    let mut cum = TimeSpan::ZERO;
    for (k, f) in anim.frames.iter().enumerate() {
        cum = cum + f.wait;
        match f.act {
            FrameAct::Draw if f.surface_id < 0 && !repeats => break,
            FrameAct::Draw | FrameAct::Inert => continue,
            _ => {}
        }
        // このコマが取れる一番早い時刻（印より後・`after` より後）。
        let mut min_at = through.map_or(start, just_after);
        if let Some((a, k0)) = after {
            min_at = min_at.max(if k > k0 { a } else { just_after(a) });
        }
        let first = start + cum;
        let lap = (min_at - first)
            .to_span()
            .map_or(0, |d| (d.as_ticks() as u64).div_ceil(divisor));
        if lap > last_lap {
            continue;
        }
        let at = first + period.mul_div(lap, 1);
        let due = at <= now && through.is_none_or(|t| at > t) && after.is_none_or(|a| (at, k) > a);
        if due && best.is_none_or(|b| (at, k) < b) {
            best = Some((at, k));
        }
    }
    best
}

/// `t` の次の時刻（100ns 後）。
fn just_after(t: TimePoint) -> TimePoint {
    TimePoint::from_ticks(t.as_ticks().saturating_add(1))
}

#[cfg(test)]
#[path = "calls_tests.rs"]
mod tests;
