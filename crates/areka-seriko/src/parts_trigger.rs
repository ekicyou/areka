//! parts の子: 部品の引き金の配線（spec: areka-P0-seriko-trigger-intervals 要件 5.4・5.6・5.7・5.9）。
//!
//! [`PartClocks`](super::PartClocks) の評価が部品 1 つごとに呼ぶ [`arm_part`]（構える）と
//! [`fire_part_triggers`]（判定して時計を作る）を置く。「いつ始めるか」の判定は `trigger.rs`、始まった
//! 時計のコマの進み方は親（抽選の時計と同じ経路）が持ち、ここは配線だけ。

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use areka_emo_compose::{PartKey, PatternState};
use areka_sakura::ActorKey;

use super::part_calls::run_part_calls;
use super::{ClockKey, Label, PartAnim, progress, unwrite};
use crate::calls::CallOutcome;
use crate::state::Slot;
use crate::table::{LoopAnimation, LoopTrigger};
use crate::timeline::{FrameStatus, LoopRng, frame_at};
use crate::trigger::{Armed, TalkWindow, log_fired};

/// `runonce`・`periodic`・`talk` を持つ部品なら引き金の状態を返す（無ければ `at_ms` を見え始めの時刻と
/// して構える・在れば窓の開け閉めを写す）。3 語を持たない部品は `None`＝状態も記録も生まれない（spec:
/// areka-P0-seriko-trigger-intervals 要件 5.4・5.6・5.9・8.1）。
pub(super) fn arm_part<'s>(
    armed: &'s mut BTreeMap<PartKey, Armed>,
    anims: &[LoopAnimation],
    scope: &ActorKey,
    slot: Slot,
    part: PartKey,
    at_ms: u64,
    open: bool,
) -> Option<&'s mut Armed> {
    if !anims.iter().any(|a| a.trigger.is_word()) {
        return None;
    }
    Some(match armed.entry(part) {
        Entry::Occupied(entry) => {
            let state = entry.into_mut();
            state.follow(open, at_ms, None);
            state
        }
        Entry::Vacant(entry) => {
            tracing::debug!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                at_ms,
                "seriko: part trigger 部品が見えた（引き金を構えた）"
            );
            entry.insert(Armed::arm(at_ms, open, None))
        }
    })
}

