//! 読み直しの窓口（areka-P0-mcp-reload・design「ReloadDesk」・要件 1.1・1.6・2.1・2.2・7.4・7.5・9.1・9.2）。
//!
//! 台本（受け口 `reload_cue.rs` → 受信端 `ReloadRx` → 入力の段の取り出しの系 [`drain`]）と MCP の
//! `reload` の 2 つの入口は、ここの [`request_reload`] 1 本だけを通る。窓口は 終了の途中 → 別の読み直し
//! → ゴースト切替 → シェル／バルーン切替 → ネットワーク更新 → インストール → 文脈 の順に判定し、
//! 断りはどれも `warn!(reload_refused)` を 1 件残す（終了の途中だけは `debug!`・返事は落とす）。
//! 受理したら `info!(reload_started)` を残し、対象ごとの始め方へ渡す。

use std::sync::Arc;

use bevy_ecs::prelude::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use super::ghost_switch::SwitchInFlight;
use super::reload_args::{DescriptNotes, ReloadTarget, ScriptReload};
use super::reload_cue::ReloadRx;
use super::shell_balloon_switch::SkinSwitchInFlight;
use crate::boot_config::BootContext;
use crate::exit_wait::{self, WorkGate};
use crate::ghost_session::GhostSlot;
use crate::log_history::TARGET_ERROR;

/// 読み直しの出どころ（記録の欄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReloadOrigin {
    Script,
    #[allow(dead_code)] // 呼び手の MCP の `reload` は task 7.5 で付く。
    Mcp,
}

/// 窓口へ渡す読み直しの要求（`notes` は台本の `\![reload,descript,パラメータ]` の語の仕分けだけが持つ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReloadRequest {
    pub target: ReloadTarget,
    pub origin: ReloadOrigin,
    pub notes: DescriptNotes,
}

/// 窓口の判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReloadVerdict {
    Accepted,
    Refused(RefusedReason),
    /// 終了の途中（始めない・MCP は返事を落とす・要件 2.2）。
    Closing,
}

/// 断りの理由（`reload_refused` の `reason` 欄の語）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusedReason {
    ReloadInFlight,
    GhostSwitching,
    SkinSwitching,
    Updating,
    Installing,
    NoContext,
    NoWords,
}

impl RefusedReason {
    fn as_ref_str(self) -> &'static str {
        match self {
            RefusedReason::ReloadInFlight => "reload_in_flight",
            RefusedReason::GhostSwitching => "ghost_switching",
            RefusedReason::SkinSwitching => "skin_switching",
            RefusedReason::Updating => "updating",
            RefusedReason::Installing => "installing",
            RefusedReason::NoContext => "no_context",
            RefusedReason::NoWords => "no_words",
        }
    }
}

/// 進行中の印（World の NonSend・高々 1 つ）。在れば次の読み直しは断る。
///
/// 消すのは対象ごとの終わり（6.2〜6.4）と終了の片付け [`discard_for_exit`]。
#[derive(Debug)]
pub(crate) struct ReloadInFlight {
    pub target: ReloadTarget,
    pub origin: ReloadOrigin,
    /// 記録に添えるゴーストの名前（`get_active_ghost_list` と同じ綴り・`TARGET_ERROR` の `ghost` 欄）。
    pub ghost: String,
}

/// 終了の門（`register` が据える・無ければ終了の途中ではない）。
#[derive(Resource)]
struct ReloadGate(Arc<WorkGate>);

/// 取り出しの系を入力の段（`dispatch_pointer_events` の後）へ登録し、門 `reload` を終了の片付け
/// [`discard_for_exit`] と一緒に終了の待ちへ登記する（プロセスに 1 回・呼び手は
/// `ghost_session::register_systems`）。門は背景で書く仕事を持たない（片付けと `is_closing` のため）。
pub(crate) fn register(world: &mut World) {
    let gate = Arc::new(WorkGate::default());
    world.insert_resource(ReloadGate(gate.clone()));
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain.after(dispatch_pointer_events));
    exit_wait::register_gate(world, "reload", gate, discard_for_exit);
}

/// 溜まった台本の読み直しの要求を全件取り出し、1 件ごとに窓口へ「台本」の出どころで渡す（Input の系）。
///
/// 受信端が無ければ無操作（LogSink の起動・結線の前）。受信端の借用を切ってから窓口を呼ぶ。
pub(crate) fn drain(world: &mut World) {
    let Some(rx) = world.get_non_send::<ReloadRx>() else {
        tracing::trace!(
            event = "reload_drain_no_wiring",
            "台本の読み直しの要求の受信端が無い——取り出しは無操作"
        );
        return;
    };
    let pending: Vec<ScriptReload> = rx.try_iter().collect();
    for raw in pending {
        request_reload(
            world,
            ReloadRequest {
                target: raw.target,
                origin: ReloadOrigin::Script,
                notes: raw.notes,
            },
        );
    }
}

