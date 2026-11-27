//! parts の子: 部品の働きのコマの配線。
//!
//! 部品 1 つの時計（入れ物の clocks）と部品の欄を一緒に [`CallPlays`] として貸し、[`run_calls`] で
//! 当てる（相手は同じ部品の面の animation）。呼ぶのは引き金の後と、抽選・`always` の始まりの後の 2 回、
//! コマを置く前（一番上と同じ・同じ時刻の 2 度目の開始の止めはこの順が前提）。`Residual`（終わって最後のコマを
//! 残したもの）は再生中に数えない。呼ばれるだけ（`never`）の時計は、抽選で始まった時計と同じく、
//! 部品が見えない間も捨てない（見えなかった間も進んでいた扱い・parts の `drop_transient_where`）。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use areka_emo_compose::{PartKey, PatternState};
use areka_sakura::ActorKey;
use dola::TimePoint;

use super::{ClockKey, Label, PartAnim, PartClocks, progress, unwrite, write_always};
use crate::calls::{CallOutcome, CallPlays, run_calls};
use crate::state::Slot;
use crate::table::{FrameAct, LoopAnimation, LoopTrigger};
use crate::timeline::{LoopRng, elapsed_since};

/// 部品 1 つの時計と欄を Calls へ貸す口。
struct PartPlays<'a> {
    clocks: &'a mut BTreeMap<ClockKey, PartAnim>,
    /// 止めた `always`（面を出すまで始め直さない）。
    stopped: &'a mut BTreeSet<ClockKey>,
    pattern: &'a mut PatternState,
    part: PartKey,
    scope: &'a ActorKey,
    slot: Slot,
    started: Vec<u32>,
}

impl CallPlays for PartPlays<'_> {
    fn playing(&self, animation: u32) -> Option<(TimePoint, Option<TimePoint>)> {
        match self.clocks.get(&(self.part, animation)) {
            Some(&PartAnim::Playing {
                started_at,
                acts_through,
            }) => Some((started_at, acts_through)),
            _ => None,
        }
    }

    fn set_acts_through(&mut self, animation: u32, through: TimePoint) {
        if let Some(PartAnim::Playing { acts_through, .. }) =
            self.clocks.get_mut(&(self.part, animation))
        {
            *acts_through = Some(through);
        }
    }

    fn start(&mut self, animation: u32, at: TimePoint) {
        let key = (self.part, animation);
        self.clocks.insert(
            key,
            PartAnim::Playing {
                started_at: at,
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
            %at,
            "seriko: part 呼び出しで再生を始めた（頭から・要件 3.1・3.8）"
        );
    }

    fn stop(&mut self, animation: u32, always: bool) {
        let key = (self.part, animation);
        self.clocks.remove(&key);
        unwrite(self.pattern, self.part, animation);
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

/// 部品 `part` の働きのコマを `now` まで当てる。結果と、呼び出しで始まった animation の番号を返す。
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
    now: TimePoint,
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
        scope,
        slot,
        started: Vec::new(),
    };
    let outcome = run_calls(
        &mut plays,
        |id| anims.iter().find(|a| a.id == id),
        now,
        pick,
    );
    (outcome, plays.started)
}

/// 時計 1 本の今のコマを `now` で欄へ置く: `always` は周の答え（時計が無ければ経過 0＝載せない）、
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
    now: TimePoint,
) {
    let LoopTrigger::Always { period, laps } = anim.trigger else {
        progress(
            clocks,
            warned_negative,
            pattern,
            scope,
            slot,
            part,
            anim,
            now,
        );
        return;
    };
    if let Some(&PartAnim::Playing { started_at, .. }) = clocks.get(&(part, anim.id)) {
        let elapsed = elapsed_since(now, started_at);
        write_always(pattern, part, anim, period, laps, elapsed);
    }
}

impl PartClocks {
    /// 面を出した: `scope` の `slot` の面の、呼び出しで止めた
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
