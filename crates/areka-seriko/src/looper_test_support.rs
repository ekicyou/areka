//! looper のテストが共有する足場（spec: areka-P0-seriko-trigger-intervals）。

use super::tests::{cfg, counting_rng};
use super::*;
use crate::resolve::SurfaceTarget;
use crate::state::ApplyOutcome;
use crate::test_support::{empty_states, scope, table_of};

/// シェルの表が `text` のランタイムと、まだ何も表示していない状態。
pub(super) fn shell_runtime(text: &str) -> (LoopRuntime, ScopeStates) {
    let (rng, _probe) = counting_rng(&[]);
    (LoopRuntime::new(cfg(table_of(text), rng)), empty_states())
}

/// アクターの面の切り替えと同じ順で踏む（出来事の時刻は `at_ms`）。面が変わらなければ `None`。
pub(super) fn switch(
    rt: &mut LoopRuntime,
    states: &mut ScopeStates,
    target: SurfaceTarget,
    at_ms: u64,
) -> Option<DisplayCommand> {
    switch_at(rt, states, target, Some(at_ms))
}

/// [`switch`] の出来事の時刻を `Option` で渡す形（`None`＝時計の無い起動と同じく直前の刻みの時刻）。
pub(super) fn switch_at(
    rt: &mut LoopRuntime,
    states: &mut ScopeStates,
    target: SurfaceTarget,
    at_ms: Option<u64>,
) -> Option<DisplayCommand> {
    let ApplyOutcome::Changed(cmd) = states.apply(&scope(), target) else {
        return None;
    };
    rt.on_surface_changed(&scope(), Slot::Shell);
    Some(
        rt.refresh(&scope(), Slot::Shell, at_ms, states)
            .unwrap_or(cmd),
    )
}

/// 一番上の animation `id` の再生の開始の時刻（再生中でなければ `None`）。
pub(super) fn started(rt: &LoopRuntime, slot: Slot, id: u32) -> Option<u64> {
    rt.playback
        .get(&(scope(), slot))
        .and_then(|pb| pb.get(&id))
        .map(|p| p.started_at_ms)
}

/// 一番上の animation `id` の欄の絵（載っていなければ `None`）。
pub(super) fn cell(states: &ScopeStates, slot: Slot, id: u32) -> Option<u32> {
    states
        .current_pattern(&scope(), slot)
        .get(id)
        .map(|f| f.surface_id)
}

/// `needle` を含む行の数。
pub(super) fn count(lines: &[String], needle: &str) -> usize {
    lines.iter().filter(|l| l.contains(needle)).count()
}