/// 唯一の入口。判定 → 語の記録 → 始めの記録 → 対象ごとの始め方。
pub(crate) fn request_reload(world: &mut World, req: ReloadRequest) -> ReloadVerdict {
    let ghost = ghost_name(world);
    if world
        .get_resource::<ReloadGate>()
        .is_some_and(|gate| gate.0.is_closing())
    {
        tracing::debug!(
            event = "reload_refused",
            reason = "closing",
            target = ?req.target,
            origin = ?req.origin,
            ghost = %ghost,
            "終了の途中なので読み直しを始めない"
        );
        return ReloadVerdict::Closing;
    }
    if let Some(reason) = overlap_or_context(world) {
        tracing::warn!(
            target: TARGET_ERROR,
            ghost = %ghost,
            event = "reload_refused",
            reason = reason.as_ref_str(),
            target = ?req.target,
            origin = ?req.origin,
            "読み直しを断った（ほかの切替・読み直し・更新・インストールの途中か、文脈が無い）"
        );
        return ReloadVerdict::Refused(reason);
    }
    // descript の語の仕分けの記録（要件 7.4・7.5）。
    for word in &req.notes.unsupported {
        tracing::warn!(
            target: TARGET_ERROR,
            ghost = %ghost,
            event = "reload_descript_unsupported",
            word = %word,
            "areka には無い種類の descript は読み直さずに飛ばす"
        );
    }
    for word in &req.notes.unknown {
        tracing::warn!(
            target: TARGET_ERROR,
            ghost = %ghost,
            event = "reload_descript_unknown",
            word = %word,
            "descript の読み直しの知らない語を飛ばす"
        );
    }
    if req.notes.empty {
        tracing::warn!(
            target: TARGET_ERROR,
            ghost = %ghost,
            event = "reload_descript_no_words",
            origin = ?req.origin,
            "descript の読み直しに読み直せる語が 1 つも無い——読み直さない"
        );
        return ReloadVerdict::Refused(RefusedReason::NoWords);
    }

    tracing::info!(
        event = "reload_started",
        target = ?req.target,
        origin = ?req.origin,
        ghost = %ghost,
        "読み直しを始める"
    );
    start(
        world,
        ReloadInFlight {
            target: req.target,
            origin: req.origin,
            ghost,
        },
    );
    ReloadVerdict::Accepted
}

/// 重なり（要件 2.1）と文脈の判定。判定の順はここの並びのとおり。
fn overlap_or_context(world: &World) -> Option<RefusedReason> {
    if world.get_non_send::<ReloadInFlight>().is_some() {
        return Some(RefusedReason::ReloadInFlight);
    }
    if world.get_non_send::<SwitchInFlight>().is_some() {
        return Some(RefusedReason::GhostSwitching);
    }
    if world.get_non_send::<SkinSwitchInFlight>().is_some() {
        return Some(RefusedReason::SkinSwitching);
    }
    if crate::update::desk::is_busy(world) {
        return Some(RefusedReason::Updating);
    }
    if crate::install::desk::is_busy(world) {
        return Some(RefusedReason::Installing);
    }
    let has_kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .is_some_and(|session| session.kanade().is_some());
    if !has_kanade || world.get_resource::<BootContext>().is_none() {
        return Some(RefusedReason::NoContext);
    }
    None
}

/// 対象ごとの始め方の継ぎ目（ghost＝6.2・shiori＝6.3・shell／balloon／descript＝6.4 が埋め、始めたら
/// 印 [`ReloadInFlight`] を置く）。まだ始め方の無い対象は、印を置かずに `reload_failed` を残して終える
/// （黙って受理だけしない・要件 9.3）。
fn start(_world: &mut World, flight: ReloadInFlight) {
    match flight.target {
        ReloadTarget::Ghost
        | ReloadTarget::Shiori
        | ReloadTarget::Shell
        | ReloadTarget::Balloon
        | ReloadTarget::Descript(_) => tracing::error!(
            target: TARGET_ERROR,
            ghost = %flight.ghost,
            event = "reload_failed",
            reason = "not_implemented",
            target = ?flight.target,
            origin = ?flight.origin,
            "この対象の読み直しはまだ始め方が無い——読み直さない"
        ),
    }
}

/// 終了が始まった（`exit_wait::begin_close` が呼ぶ）: 印があれば `info!(reload_dropped, reason=exit)` を
/// 残して下ろす。
pub(crate) fn discard_for_exit(world: &mut World) {
    if let Some(dropped) = world.remove_non_send::<ReloadInFlight>() {
        tracing::info!(
            event = "reload_dropped",
            reason = "exit",
            target = ?dropped.target,
            origin = ?dropped.origin,
            ghost = %dropped.ghost,
            "終了が始まったので読み直しを取りやめた"
        );
    }
}

/// 置き場のゴーストの名前（`get_active_ghost_list` と同じ綴り＝descript の `name`、無ければルート
/// フォルダの絶対パス・末尾の区切りなし）。置き場が空なら空文字列。
fn ghost_name(world: &World) -> String {
    let Some(session) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
    else {
        return String::new();
    };
    match session
        .names()
        .and_then(|n| n.name.clone())
        .filter(|n| !n.is_empty())
    {
        Some(name) => name,
        None => {
            let dir = session.ghost_dir();
            std::path::absolute(dir)
                .unwrap_or_else(|_| dir.to_path_buf())
                .display()
                .to_string()
                .trim_end_matches(['\\', '/'])
                .to_owned()
        }
    }
}

#[cfg(test)]
#[path = "reload_tests.rs"]
mod tests;
