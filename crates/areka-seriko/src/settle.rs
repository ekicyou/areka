//! 面 1 つを時点 T まで画面と同じ道で進める口。
//!
//! 書き出しの時点 T は、その面の surface*ブレスに書かれた `runonce` の待ちの合計のうち最も長いもの
//! （SSP 2.9.23 の測り）。同じスレッドで状態・統括器・注入の時計を組み、
//! 画面のアクターと同じ `handle_message` に「時刻 0 の刻み」「面の合図」「16ms ごとの刻み」「時刻 T の
//! 刻み」を順に渡して、そのスコープの面の状態を返す。乱数は常に当たらないので random・BindRandom は
//! 始まらない。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use dola::{TimePoint, TimeSpan};

use crate::actor::{SerikoMsg, handle_message};
use crate::bind::{BindResolver, DefaultBinds};
use crate::looper::{LoopRuntime, SerikoLoopConfig};
use crate::output::{DisplayCommand, SurfaceOutput};
use crate::resolve::SurfaceResolver;
use crate::state::{ScopeStates, Slot};
use crate::table::{AnimationTable, LoopTrigger, total_wait};

/// 本番の刻みと同じ間隔（`areka-ghost` の ticker と同じ）。
pub(crate) const SETTLE_TICK: TimeSpan = TimeSpan::from_millis(16);

/// 進める刻みの数の上限（16ms × 37,500 ＝ 10 分）。作者の書いた長すぎる待ちで書き出しが止まらないように。
pub(crate) const SETTLE_MAX_TICKS: u64 = 37_500;

/// その面の `runonce` の待ちの合計の最大（無ければ 0・部品の `runonce` は数えない）。
pub fn settle_time(table: &AnimationTable, surface_id: u32) -> TimeSpan {
    table
        .animations(surface_id)
        .iter()
        .filter(|a| a.trigger == LoopTrigger::Runonce)
        .map(|a| total_wait(&a.frames))
        .max()
        .unwrap_or(TimeSpan::ZERO)
}

/// 指令を捨てる出力（書き出しは最後の状態だけを読む）。
struct Discard;

impl SurfaceOutput for Discard {
    fn send(&mut self, _command: DisplayCommand) {}
}

/// 面を時刻 0 に見せ始め、[`settle_time`] まで画面と同じ道で進めた状態を返す（同じスレッド・決定論）。
/// [`settle_time`] が `SETTLE_MAX_TICKS` 刻み（10 分）を越える面は `warn!` を 1 行残し、上限の時刻で止める。
///
/// 刻みの位相は合図と同じ時刻 0 から 16ms ごとに固定する（画面は絶対の格子に揃うので合図から最初の
/// 刻みまでは 0〜16ms のどれか。`runonce` の最後のコマはどちらでも同じ）。
pub fn settle_surface(
    table: &AnimationTable,
    scope: u32,
    binds: &BindSet,
    surface_id: u32,
) -> PatternState {
    let wait = settle_time(table, surface_id);
    let until = wait.min(SETTLE_TICK.mul_div(SETTLE_MAX_TICKS, 1));
    if until < wait {
        tracing::warn!(
            surface_id,
            max_ticks = SETTLE_MAX_TICKS,
            wait = %wait,
            "seriko: settle runonce の待ちの合計が上限を越える——上限の時点で打ち切って書き出す"
        );
    }
    let clock = Arc::new(AtomicI64::new(0));
    let read = Arc::clone(&clock);
    let resolver = SurfaceResolver::new(BTreeMap::new());
    let bind_resolver = BindResolver::empty();
    let mut states = ScopeStates::new(DefaultBinds::single(scope, binds.clone()));
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: table.clone(),
        balloon_tables: BTreeMap::new(),
        rng: Box::new(|_| 1),
        pick_rng: crate::seeded_rng(0),
    })
    .with_clock(Some(Arc::new(move || {
        TimePoint::from_ticks(read.load(Ordering::SeqCst))
    })));
    let actor = ActorKey::from(scope.to_string());
    let mut send = |now: TimePoint, msg: SerikoMsg| {
        clock.store(now.as_ticks(), Ordering::SeqCst);
        let _ = handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut rt,
            &mut Discard,
            msg,
        );
    };

    let start = TimePoint::ZERO;
    send(start, SerikoMsg::Tick { now: start });
    send(
        start,
        SerikoMsg::Cue(TalkCue::scheduled(
            start,
            TimeSpan::ZERO,
            actor.clone(),
            CueCommand::Emote {
                key: surface_id.to_string(),
            },
            TimeSpan::ZERO,
        )),
    );
    let end = start + until;
    let mut now = start + SETTLE_TICK;
    while now < end {
        send(now, SerikoMsg::Tick { now });
        now = now + SETTLE_TICK;
    }
    send(end, SerikoMsg::Tick { now: end });
    drop(send);
    states.current_pattern(&actor, Slot::Shell).clone()
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
