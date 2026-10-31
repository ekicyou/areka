//! 一番上の面の働きのコマの配線（spec: seriko-pattern-calls 要件 1.3・3.1・3.4・3.5・3.8・3.9・4.1・
//! 4.5・7.7）。
//!
//! 面の段の再生の表と作りかけの欄を一緒に [`CallPlays`] として貸し、[`run_calls`] で当てる。呼ぶのは
//! 抽選・`always`・引き金の始まりの後、コマを置く前（同じ時刻の 2 度目の開始の止めはこの順が前提）。
//! 欄に置くのは描くコマだけ（働きのコマは絵を重ねず、前の絵を残す）。

use std::collections::{HashMap, HashSet};

use areka_emo_compose::PatternState;
use areka_sakura::ActorKey;

use super::{LoopRuntime, Playback, SlotPlayback, pattern_frame};
use crate::calls::{CallOutcome, CallPlays, run_calls};
use crate::output::DisplayCommand;
use crate::state::Slot;
use crate::table::{LoopAnimation, LoopTrigger};
use crate::timeline::{AlwaysView, LoopRng, always_visible, visible_frame};

/// 止めた `always` の印（スコープ, 面の段, animation）。印がある間は始め直さず、面を出したときに消す
/// （SSP の測り M2・2026-10-30 設計議題 1）。
pub(super) type StoppedAlways = HashSet<(ActorKey, Slot, u32)>;

/// ずれの持ち主（spec: seriko-pattern-calls 要件 3.8・7.1・7.2・7.6・9.6）: (スコープ, 面の段) ごとの
/// 今のずれ（無ければ 0）と、まだ出していないずれの命令。ずれは `PatternState` に入れない（書き出しに
/// 写らない・M8）。
#[derive(Default)]
pub(super) struct Shifts {
    now: HashMap<(ActorKey, Slot), (i64, i64)>,
    outbox: Vec<DisplayCommand>,
}

impl Shifts {
    /// 一番上と部品を合わせた結果に move があれば、その段のずれにする（部品の move もキャラクター
    /// 全体のずれ・3.8）。
    pub(super) fn apply(&mut self, scope: &ActorKey, slot: Slot, outcome: CallOutcome) {
        if let Some((_, x, y)) = outcome.shift {
            self.set(scope, slot, (x, y));
        }
    }

    /// 面を出した（同じ面の出し直し・シェルの差し替えを含む）: 0 へ戻す。終わり・`stop`・`-1` では
    /// 呼ばない（7.6・M3）。
    pub(super) fn reset(&mut self, scope: &ActorKey, slot: Slot) {
        self.set(scope, slot, (0, 0));
    }

    /// 積んだ命令を順に取り出す。
    pub(super) fn take_all(&mut self) -> Vec<DisplayCommand> {
        std::mem::take(&mut self.outbox)
    }

    /// 値が変わったときだけ命令を積む（同じ値は繰り返さない）。
    fn set(&mut self, scope: &ActorKey, slot: Slot, xy: (i64, i64)) {
        let key = (scope.clone(), slot);
        if self.now.get(&key).copied().unwrap_or((0, 0)) == xy {
            return;
        }
        if xy == (0, 0) {
            self.now.remove(&key);
        } else {
            self.now.insert(key, xy);
        }
        let (scope, (x, y)) = (scope.clone(), xy);
        tracing::debug!(
            scope = scope.as_str(),
            ?slot,
            x,
            y,
            "seriko: ずれが変わった（要件 7.1・7.6）"
        );
        self.outbox.push(match slot {
            Slot::Shell => DisplayCommand::MoveShift { scope, x, y },
            Slot::Balloon => DisplayCommand::MoveShiftBalloon { scope, x, y },
        });
    }
}

impl LoopRuntime {
    /// まだ出していないずれの命令を積んだ順に取り出す（刻みは [`LoopRuntime::on_tick`] が `Show` の後に
    /// 付け、合図は actor が合図 1 件の処理の後に出す）。
    pub(crate) fn take_shifts(&mut self) -> Vec<DisplayCommand> {
        self.shifts.take_all()
    }
}

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
    let view = |elapsed: u64| always_visible(frames, period_ms, laps, elapsed);
    match view(elapsed_ms) {
        now if now == view(0) => pattern.remove(anim.id),
        Some(AlwaysView::Frame(i)) => pattern.set(anim.id, pattern_frame(&frames[i])),
        Some(AlwaysView::Nothing) => pattern.set_blank(anim.id),
        None => pattern.remove(anim.id),
    }
}
