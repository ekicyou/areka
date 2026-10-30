//! parts の子: 部品の働きのコマの配線（spec: seriko-pattern-calls 要件 3.8・4.5・11.2）。
//!
//! 部品 1 つの時計（入れ物の clocks）と部品の欄を一緒に [`CallPlays`] として貸し、[`run_calls`] で
//! 当てる（相手は同じ部品の面の animation）。呼ぶのは抽選・`always`・引き金の始まりの後、コマを置く前
//! （一番上と同じ・同じ時刻の 2 度目の開始の止めはこの順が前提）。`Residual`（終わって最後のコマを
//! 残したもの）は再生中に数えない。呼ばれるだけ（`never`）の時計は、抽選で始まった時計と同じく、
//! 部品が見えない間も捨てない（見えなかった間も進んでいた扱い・parts の `drop_transient_where`）。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use areka_emo_compose::{PartKey, PatternState};
use areka_sakura::ActorKey;

use super::{ClockKey, Label, PartAnim, PartClocks, progress, unwrite, write_always};
use crate::calls::{CallOutcome, CallPlays, run_calls};
use crate::state::Slot;
use crate::table::{FrameAct, LoopAnimation, LoopTrigger};
use crate::timeline::LoopRng;

/// 部品 1 つの時計と欄を Calls へ貸す口。
struct PartPlays<'a> {
    clocks: &'a mut BTreeMap<ClockKey, PartAnim>,
    /// 止めた `always`（面を出すまで始め直さない・4.5）。
    stopped: &'a mut BTreeSet<ClockKey>,
    pattern: &'a mut PatternState,
    part: PartKey,
    anims: &'a [LoopAnimation],
    scope: &'a ActorKey,
    slot: Slot,
    started: Vec<u32>,
}

impl CallPlays for PartPlays<'_> {
    fn playing(&self, animation: u32) -> Option<(u64, Option<u64>)> {
        match self.clocks.get(&(self.part, animation)) {
            Some(&PartAnim::Playing {
                started_at_ms,
                acts_through,
            }) => Some((started_at_ms, acts_through)),
            _ => None,
        }
    }

    fn set_acts_through(&mut self, animation: u32, through_ms: u64) {
        if let Some(PartAnim::Playing { acts_through, .. }) =
            self.clocks.get_mut(&(self.part, animation))
        {
            // 合図の時刻が前の刻みより前でも印を戻さない（同じコマを 2 度当てない）。
            *acts_through = Some(acts_through.map_or(through_ms, |t| t.max(through_ms)));
        }
    }

    fn start(&mut self, animation: u32, at_ms: u64) {
        let key = (self.part, animation);
        self.clocks.insert(
            key,
            PartAnim::Playing {
                started_at_ms: at_ms,
                acts_through: None,
            },
        );
        self.stopped.remove(&key);
        self.started.push(animation);
        tracing::debug!(
            scope = self.scope.as_str(),
            slot = ?self.slot,
            part = %Label(self.part),
            animation_id = animation,
            at_ms,
            "seriko: part 呼び出しで再生を始めた（頭から・要件 3.1・3.8）"
        );
    }

    fn stop(&mut self, animation: u32) {
        let key = (self.part, animation);
        self.clocks.remove(&key);
        unwrite(self.pattern, self.part, animation);
        let always = self
            .anims
            .iter()
            .any(|a| a.id == animation && matches!(a.trigger, LoopTrigger::Always { .. }));
        if always {
            self.stopped.insert(key);
        }
        tracing::debug!(
            scope = self.scope.as_str(),
            slot = ?self.slot,
            part = %Label(self.part),
            animation_id = animation,
            always,
            "seriko: part 呼び出しで再生を止めた（-1 と同じ・要件 4.1・3.8）"
        );
    }

    fn playing_ids(&self) -> Vec<u32> {
        self.clocks
            .range((self.part, 0)..=(self.part, u32::MAX))
            .filter(|(_, c)| matches!(c, PartAnim::Playing { .. }))
            .map(|(&(_, id), _)| id)
            .collect()
    }
}

/// 部品 `part` の働きのコマを `now_ms` まで当てる。結果と、呼び出しで始まった animation の番号を返す。
/// 働きのコマを持たない部品は何もしない（当てるものが無い・当て終えた印も付けない）。
#[allow(clippy::too_many_arguments)]
pub(super) fn run_part_calls(
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    stopped: &mut BTreeSet<ClockKey>,
    pattern: &mut PatternState,
    scope: &ActorKey,
    slot: Slot,
    part: PartKey,
    anims: &[LoopAnimation],
    now_ms: u64,
    pick: &mut LoopRng,
) -> (CallOutcome, Vec<u32>) {
    let acts = |a: &LoopAnimation| {
        a.frames
            .iter()
            .any(|f| !matches!(f.act, FrameAct::Draw | FrameAct::Inert))
    };
    if !anims.iter().any(acts) {
        return (CallOutcome::default(), Vec::new());
    }
    let mut plays = PartPlays {
        clocks,
        stopped,
        pattern,
        part,
        anims,
        scope,
        slot,
        started: Vec::new(),
    };
    let outcome = run_calls(
        &mut plays,
        |id| anims.iter().find(|a| a.id == id),
        now_ms,
        pick,
    );
    (outcome, plays.started)
}

/// 時計 1 本の今のコマを `now_ms` で欄へ置く: `always` は周の答え（時計が無ければ経過 0＝載せない）、
/// ほかは刻みの進行（[`progress`]）。
#[allow(clippy::too_many_arguments)]
pub(super) fn place(
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    warned_negative: &mut HashSet<(ActorKey, Slot, ClockKey)>,
    pattern: &mut PatternState,
    scope: &ActorKey,
    slot: Slot,
    part: PartKey,
    anim: &LoopAnimation,
    now_ms: u64,
) {
    let LoopTrigger::Always { period_ms, laps } = anim.trigger else {
        progress(
            clocks,
            warned_negative,
            pattern,
            scope,
            slot,
            part,
            anim,
            now_ms,
        );
        return;
    };
    if let Some(&PartAnim::Playing { started_at_ms, .. }) = clocks.get(&(part, anim.id)) {
        let elapsed = now_ms.saturating_sub(started_at_ms);
        write_always(pattern, part, anim, period_ms, laps, elapsed);
    }
}

impl PartClocks {
    /// 面を出した（spec: seriko-pattern-calls 4.5・3.8）: `scope` の `slot` の面の、呼び出しで止めた
    /// `always` の印を消す。消したら真。
    pub(crate) fn on_surface_shown(&mut self, scope: &ActorKey, slot: Slot) -> bool {
        self.clocks
            .get_mut(scope)
            .and_then(|c| c.get_mut(&slot))
            .is_some_and(|c| {
                let had = !c.stopped.is_empty();
                c.stopped.clear();
                had
            })
    }
}
