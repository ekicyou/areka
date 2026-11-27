//! looper のテストが共有する足場。

use super::tests::{cfg, counting_rng};
use super::*;
use crate::parts::PartAnim;
use crate::resolve::SurfaceTarget;
use crate::state::{ApplyOutcome, StageNote};
use crate::test_support::{empty_states, scope, table_of};

/// シェルの表が `text` のランタイムと、まだ何も表示していない状態。
pub(super) fn shell_runtime(text: &str) -> (LoopRuntime, ScopeStates) {
    let (rng, _probe) = counting_rng(&[]);
    (LoopRuntime::new(cfg(table_of(text), rng)), empty_states())
}

/// アクターの面の切り替えと同じ順（`catch_up` → `apply` → `on_surface_shown` → `on_surface_changed` →
/// `refresh`）で踏む（出来事の時刻は `at`）。面が変わらなければ、同じ面の出し直しで始め直す絵も出さずに
/// `None`。
pub(super) fn switch(
    rt: &mut LoopRuntime,
    states: &mut ScopeStates,
    target: SurfaceTarget,
    at: TimePoint,
) -> Option<DisplayCommand> {
    let _ = rt.catch_up(&scope(), Slot::Shell, at, states);
    let outcome = states.apply(&scope(), target);
    rt.on_surface_shown(&scope(), Slot::Shell);
    let ApplyOutcome::Changed(cmd) = outcome else {
        return None;
    };
    rt.on_surface_changed(&scope(), Slot::Shell);
    Some(rt.refresh(&scope(), Slot::Shell, at, states).unwrap_or(cmd))
}

/// 一番上の animation `id` の再生の開始の時刻（再生中でなければ `None`）。
pub(super) fn started(rt: &LoopRuntime, slot: Slot, id: u32) -> Option<TimePoint> {
    rt.playback
        .get(&(scope(), slot))
        .and_then(|pb| pb.get(&id))
        .map(|p| p.started_at)
}

/// スコープ 0 のバルーンの窓が開いた（`open`）か閉じたこと（面 `face`）を知らせる。
pub(super) fn note_balloon(states: &mut ScopeStates, open: bool, face: u32) {
    states.note_stage(&StageNote::Balloon {
        scope: scope(),
        open,
        face,
        generation: 0,
    });
}

/// スコープ 0 の `slot` の面の動く絵の子 `film` の時計（無ければ `None`）。
pub(super) fn film_clock(
    rt: &LoopRuntime,
    slot: Slot,
    film: areka_emo_compose::FilmId,
) -> Option<PartAnim> {
    rt.parts
        .clock_at(&scope(), slot, areka_emo_compose::PartKey::Film(film), 0)
}

/// 一番上の animation `id` の欄の絵（載っていなければ `None`）。
pub(super) fn cell(states: &ScopeStates, slot: Slot, id: u32) -> Option<u32> {
    states
        .current_pattern(&scope(), slot)
        .get(id)
        .map(|f| f.surface_id)
}
