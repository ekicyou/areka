//! `check_script` の処理（spec: areka-P0-mcp-author-tools）——事実を写し取り、別のスレッドで判断して答える。
//!
//! 入口は `dump_surface` の [`start`] をそのまま使う（装着の相がまだ → [`super::later`] へ預けて、装着の
//! 後のフレームで答える・スレッドを起こせなければ `error!` を残して `NG:` で答える）。UI スレッドでするのは
//! 事実の写し取り（シェルとバルーンの面の ID の集合・scope 0 の別名の表）までで、量はシェルの大きさで
//! 決まり台本の長さに依らない。読む段・`\!` の表・判断・結果の組み立ては別のスレッドで行う。
//! World は読むだけで、SHIORI・再生・表示・ファイル・ネットへ進む呼び出しを持たない（要件 2.4）。

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use areka_emo_present::EmoPresenter;
use areka_mcp::tools::ReplyTo;
use areka_mcp::tools::check_script::{Args, NO_WINDOW_NOTE, render};
use areka_parsers::sakura::parse_noted;
use areka_seriko::{SurfaceResolver, SurfaceTarget};
use bevy_ecs::world::World;
use tracing::debug;

use super::check_script_judge::{ScriptFacts, diagnose};
use super::dump_surface::{Job, Step, start};
use super::resolve::ActiveGhost;
use crate::emo2_boot::consumer_ledger::ConsumerLedger;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::target_map::{balloon_target, shell_target};
use crate::placement::spawn::GhostWindows;

const TOOL: &str = "check_script";

/// 写しまで済んだら別のスレッドで判断して答え、装着の相がまだなら `later` に預けて答える（要件 2.5・3.1）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    // 台本は写さずに 1 度だけ仕事へ移す（UI スレッドの仕事を台本の長さに依らせない）。
    let mut script = args.script;
    start(world, TOOL, ghost, reply, move |w| answer(w, &mut script));
}

/// 今答えられるなら `Some`。装着の相がまだなら `None`（`later` が次のフレームでもう 1 度呼ぶ）。
/// `Some` を返すのは 1 度だけなので、そこで台本を取り出す。
fn answer(world: &World, script: &mut String) -> Option<Step> {
    // 表示の結線が無い＝窓の無いゴースト。surface とバルーンは診ない。
    let facts = match world.get_non_send::<Emo2Wiring>() {
        None => None,
        Some(wiring) if !wiring.attached() => return None,
        Some(wiring) => {
            let scopes = world
                .get_resource::<GhostWindows>()
                .into_iter()
                .flat_map(GhostWindows::scopes)
                .filter_map(|s| u32::try_from(s).ok());
            Some(ScriptFactsSnapshot::copy(wiring.presenter(), scopes))
        }
    };
    Some(Step::Encode(0, job(std::mem::take(script), facts)))
}

/// 別のスレッドでする仕事（読む段・`\!` の表・判断・結果の組み立て）。どれも World に触らない。
fn job(script: String, facts: Option<ScriptFactsSnapshot>) -> Job {
    Box::new(move |ui| {
        let started = Instant::now();
        let reads = parse_noted(&script);
        let ledger = ConsumerLedger::canonical();
        let facts_ref = facts.as_ref().map(|f| f as &dyn ScriptFacts);
        let list = diagnose(&script, &reads, &ledger, facts_ref);
        let unchecked = facts.is_none().then_some(NO_WINDOW_NOTE);
        debug!(
            tool = TOOL,
            diagnostics = list.len(),
            script_bytes = script.len(),
            ui_us = ui.as_micros() as u64,
            worker_us = started.elapsed().as_micros() as u64,
            "[mcp] 台本を確かめた"
        );
        render(&list, unchecked)
    })
}

/// UI スレッドで表示の層から写し取った事実（別のスレッドへ渡せる値）。
pub(in crate::mcp) struct ScriptFactsSnapshot {
    /// scope 0 のシェルの別名の表から作った解決器（再生の解決器と同じ作り方）。
    resolver: SurfaceResolver,
    /// スコープ → シェルの生の surface ID の集合（表示の層に登録の在るスコープだけ）。
    shells: BTreeMap<u32, BTreeSet<u32>>,
    /// スコープ → バルーンの面の ID の集合（同上）。
    balloons: BTreeMap<u32, BTreeSet<u32>>,
}

impl ScriptFactsSnapshot {
    /// 窓を持つスコープごとに、表示の層から写す。登録の無い target は入れない（その scope は診ない）。
    fn copy(presenter: &EmoPresenter, scopes: impl Iterator<Item = u32>) -> Self {
        let mut shells = BTreeMap::new();
        let mut balloons = BTreeMap::new();
        for scope in scopes {
            if let Some(ids) = presenter.surface_ids(shell_target(scope)) {
                shells.insert(scope, ids);
            }
            if let Some(ids) = presenter.surface_ids(balloon_target(scope)) {
                balloons.insert(scope, ids);
            }
        }
        let aliases = presenter
            .alias_snapshot(shell_target(0))
            .unwrap_or_default();
        Self::new(aliases, shells, balloons)
    }

    /// 写した値から組む（テストは手で組んだ値をここへ渡す）。
    pub(in crate::mcp) fn new(
        aliases: BTreeMap<String, Vec<u32>>,
        shells: BTreeMap<u32, BTreeSet<u32>>,
        balloons: BTreeMap<u32, BTreeSet<u32>>,
    ) -> Self {
        Self {
            resolver: SurfaceResolver::new(aliases),
            shells,
            balloons,
        }
    }
}

impl ScriptFacts for ScriptFactsSnapshot {
    fn resolve_surface(&self, key: &str) -> SurfaceTarget {
        self.resolver.resolve(key)
    }

    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool> {
        self.shells.get(&scope).map(|ids| ids.contains(&surface_id))
    }

    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool> {
        self.balloons
            .get(&scope)
            .map(|ids| ids.contains(&balloon_id))
    }
}

#[cfg(test)]
#[path = "check_script_tests.rs"]
mod check_script_tests;