/// 部品 `part` の 3 語の animation を開始の時刻の順（同じ時刻は番号の順）に、その時刻までの部品の
/// 働きのコマを当ててから判定し、始まるものの時計を判定が返した開始の時刻（`runonce` は見え始め・
/// `periodic` は周の境目・`talk` は区切りの文字が現れた時刻で、`at_ms` ではない）で作る。一番上の
/// `fire_top_triggers` と同じ形（spec: areka-P0-seriko-trigger-intervals 要件 3.1・3.3・5.4・6.1・
/// seriko-pattern-calls 要件 3.1・3.8）。始まった animation・働きのコマの結果・呼び出しで始まった
/// animation の番号を返す（`at_ms` までのコマを置くのは呼び手）。
///
/// 「再生中か」は、開始の時刻までの働きのコマを当てた後の時計で、開始の時刻に測る: 時計がその時刻に
/// まだ終わり（負の番号で停止・末尾）でなければ再生中（要件 3.3・5.3）。呼び出しが止めた再生は再生中で
/// なく、呼び出しがちょうど開始の時刻に始めた再生は鳴らした扱いで入れ替えない（seriko-pattern-calls
/// 要件 5.2）。開始の時刻には終えていて、まだ片付けていない再生は、その時刻まで進めて片付けてから
/// （終わりの記録は 1 件）入れ替える。
///
/// `talk` はこの面の文字の窓（部品は文字の数えを持たず、面の数えを借りる・要件 4.1・4.5・5.6）。
/// `runonce_only` は出来事の直後の評価（`periodic`・`talk` は刻みが鳴らす）。乱数は引かない。始めない
/// 経路（隠れている・再生中・境目の前）は記録しない（要件 5.7・7.5）。
#[allow(clippy::too_many_arguments)]
pub(super) fn fire_part_triggers<'a>(
    state: &mut Armed,
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    stopped: &mut BTreeSet<ClockKey>,
    warned_negative: &mut HashSet<(ActorKey, Slot, ClockKey)>,
    pattern: &mut PatternState,
    anims: &'a [LoopAnimation],
    (scope, slot, part): (&ActorKey, Slot, PartKey),
    at_ms: u64,
    talk: Option<&TalkWindow<'_>>,
    runonce_only: bool,
    pick: &mut LoopRng,
) -> (Vec<&'a LoopAnimation>, CallOutcome, Vec<u32>) {
    let mut due: Vec<(u64, &LoopAnimation)> = anims
        .iter()
        .filter(|a| a.trigger.is_word() && (!runonce_only || a.trigger == LoopTrigger::Runonce))
        .filter_map(|a| state.next_start(a, at_ms, talk).map(|at| (at, a)))
        .collect();
    due.sort_by_key(|&(at, a)| (at, a.id));
    let (mut started, mut outcome, mut called) = (Vec::new(), CallOutcome::default(), Vec::new());
    for (started_at_ms, anim) in due {
        let (early, by_call) = run_part_calls(
            clocks,
            stopped,
            pattern,
            scope,
            slot,
            part,
            anims,
            started_at_ms,
            pick,
        );
        outcome = outcome.then(early);
        called.extend(by_call);
        let key = (part, anim.id);
        let prev = match clocks.get(&key) {
            Some(&PartAnim::Playing { started_at_ms, .. }) => Some(started_at_ms),
            _ => None,
        };
        let playing = prev.is_some_and(|p| {
            p != started_at_ms
                && matches!(
                    frame_at(&anim.frames, started_at_ms.saturating_sub(p)),
                    FrameStatus::Pending | FrameStatus::Active(_)
                )
        });
        if !state.commit(anim, started_at_ms, playing) {
            continue;
        }
        if prev != Some(started_at_ms) {
            if prev.is_some() {
                progress(
                    clocks,
                    warned_negative,
                    pattern,
                    scope,
                    slot,
                    part,
                    anim,
                    started_at_ms,
                );
            }
            // 前の再生のコマは外す＝鳴り直した後は新しい再生の経過だけで決まる。
            unwrite(pattern, part, anim.id);
            clocks.insert(
                key,
                PartAnim::Playing {
                    started_at_ms,
                    acts_through: None,
                },
            );
        }
        log_fired(anim, started_at_ms, scope, slot, Some(&Label(part)));
        started.push(anim);
    }
    (started, outcome, called)
}

/// 2 段目（見えると決まった部品 1 つ）: [`arm_part`] で構え、[`fire_part_triggers`] で判定し、何か
/// 始まったら働きのコマを `at_ms` まで当てる。始まった animation と呼び出しで始まった animation（呼び手が
/// 欄へ置く）と、働きのコマの結果を返す。何も始まらなければ `None`。
#[allow(clippy::too_many_arguments)]
pub(super) fn judge_part<'a>(
    armed: &mut BTreeMap<PartKey, Armed>,
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    stopped: &mut BTreeSet<ClockKey>,
    warned_negative: &mut HashSet<(ActorKey, Slot, ClockKey)>,
    pattern: &mut PatternState,
    anims: &'a [LoopAnimation],
    (scope, slot, part): (&ActorKey, Slot, PartKey),
    (at_ms, open): (u64, bool),
    talk: Option<&TalkWindow<'_>>,
    runonce_only: bool,
    pick: &mut LoopRng,
) -> Option<(Vec<&'a LoopAnimation>, CallOutcome)> {
    let state = arm_part(armed, anims, scope, slot, part, at_ms, open)?;
    let who = (scope, slot, part);
    let (mut heads, early, mut called) = fire_part_triggers(
        state,
        clocks,
        stopped,
        warned_negative,
        pattern,
        anims,
        who,
        at_ms,
        talk,
        runonce_only,
        pick,
    );
    if heads.is_empty() {
        return None;
    }
    let (late, by_call) = run_part_calls(
        clocks, stopped, pattern, scope, slot, part, anims, at_ms, pick,
    );
    called.extend(by_call);
    heads.extend(anims.iter().filter(|a| called.contains(&a.id)));
    Some((heads, early.then(late)))
}
