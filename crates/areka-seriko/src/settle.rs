//! 面 1 つを時点 T まで画面と同じ道で進める口（spec: areka-P0-offline-surface-dump 要件 4.1・4.7・4.9・
//! design「SettleSurface」）。
//!
//! 書き出しの時点 T は、その面の surface*ブレスに書かれた `runonce` の待ちの合計のうち最も長いもの
//! （SSP 2.9.23 の測り・research.md §10.1）。同じスレッドで状態・統括器・注入の時計を組み、
//! 画面のアクターと同じ `handle_message` に「時刻 0 の刻み」「面の合図」「16ms ごとの刻み」「時刻 T の
//! 刻み」を順に渡して、そのスコープの面の状態を返す。乱数は常に当たらないので random・BindRandom は
//! 始まらない。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::{ActorKey, CueCommand, TalkCue};

use crate::actor::{SerikoMsg, handle_message};
use crate::bind::{BindResolver, DefaultBinds};
use crate::looper::{LoopRuntime, SerikoLoopConfig};
use crate::output::{DisplayCommand, SurfaceOutput};
use crate::resolve::SurfaceResolver;
use crate::state::{ScopeStates, Slot};
use crate::table::{AnimationTable, LoopTrigger};

/// 本番の刻みと同じ間隔（ms・`areka-ghost` の ticker と同じ）。
pub const SETTLE_TICK_MS: u64 = 16;

/// その面の `runonce` の待ちの合計の最大（無ければ 0・部品の `runonce` は数えない）。
pub fn settle_time(table: &AnimationTable, surface_id: u32) -> u64 {
    table
        .animations(surface_id)
        .iter()
        .filter(|a| a.trigger == LoopTrigger::Runonce)
        .map(|a| a.frames.iter().map(|f| u64::from(f.wait_ms)).sum::<u64>())
        .max()
        .unwrap_or(0)
}

/// 指令を捨てる出力（書き出しは最後の状態だけを読む）。
struct Discard;

impl SurfaceOutput for Discard {
    fn send(&mut self, _command: DisplayCommand) {}
}

/// 面を時刻 0 に見せ始め、[`settle_time`] まで画面と同じ道で進めた状態を返す（同じスレッド・決定論）。
///
/// 刻みの位相は合図と同じ時刻 0 から 16ms ごとに固定する（画面は絶対の格子に揃うので合図から最初の
/// 刻みまでは 0〜16ms のどれか。`runonce` の最後のコマはどちらでも同じ）。
pub fn settle_surface(
    table: &AnimationTable,
    scope: u32,
    binds: &BindSet,
    surface_id: u32,
) -> PatternState {
    let until = settle_time(table, surface_id);
    let clock = Arc::new(AtomicU64::new(0));
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
    .with_clock(Some(Arc::new(move || read.load(Ordering::SeqCst))));
    let actor = ActorKey::from(scope.to_string());
    let mut send = |ms: u64, msg: SerikoMsg| {
        clock.store(ms, Ordering::SeqCst);
        let _ = handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut rt,
            &mut Discard,
            msg,
        );
    };

    send(0, SerikoMsg::Tick { now_ms: 0 });
    send(
        0,
        SerikoMsg::Cue(TalkCue {
            at: 0.0,
            actor: actor.clone(),
            command: CueCommand::Emote {
                key: surface_id.to_string(),
            },
            duration: 0.0,
        }),
    );
    let mut now = SETTLE_TICK_MS;
    while now < until {
        send(now, SerikoMsg::Tick { now_ms: now });
        now += SETTLE_TICK_MS;
    }
    send(until, SerikoMsg::Tick { now_ms: until });
    drop(send);
    states.current_pattern(&actor, Slot::Shell).clone()
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
