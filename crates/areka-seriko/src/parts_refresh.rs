//! parts の子: 面の切り替え・着せ替えの変化の直後の作り直し（[`PartClocks::refresh`]・spec:
//! areka-P0-surface-element-nesting 要件 5.6・areka-P0-animated-image-playback 要件 3.1・4.1・
//! areka-P0-seriko-trigger-intervals 要件 3.1・5.9・6.1）。

use std::collections::BTreeMap;

use areka_emo_compose::{BindSet, PartKey, PatternState};
use areka_sakura::ActorKey;

use super::part_calls::run_part_calls;
use super::part_trigger::{fire_part_triggers, judge_part};
use super::{
    ClockKey, Container, Gate, Look, PartAnim, PartClocks, Phase, always_start, close,
    drop_unseen_transient, gate, look, rebuild, sort_by_id, unwrite, write, write_always,
};
use crate::calls::CallOutcome;
use crate::state::Slot;
use crate::table::{AnimationTable, LoopAnimation, LoopTrigger};
use crate::timeline::LoopRng;
use crate::trigger::TalkWindow;

impl PartClocks {
    /// 面の切り替え・着せ替えの変化の直後（出来事の時刻 `at_ms`）に、抽選せずに `pattern` の部品の欄を
    /// 作り直す（要件 5.6・spec: areka-P0-animated-image-playback 要件 3.1・4.1）。見える部品の求め方は
    /// `advance` と同じ。見えている `always` の時計が無ければ `at_ms` で作り、見えなくなった部品の回数
    /// つきの時計と 3 語の時計（と引き金の状態）を捨てる。3 語は刻みと同じ形（`talk` は面の文字の窓
    /// `talk`）で `at_ms` まで判定する（新しく見えた部品の `runonce` は `at_ms` で鳴る）。抽選の animation の
    /// 時計は書き換えない（末尾に着いたコマ・負の番号のコマも、時計を
    /// 移さず・消さずに、出す・出さないだけを決める）。`bind+random` で `binds` に無い animation の
    /// コマは書かない（時計を消すのは次の `advance`・要件 5.12）。`at_ms` が無い（刻みが 1 度も来て
    /// いない）ときは時計を作らず、経過 0 の時刻で読む。`open` が偽なら `advance` と同じく時計を
    /// 作らず、回数つきの時計と 3 語の時計を全部捨てて、引き金の状態を隠す。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn refresh(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        at_ms: Option<u64>,
        open: bool,
        talk: Option<&TalkWindow<'_>>,
        pick: &mut LoopRng,
        pattern: &mut PatternState,
    ) -> CallOutcome {
        let PartClocks {
            clocks,
            warned_negative,
            visible,
            films,
            evaluated,
            order,
        } = self;
        let container = clocks
            .entry(scope.clone())
            .or_default()
            .entry(slot)
            .or_default();
        if !open {
            close(container, table, scope, slot);
        }
        let Container {
            clocks,
            armed,
            stopped,
        } = &mut *container;
        let now_ms = at_ms.unwrap_or(0);
        // 窓が閉じている間は時計を作らない（今ある時計を読むだけ）。
        let create_at = at_ms.filter(|_| open);
        let mut shift = CallOutcome::default();
        // 時計を書き換えずに今のコマを読んで置く（`always` は周の答え・時計が無ければ載せない）。
        let put = |clocks: &BTreeMap<ClockKey, PartAnim>,
                   pattern: &mut PatternState,
                   part: PartKey,
                   anim: &LoopAnimation,
                   at: u64| {
            let clock = clocks.get(&(part, anim.id)).copied();
            match (anim.trigger, clock) {
                (
                    LoopTrigger::Always { period_ms, laps },
                    Some(PartAnim::Playing { started_at_ms, .. }),
                ) => {
                    let elapsed = at.saturating_sub(started_at_ms);
                    write_always(pattern, part, anim, period_ms, laps, elapsed);
                }
                (LoopTrigger::Always { .. }, _) => {}
                _ => {
                    if let Look::Frame(i) | Look::Finished(i) = look(anim, clock, at) {
                        write(pattern, part, anim, i);
                    }
                }
            }
        };
        let step = |phase: Phase, part: PartKey, pattern: &mut PatternState| {
            let anims = table.part_animations(part);
            if let Phase::Trigger = phase {
                // 時刻が分からなければ構えない（次の刻みが、その時刻で構える）。構えてあった部品は
                // 1 段目が時刻の順に判定済み。ここは新しく見えた部品だけを構える。
                let Some(at) = at_ms.filter(|_| !armed.contains_key(&part)) else {
                    return false;
                };
                let who = (scope, slot, part);
                let Some((heads, outcome)) = judge_part(
                    armed,
                    clocks,
                    stopped,
                    warned_negative,
                    pattern,
                    anims,
                    who,
                    (at, open),
                    talk,
                    pick,
                ) else {
                    return false;
                };
                shift = shift.then(outcome);
                for anim in heads {
                    // 頭のコマをこの評価の絵に載せる（待ちの在る頭は次の刻みが置く）。
                    unwrite(pattern, part, anim.id);
                    put(clocks, pattern, part, anim, at);
                }
                return true;
            }
            sort_by_id(anims, order);
            // 構えてあった部品の 3 語も刻みと同じく、出来事の時刻まで働きのコマと時刻の順に揃えて判定し、
            // 働きのコマを出来事の時刻まで当ててから `always` を始める（出来事の時刻に始まる再生に、
            // それより前の呼び出しを当てない・spec: seriko-pattern-calls 3.1・3.8）。
            if let Some(at) = at_ms {
                if let Some(state) = armed.get_mut(&part) {
                    state.follow(open, at, None);
                    let who = (scope, slot, part);
                    let (_, early, _) = fire_part_triggers(
                        state,
                        clocks,
                        stopped,
                        warned_negative,
                        pattern,
                        anims,
                        who,
                        at,
                        talk,
                        pick,
                    );
                    shift = shift.then(early);
                }
                let (outcome, _) =
                    run_part_calls(clocks, stopped, pattern, scope, slot, part, anims, at, pick);
                shift = shift.then(outcome);
            }
            for &i in order.iter() {
                let anim = &anims[i];
                if let Gate::Always { laps } = gate(anim, binds)
                    && !stopped.contains(&(part, anim.id))
                {
                    always_start(
                        clocks,
                        (part, anim.id),
                        create_at,
                        scope,
                        slot,
                        laps.is_some(),
                    );
                }
            }
            // 始めた `always` の頭の働きのコマを当てる（頭のコマを置く前）。
            if let Some(at) = at_ms {
                let (outcome, _) =
                    run_part_calls(clocks, stopped, pattern, scope, slot, part, anims, at, pick);
                shift = shift.then(outcome);
            }
            for &i in order.iter() {
                let anim = &anims[i];
                if !matches!(gate(anim, binds), Gate::Off) {
                    put(clocks, pattern, part, anim, now_ms);
                }
            }
            false
        };
        rebuild(
            table, surface_id, binds, pattern, visible, evaluated, films, step,
        );
        drop_unseen_transient(container, table, scope, slot, visible, films);
        shift
    }
}
