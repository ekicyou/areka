//! 一番上の面の働きのコマの配線（spec: seriko-pattern-calls 要件 1.3・3.1・3.4・3.5・3.8・3.9・4.1・
//! 4.5・7.7）。
//!
//! 面の段の再生の表と作りかけの欄を一緒に [`CallPlays`] として貸し、[`run_calls`] で当てる。呼ぶのは
//! 抽選・`always`・引き金の始まりの後、コマを置く前（同じ時刻の 2 度目の開始の止めはこの順が前提）。
//! 欄に置くのは描くコマだけ（働きのコマは絵を重ねず、前の絵を残す）。

use std::collections::{HashMap, HashSet};

use areka_emo_compose::PatternState;
use areka_sakura::ActorKey;

use super::{Playback, SlotPlayback, pattern_frame};
use crate::calls::{CallOutcome, CallPlays, run_calls};
use crate::state::Slot;
use crate::table::{LoopAnimation, LoopTrigger};
use crate::timeline::{AlwaysView, LoopRng, always_at, visible_frame};

/// 止めた `always` の印（スコープ, 面の段, animation）。印がある間は始め直さず、面を出したときに消す
/// （SSP の測り M2・2026-10-30 設計議題 1）。
pub(super) type StoppedAlways = HashSet<(ActorKey, Slot, u32)>;

/// 一番上の面の段の再生の表と欄を Calls へ貸す口。
struct TopPlays<'a> {
    plays: &'a mut SlotPlayback,
    pattern: &'a mut PatternState,
    stopped: &'a mut StoppedAlways,
    key: &'a (ActorKey, Slot),
    anims: &'a [LoopAnimation],
    started: Vec<u32>,
}

impl CallPlays for TopPlays<'_> {
    // 再生の表に在るものが再生中（終わって最後のコマを残したものは進行が表から外している）。
    fn playing(&self, animation: u32) -> Option<(u64, Option<u64>)> {
        self.plays
            .get(&animation)
            .map(|p| (p.started_at_ms, p.acts_through))
    }

    fn set_acts_through(&mut self, animation: u32, through_ms: u64) {
        if let Some(p) = self.plays.get_mut(&animation) {
            // 合図の時刻が前の刻みより前でも印を戻さない（同じコマを 2 度当てない）。
            p.acts_through = Some(p.acts_through.map_or(through_ms, |t| t.max(through_ms)));
        }
    }

    fn start(&mut self, animation: u32, at_ms: u64) {
        self.plays.insert(
            animation,
            Playback {
                started_at_ms: at_ms,
                acts_through: None,
            },
        );
        self.stopped
            .remove(&(self.key.0.clone(), self.key.1, animation));
        self.started.push(animation);
        tracing::debug!(
            scope = self.key.0.as_str(),
            slot = ?self.key.1,
            animation_id = animation,
            at_ms,
            "seriko: 呼び出しで再生を始めた（頭から・要件 3.1）"
        );
    }

    fn stop(&mut self, animation: u32) {
        self.plays.remove(&animation);
        self.pattern.remove(animation);
        let always = self
            .anims
            .iter()
            .any(|a| a.id == animation && matches!(a.trigger, LoopTrigger::Always { .. }));
        if always {
            self.stopped
                .insert((self.key.0.clone(), self.key.1, animation));
        }
        tracing::debug!(
            scope = self.key.0.as_str(),
            slot = ?self.key.1,
            animation_id = animation,
            always,
            "seriko: 呼び出しで再生を止めた（-1 と同じ・要件 4.1）"
        );
    }

    fn playing_ids(&self) -> Vec<u32> {
        self.plays.keys().copied().collect()
    }
}

/// 一番上の面の段 `key` の働きのコマを `now_ms` まで当てる。結果と、呼び出しで始まった animation の
/// 番号を返す（合図の更新はそのコマを置く）。再生が 1 つも無ければ何もしない。
pub(super) fn run_top_calls(
    playback: &mut HashMap<(ActorKey, Slot), SlotPlayback>,
    pattern: &mut PatternState,
    stopped: &mut StoppedAlways,
    key: &(ActorKey, Slot),
    anims: &[LoopAnimation],
    now_ms: u64,
    pick: &mut LoopRng,
) -> (CallOutcome, Vec<u32>) {
    let Some(plays) = playback.get_mut(key) else {
        return (CallOutcome::default(), Vec::new());
    };
    let mut top = TopPlays {
        plays,
        pattern,
        stopped,
        key,
        anims,
        started: Vec::new(),
    };
    let outcome = run_calls(
        &mut top,
        |id| anims.iter().find(|a| a.id == id),
        now_ms,
        pick,
    );
    (outcome, top.started)
}

/// 今のコマ `i` の代わりに欄へ置く描くコマ（`i` 以下で最後の描くコマ・無ければ欄を外す）。
pub(super) fn put_visible(pattern: &mut PatternState, anim: &LoopAnimation, i: usize) {
    match visible_frame(&anim.frames, i) {
        Some(d) => pattern.set(anim.id, pattern_frame(&anim.frames[d])),
        None => pattern.remove(anim.id),
    }
}

/// 一番上の `always` の経過 `elapsed_ms` の答えを欄へ置く: 経過 0 の答えと同じなら載せない／
/// コマならコマ／何も出さないなら「消えている」（spec: areka-P0-animated-image-playback 要件 4.1・
/// 4.2・4.5・7.4）。再生は末尾でも負の番号でも捨てない（周の頭へ戻って続く）。
///
/// 今のコマが働きのコマなら、その周の最後の描くコマを置く。周の頭から働きのコマなら前の周の最後の
/// 描くコマを残し、1 周目なら欄を外す（spec: seriko-pattern-calls 1.3・7.7）。
pub(super) fn put_top_always(pattern: &mut PatternState, anim: &LoopAnimation, elapsed_ms: u64) {
    let LoopTrigger::Always { period_ms, laps } = anim.trigger else {
        return;
    };
    let frames = &anim.frames;
    // `None` は「欄を外す」（描くコマがまだ無い）。
    let view = |elapsed: u64| match always_at(frames, period_ms, laps, elapsed) {
        AlwaysView::Frame(i) => {
            let previous_lap = (elapsed >= period_ms.get()).then(|| frames.len() - 1);
            let d = visible_frame(frames, i)
                .or_else(|| previous_lap.and_then(|l| visible_frame(frames, l)))?;
            Some(
                if frames[d].picture.is_some() || frames[d].surface_id >= 0 {
                    AlwaysView::Frame(d)
                } else {
                    AlwaysView::Nothing
                },
            )
        }
        AlwaysView::Nothing => Some(AlwaysView::Nothing),
    };
    match view(elapsed_ms) {
        now if now == view(0) => pattern.remove(anim.id),
        Some(AlwaysView::Frame(i)) => pattern.set(anim.id, pattern_frame(&frames[i])),
        Some(AlwaysView::Nothing) => pattern.set_blank(anim.id),
        None => pattern.remove(anim.id),
    }
}
