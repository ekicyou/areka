//! looper の子: 台本の合図の後の作り直し（[`LoopRuntime::refresh`]・spec:
//! areka-P0-surface-element-nesting 要件 5.6・areka-P0-animated-image-playback 要件 2.3・3.1・4.1・4.3・
//! areka-P0-seriko-trigger-intervals 要件 2・5.5・6.1）。

use areka_sakura::ActorKey;

use super::top_calls::{put_top_always, put_visible, run_top_calls};
use super::{
    LoopRuntime, SerikoLoopConfig, arm_slot, fire_top_triggers, glyph_wall_ms, revealed_at,
    slot_binds, start_top_always, talk_window, top_kinds,
};
use crate::output::DisplayCommand;
use crate::state::{PatternApplyOutcome, ScopeStates, Slot};
use crate::table::LoopTrigger;
use crate::timeline::{FrameStatus, frame_at};

impl LoopRuntime {
    /// 面の切り替え・着せ替えの変化の直後に呼ぶ（spec: areka-P0-surface-element-nesting 要件 5.6・
    /// spec: areka-P0-animated-image-playback 要件 2.3・3.1・4.1・4.3）。
    ///
    /// 繰り返しの経路を通す表（[`AnimationTable::is_continuous`]）で、`scope` の `slot` の面が
    /// 進行の対象（[`ScopeStates::stage_slots`]・バルーンは知らせの面も）なら、保持している
    /// [`PatternState`] の写しを出来事の時刻 `at_ms`（無ければ直前の刻みの時刻）で作り直す: 一番上の `always` の再生が無ければその時刻で作って欄を置き、部品の欄は
    /// [`PartClocks::refresh`]（見えている `always` の時計を作る・回数つきの見えなくなった時計を捨てる・
    /// 抽選の時計と抽選の乱数は触らない（働きのコマの始め・止めと選びの乱数を除く））で作り直す。`commit_pattern` が変化を返したらその `Show` を返す。
    /// 面が無い（`\s[-1]`・`\b[-1]` の後）なら、その面の回数つきの時計と、部品の 3 語の時計・引き金の
    /// 状態を捨てて何も返さない（[`PartClocks::drop_hidden`]）。
    /// バルーンの窓が閉じていれば時計を作らない（回数つきは捨てる）。
    /// 出来事の時刻は刻みの単調性の番人（`last_seen`）に入れない。刻みが 1 度も来ておらず `at_ms` も
    /// 無ければ時計を作らない（次の刻みで生まれる）。
    ///
    /// 引き金（spec: areka-P0-seriko-trigger-intervals 要件 2・3.1・4.1・5.5・6.1）: 構える条件を満たす面に
    /// 引き金の状態が無ければ（＝[`LoopRuntime::on_surface_changed`] が捨てた後＝面に入った）出来事の
    /// 時刻で構え、一番上の `runonce` をその時刻で鳴らして頭のコマを載せる。状態が在れば（着せ替え・
    /// 窓の知らせ）構え直さず、窓の開け閉めだけを写す。どちらでも 3 語を刻みと同じ形（文字の窓も）で
    /// 出来事の時刻まで判定してから働きのコマを当て、終わりに文字の数えを出来事の時刻まで進める。
    /// 時刻が分からなければ構えない（次の刻みで構える）。
    pub(crate) fn refresh(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        at_ms: Option<u64>,
        states: &mut ScopeStates,
    ) -> Option<DisplayCommand> {
        let LoopRuntime {
            config,
            playback,
            warned_negative,
            parts,
            last_seen,
            armed,
            epoch,
            feeds,
            stopped_always,
            shifts,
            ..
        } = self;
        let SerikoLoopConfig {
            shell_table,
            balloon_tables,
            pick_rng,
            ..
        } = config;
        let table = match slot {
            Slot::Shell => &*shell_table,
            Slot::Balloon => balloon_tables.get(scope)?,
        };
        if !table.is_continuous() {
            return None;
        }
        let Some((_, _, sid, open)) = states
            .stage_slots()
            .into_iter()
            .find(|(s, sl, ..)| s == scope && *sl == slot)
        else {
            parts.drop_hidden(scope, slot, table);
            return None;
        };
        let at_ms = at_ms.or(*last_seen);
        let mut pattern = states.current_pattern(scope, slot).clone();
        let mut shift = Default::default();
        let key = (scope.clone(), slot);
        let (_, has_trigger) = top_kinds(table, sid);
        let talks = table.has_talk();
        // 引き金: 構える条件を満たす面でだけ構える（面に入った・時刻が分からなければ構えない）。文字は
        // この出来事の時刻までに現れた数の次から数える。
        let revealed = at_ms
            .filter(|_| talks && open)
            .map(|at| revealed_at(epoch, feeds, scope, at));
        let state = at_ms
            .filter(|_| has_trigger || talks)
            .map(|at| arm_slot(armed, &key, sid, open, at, revealed));
        // 文字の窓（刻みと同じ形・部品も借りる）。窓が在るのは時刻が分かるときだけ。
        let bounds = state
            .as_ref()
            .and_then(|state| state.talk_window_bounds())
            .zip(revealed);
        let wall_ms = |glyph| glyph_wall_ms(epoch, feeds.get(scope), glyph, at_ms.unwrap_or(0));
        let window = talk_window(bounds, &wall_ms);
        if let Some(at) = at_ms {
            // 頭のコマを置く `always` 以外の再生（鳴らした引き金と呼び出しで始まったもの）。
            let mut heads = Vec::new();
            // 3 語を刻みと同じく出来事の時刻まで判定し、頭のコマをこの評価の絵に載せる（待ちの在る頭は
            // 次の刻みが置く・areka-P0-seriko-trigger-intervals 要件 3.1・4.1・6.1）。
            if let Some(state) = state.filter(|_| has_trigger) {
                let all = table.animations(sid);
                let anims = all.iter().filter(|a| a.trigger.is_word());
                let (fired, early, called) = fire_top_triggers(
                    state,
                    playback,
                    &mut pattern,
                    warned_negative,
                    &key,
                    anims,
                    at,
                    window.as_ref(),
                    (stopped_always, all, pick_rng),
                );
                heads.extend(fired.iter().map(|(anim, _)| anim.id).chain(called));
                shift = early;
            }
            // 働きのコマを合図の時刻まで当ててから `always` を始め、その頭の働きのコマを当てる（頭のコマを
            // 置く前・合図の時刻に始まる再生に、それより前の呼び出しを当てない・spec:
            // seriko-pattern-calls 3.1）。
            let (top, called) = run_top_calls(
                playback,
                &mut pattern,
                stopped_always,
                &key,
                table.animations(sid),
                at,
                pick_rng,
            );
            shift = shift.then(top);
            heads.extend(called);
            if open {
                start_top_always(playback, stopped_always, table, scope, slot, sid, at);
                let (top, called) = run_top_calls(
                    playback,
                    &mut pattern,
                    stopped_always,
                    &key,
                    table.animations(sid),
                    at,
                    pick_rng,
                );
                shift = shift.then(top);
                heads.extend(called);
            }
            for anim in table.animations(sid) {
                let Some(p) = playback.get(&key).and_then(|pb| pb.get(&anim.id)) else {
                    continue;
                };
                let elapsed = at.saturating_sub(p.started_at_ms);
                if matches!(anim.trigger, LoopTrigger::Always { .. }) {
                    put_top_always(&mut pattern, anim, elapsed);
                } else if heads.contains(&anim.id) {
                    // 刻みの進行と同じく、先頭の待ちの前と止まった頭は前の再生の残りのコマを外す。
                    match frame_at(&anim.frames, elapsed) {
                        FrameStatus::Active(i) | FrameStatus::FinishedResidual(i) => {
                            put_visible(&mut pattern, anim, i);
                        }
                        _ => {
                            pattern.remove(anim.id);
                        }
                    }
                }
            }
        }
        // 部品は面の種類を問わない（バルーンの面の着せ替えの集合は空）。
        if table.has_animated_parts() {
            shift = shift.then(parts.refresh(
                scope,
                slot,
                sid,
                slot_binds(states, scope, slot),
                table,
                at_ms,
                open,
                window.as_ref(),
                pick_rng,
                &mut pattern,
            ));
        }
        // 一番上も部品も窓を読み終えた: この面の数えを出来事の時刻まで進める（同じ区切りを次の刻みで
        // 2 度鳴らさない。文字の列の刈り込みは刻みの最後だけ）。
        if let Some(now_seen) = revealed
            && let Some(state) = armed.get_mut(&key)
        {
            state.advance_talk(now_seen);
        }
        let shown = states.commit_pattern(scope, slot, pattern);
        shifts.apply(scope, slot, shift);
        match shown {
            PatternApplyOutcome::Changed(cmd) => Some(cmd),
            PatternApplyOutcome::Unchanged => None,
        }
    }
}
