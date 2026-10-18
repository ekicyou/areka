//! 読み直しの窓口（areka-P0-mcp-reload・design「ReloadDesk」・要件 1.1・1.6・2.1・2.2・7.4・7.5・9.1・9.2）。
//!
//! 台本（受け口 `reload_cue.rs` → 受信端 `ReloadRx` → 入力の段の取り出しの系 [`drain`]）と MCP の
//! `reload` の 2 つの入口は、ここの [`request_reload`] 1 本だけを通る。窓口は 終了の途中 → 別の読み直し
//! → ゴースト切替 → シェル／バルーン切替 → ネットワーク更新 → インストール → 文脈 の順に判定し、
//! 断りはどれも `warn!(reload_refused)` を 1 件残す（終了の途中だけは `debug!`・返事は落とす）。
//! 受理したら `info!(reload_started)` を残し、対象ごとの始め方へ渡す。

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_actor::{ReplyReceiver, reply_channel};
use areka_kanade::{
    BootOrigin, ChangeOrigin, GapRaise, KanadeMsg, ReloadScope, ShioriMethod, TalkGap,
};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::resolve_with_shell;
use bevy_ecs::prelude::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use super::ghost_switch::{
    GhostSpec, SwitchEnd, SwitchInFlight, SwitchRequest, SwitchTarget, SwitchVerdict,
    WelcomeAttempt, accept_switch,
};
use super::reload_args::{DescriptNotes, ReloadTarget, ScriptReload};
use super::reload_cue::ReloadRx;
use super::shell_balloon_resolve::SkinCandidate;
use super::shell_balloon_switch::{
    SkinKind, SkinSwitchInFlight, SkinVerdict, SwitchContext, accept_skin,
};
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
pub(crate) struct ReloadInFlight {
    pub target: ReloadTarget,
    pub origin: ReloadOrigin,
    /// 記録に添えるゴーストの名前（`get_active_ghost_list` と同じ綴り・`TARGET_ERROR` の `ghost` 欄）。
    pub ghost: String,
    pub stage: ReloadStage,
}

/// 印の段。
pub(crate) enum ReloadStage {
    /// shiori: 切れ目の印（`OnDestroy`）の返事を待っている（予約はまだ無い・下ろすのは [`drain`]）。
    AwaitingDestroy { gap: ReplyReceiver<TalkGap> },
    /// ghost／shiori: ゴースト切替の予約に乗っている（終わりは [`on_switch_end`] が告げる）。
    /// `reserved` は窓口自身が `accept_switch` で立てた予約の鍵（`SwitchTarget.dir`）。
    Switching { reserved: PathBuf },
    /// shell／balloon／descript: 列を進めている。`reserved` は進行中の段で窓口自身が [`accept_skin`] で
    /// 立てた予約の鍵（`SkinKind`＋`SkinCandidate.dir`）。`Names` の段と段の間は `None`。
    Skins {
        queue: VecDeque<SkinStep>,
        applied: bool,
        reserved: Option<(SkinKind, PathBuf)>,
    },
}

/// shell／balloon／descript の列の段（この順に直列に進める）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinStep {
    /// ゴーストの descript.txt の値（名前・readme の置き場）を替える（同期）。
    Names,
    Shell,
    Balloon,
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

/// 溜まった台本の読み直しの要求を全件取り出し、1 件ごとに窓口へ「台本」の出どころで渡してから、
/// shiori の切れ目の返事を覗く（[`poll_destroy`]・Input の系）。
///
/// 受信端が無ければ取り出しは無操作（LogSink の起動・結線の前）。受信端の借用を切ってから窓口を呼ぶ。
pub(crate) fn drain(world: &mut World) {
    let pending: Vec<ScriptReload> = match world.get_non_send::<ReloadRx>() {
        Some(rx) => rx.try_iter().collect(),
        None => {
            tracing::trace!(
                event = "reload_drain_no_wiring",
                "台本の読み直しの要求の受信端が無い——取り出しは無操作"
            );
            Vec::new()
        }
    };
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
    poll_destroy(world);
}

/// 印が `AwaitingDestroy` なら切れ目の返事を覗く（要件 2.4・4.1）。まだなら何もしない。`Reached` なら
/// 印を下ろして [`start_switch`] へ（予約が立てば `Switching` の印に替わる）。それ以外は始めずに
/// 印を下ろし、`reload_failed(gap_*)` を 1 件残す（利用者の取りやめ・ほかの切替は `warn!`、許可表の
/// 漏れと kanade の脱落は `error!`・要件 9.1・9.3）。
fn poll_destroy(world: &mut World) {
    let Some(reply) = world
        .get_non_send::<ReloadInFlight>()
        .and_then(|f| match &f.stage {
            ReloadStage::AwaitingDestroy { gap } => Some(gap.try_recv()),
            _ => None,
        })
    else {
        return;
    };
    if matches!(reply, Ok(None)) {
        return;
    }
    let Some(flight) = world.remove_non_send::<ReloadInFlight>() else {
        return;
    };
    let ReloadInFlight {
        target,
        origin,
        ghost,
        ..
    } = flight;
    let (reason, left) = match reply {
        Ok(Some(TalkGap::Reached { .. })) => {
            start_switch(world, target, origin, ghost);
            return;
        }
        Ok(Some(TalkGap::CancelledByUser)) => ("gap_cancelled", None),
        Ok(Some(TalkGap::Left { reason })) => ("gap_left", Some(reason)),
        Ok(Some(TalkGap::NotSent { .. })) => {
            return failed(&ghost, &target, origin, "gap_not_sent");
        }
        Ok(None) | Err(_) => return failed(&ghost, &target, origin, "kanade_gone"),
    };
    tracing::warn!(
        target: TARGET_ERROR,
        ghost = %ghost,
        event = "reload_failed",
        reason = reason,
        left = ?left,
        target = ?target,
        origin = ?origin,
        "台詞の切れ目に達しなかった（利用者の取りやめ・ほかの切替）ので shiori を読み直さない"
    );
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
    start(world, req.target, req.origin, ghost);
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

/// 対象ごとの始め方の継ぎ目（始めたら印 [`ReloadInFlight`] を置く）。
fn start(world: &mut World, target: ReloadTarget, origin: ReloadOrigin, ghost: String) {
    match &target {
        ReloadTarget::Ghost => start_switch(world, target, origin, ghost),
        ReloadTarget::Shiori => start_shiori(world, target, origin, ghost),
        ReloadTarget::Shell | ReloadTarget::Balloon | ReloadTarget::Descript(_) => {
            let queue = skin_steps(&target);
            world.insert_non_send(ReloadInFlight {
                target,
                origin,
                ghost,
                stage: ReloadStage::Skins {
                    queue,
                    applied: false,
                    reserved: None,
                },
            });
            advance_skins(world);
        }
    }
}

/// ghost の読み直し（要件 3.1）: 目録を引かず今のゴースト（`BootContext.current.ghost`）から切替先を組み、
/// `raise_event: false` で [`accept_switch`] を呼ぶ。受理されたら立った予約の鍵を印に控えて置く。
/// 受理されなければ印を置かずに `reload_failed(not_accepted)`（`accept_switch` が理由を記録済み）。
fn start_switch(world: &mut World, target: ReloadTarget, origin: ReloadOrigin, ghost: String) {
    let Some(current) = world
        .get_resource::<BootContext>()
        .map(|ctx| ctx.current.ghost.clone())
    else {
        failed(&ghost, &target, origin, "no_context");
        return;
    };
    // 起動の引数で直に指したゴースト（`folder` が無い）はフォルダの末尾を名乗る。
    let folder = current.folder.clone().unwrap_or_else(|| {
        current
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let to = SwitchTarget {
        dir: current.dir.clone(),
        name: crate::boot_resolve::running_name(&current),
        folder: folder.clone(),
        sakura_name: None,
    };
    let req = SwitchRequest {
        ghost: GhostSpec::Folder(folder),
        raise_event: false,
        origin: ChangeOrigin::Automatic,
        boot_event: None,
    };
    match accept_switch(world, to, req) {
        SwitchVerdict::Accepted => {
            let reserved = world
                .get_non_send::<SwitchInFlight>()
                .map(|f| f.target.dir.clone())
                .unwrap_or(current.dir);
            world.insert_non_send(ReloadInFlight {
                target,
                origin,
                ghost,
                stage: ReloadStage::Switching { reserved },
            });
        }
        verdict => {
            tracing::debug!(event = "reload_switch_verdict", verdict = ?verdict, "切替の受理の判定");
            failed(&ghost, &target, origin, "not_accepted");
        }
    }
}

/// shiori の読み直し（要件 2.4・4.1）: 台詞の切れ目の待ちに `OnDestroy`（Reference0＝`reload`・NOTIFY）
/// の印を載せて kanade へ送り、返事の受け手を `AwaitingDestroy` の印に置く（返事は [`drain`] が覗く）。
/// 送れなければ印を置かずに `reload_failed(kanade_gone)`。
fn start_shiori(world: &mut World, target: ReloadTarget, origin: ReloadOrigin, ghost: String) {
    let (reply, gap) = reply_channel();
    let raise = Some(GapRaise {
        id: "OnDestroy".to_owned(),
        references: vec!["reload".to_owned()],
        method: ShioriMethod::Notify,
    });
    let sent = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade())
        .is_some_and(|k| k.send(KanadeMsg::AwaitTalkGap { raise, reply }).is_ok());
    if !sent {
        failed(&ghost, &target, origin, "kanade_gone");
        return;
    }
    world.insert_non_send(ReloadInFlight {
        target,
        origin,
        ghost,
        stage: ReloadStage::AwaitingDestroy { gap },
    });
}

/// 失敗の終わりの記録（要件 9.2・`TARGET_ERROR`＋`ghost`）。
fn failed(ghost: &str, target: &ReloadTarget, origin: ReloadOrigin, reason: &str) {
    tracing::error!(
        target: TARGET_ERROR,
        ghost = %ghost,
        event = "reload_failed",
        reason = reason,
        target = ?target,
        origin = ?origin,
        "読み直しが終わらなかった"
    );
}

/// 進行中の予約が窓口自身の予約か（印が `Switching { reserved }` で `reserved == SwitchInFlight.target.dir`）。
fn own_switch(world: &World) -> Option<&ReloadInFlight> {
    let flight = world.get_non_send::<ReloadInFlight>()?;
    let ReloadStage::Switching { reserved } = &flight.stage else {
        return None;
    };
    let switch = world.get_non_send::<SwitchInFlight>()?;
    (switch.target.dir == *reserved).then_some(flight)
}

/// ゴースト切替の予約が下りる全部の出口から（呼び手 `ghost_switch::end_switch`・**予約を下ろす前**に呼ぶ）。
/// 印が `Switching` なら終わりの記録を 1 件残して印を下ろす（要件 3.7・8.4）。予約が窓口自身の
/// ものでなければ `reload_failed(overtaken)`。印が無い・別の段なら何もしない。
pub(crate) fn on_switch_end(world: &mut World, end: SwitchEnd) {
    if !matches!(
        world.get_non_send::<ReloadInFlight>().map(|f| &f.stage),
        Some(ReloadStage::Switching { .. })
    ) {
        return;
    }
    let own = own_switch(world).is_some();
    let Some(flight) = world.remove_non_send::<ReloadInFlight>() else {
        return;
    };
    let reason = match end {
        _ if !own => "overtaken",
        SwitchEnd::Steady {
            attempt: WelcomeAttempt::Target,
        } => {
            tracing::info!(
                event = "reload_done",
                target = ?flight.target,
                origin = ?flight.origin,
                ghost = %flight.ghost,
                "読み直しが終わった"
            );
            return;
        }
        SwitchEnd::Steady {
            attempt: WelcomeAttempt::Default,
        } => "fell_back_to_default",
        SwitchEnd::Cancelled => "cancelled",
        SwitchEnd::NotAccepted => "not_accepted",
        SwitchEnd::TargetQuit => "target_quit",
        SwitchEnd::DefaultFault | SwitchEnd::Fatal | SwitchEnd::WriteFailure => "fatal",
    };
    failed(&flight.ghost, &flight.target, flight.origin, reason);
}

/// `switch_to` が由来を決めるとき（要件 3.2・3.3）: 印が ghost／shiori の読み直しで、進行中の予約が
/// 窓口自身のものなら `Some(Reloaded{scope})`。他人の予約なら `None`（既存の由来のまま）。
pub(crate) fn boot_origin_override(world: &World) -> Option<BootOrigin> {
    let scope = match own_switch(world)?.target {
        ReloadTarget::Ghost => ReloadScope::Ghost,
        ReloadTarget::Shiori => ReloadScope::Shiori,
        _ => return None,
    };
    Some(BootOrigin::Reloaded { scope })
}

/// `switch_to` の分かれ目: 印が shiori の読み直しで、進行中の予約が窓口自身のものなら真（窓を閉じずに
/// 実行系だけ組み直す）。
pub(crate) fn wants_runtime_only(world: &World) -> bool {
    own_switch(world).is_some_and(|f| f.target == ReloadTarget::Shiori)
}

/// 対象から段の列を組む（Flow 4・要件 5.1・6.1・7.1・7.2）: `shell`（`reloadsurface` も）→ `[Shell]`、
/// `balloon` → `[Balloon]`、`descript` → 語の集合から（`ghost` は `Names`＋`Shell`＝ゴーストの descript の
/// 配置の値はシェルの差し替えの後始末が入れ直す・重なった段は 1 度）。
fn skin_steps(target: &ReloadTarget) -> VecDeque<SkinStep> {
    let (names, shell, balloon) = match target {
        ReloadTarget::Shell => (false, true, false),
        ReloadTarget::Balloon => (false, false, true),
        ReloadTarget::Descript(parts) => (parts.ghost, parts.ghost || parts.shell, parts.balloon),
        ReloadTarget::Ghost | ReloadTarget::Shiori => (false, false, false),
    };
    [
        (names, SkinStep::Names),
        (shell, SkinStep::Shell),
        (balloon, SkinStep::Balloon),
    ]
    .into_iter()
    .filter_map(|(on, step)| on.then_some(step))
    .collect()
}

/// 列を進める: 先頭の段を取り、`Names` は同期で済ませて次へ、`Shell`／`Balloon` は今のフォルダから
/// 組んだ切替先で `accept_skin(raise: None)` を呼んで予約の鍵を印に控えて戻る（終わりは差し替えの相の
/// [`skin_phase_ended`] が告げる）。列が空なら `reload_done` を残して印を下ろす（要件 8.4）。始められ
/// なければ `reload_failed` を残して印を下ろす（要件 9.3）。
fn advance_skins(world: &mut World) {
    loop {
        let Some(step) = world
            .get_non_send_mut::<ReloadInFlight>()
            .and_then(|mut f| match &mut f.stage {
                ReloadStage::Skins { queue, .. } => Some(queue.pop_front()),
                _ => None,
            })
        else {
            return;
        };
        let kind = match step {
            None => {
                if let Some(done) = world.remove_non_send::<ReloadInFlight>() {
                    tracing::info!(
                        event = "reload_done",
                        target = ?done.target,
                        origin = ?done.origin,
                        ghost = %done.ghost,
                        "読み直しが終わった"
                    );
                }
                return;
            }
            Some(SkinStep::Names) => {
                if !reload_names(world) {
                    return end_skins(world, "names_failed");
                }
                continue;
            }
            Some(SkinStep::Shell) => SkinKind::Shell,
            Some(SkinStep::Balloon) => SkinKind::Balloon,
        };
        // 段の間に通った他人の切替の予約を上書きしない（印は他人の切替に追い越された）。
        if world.get_non_send::<SkinSwitchInFlight>().is_some()
            || world.get_non_send::<SwitchInFlight>().is_some()
        {
            return end_skins(world, "overtaken");
        }
        let Some(ctx) = SwitchContext::read(world) else {
            return end_skins(world, "no_context");
        };
        let Some(to) = current_skin(world, &ctx, kind) else {
            return end_skins(world, "no_context");
        };
        let dir = to.dir.clone();
        if accept_skin(world, &ctx, kind, to, None) != SkinVerdict::Accepted {
            return end_skins(world, "kanade_gone");
        }
        if let Some(mut flight) = world.get_non_send_mut::<ReloadInFlight>()
            && let ReloadStage::Skins {
                applied, reserved, ..
            } = &mut flight.stage
        {
            *applied = false;
            *reserved = Some((kind, dir));
        }
        return;
    }
}

/// 今のシェル（実行系のマウントの `shell.dir`）／今のバルーン（`BootContext.current.balloon.dir`）を
/// 目録を引かずに切替先として組む（要件 5.1・6.1）。実行系が無くシェルが分からなければ `None`。
fn current_skin(world: &World, ctx: &SwitchContext, kind: SkinKind) -> Option<SkinCandidate> {
    let (dir, folder) = match kind {
        SkinKind::Shell => {
            let dir = world
                .get_non_send::<GhostSlot>()
                .and_then(|slot| slot.0.as_ref())
                .and_then(|session| session.runtime())
                .map(|runtime| runtime.mount().shell.dir.clone())?;
            (dir, ctx.current_shell.clone()?)
        }
        SkinKind::Balloon => {
            let folder = ctx.current_balloon.clone().unwrap_or_else(|| {
                ctx.balloon_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            (ctx.balloon_dir.clone(), folder)
        }
    };
    Some(SkinCandidate {
        dir,
        folder,
        name: None,
        hidden: false,
    })
}

/// `Names` の段（要件 7.3・7.6・design「descript の読み直しで替える値」）: ゴーストの descript.txt を
/// 今のシェルの名前つきで読み直し、名前（`set_names`＝sylphya の静的構成も）と readme の置き場を替える。
/// 読めなければ今の値を保って `reload_descript_read_failed` を残し、真を返して列を続ける。実行系が無く
/// 名前を替えられなければ偽。sylphya への投函の barrier は待たない（読み手は次に問い合わせるとき見る）。
fn reload_names(world: &mut World) -> bool {
    let ghost = world
        .get_non_send::<ReloadInFlight>()
        .map(|f| f.ghost.clone())
        .unwrap_or_default();
    let Some((root, shell)) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .map(|s| (s.ghost_dir().to_path_buf(), s.current_shell_folder()))
    else {
        return false;
    };
    let model = match resolve_with_shell(&root, DefaultEncoding::Ansi, shell.as_deref()) {
        Ok(model) => model,
        Err(err) => {
            tracing::error!(
                target: TARGET_ERROR,
                ghost = %ghost,
                event = "reload_descript_read_failed",
                file = %root.join("ghost").join("master").join("descript.txt").display(),
                reason = ?err,
                "ゴーストの descript.txt を読めなかった——今の名前と readme の置き場を保つ"
            );
            return true;
        }
    };
    let replaced = world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.as_mut().map(|s| s.set_names(model.names)))
        .unwrap_or(false);
    if replaced {
        crate::readme::set_path(
            world,
            crate::readme::resolve_path(&root, model.readme.as_deref()),
        );
    }
    replaced
}

/// 列の途中の失敗の終わり（印を下ろして `reload_failed` を 1 件）。
fn end_skins(world: &mut World, reason: &str) {
    if let Some(flight) = world.remove_non_send::<ReloadInFlight>() {
        failed(&flight.ghost, &flight.target, flight.origin, reason);
    }
}

/// 差し替えの相の分かれ目（呼び手 `frame/switch.rs` の `finish_shell`／`finish_balloon`）: 印が `Skins` で、
/// 終えようとしている相の `kind`・`dir`（`SkinSwitchInFlight.target.dir`）が窓口の控えた予約の鍵と一致
/// するときだけ真。印があっても他人の予約（利用者の `\![change,shell,…]` など）なら偽。相は印を
/// World から外して進むので、鍵は呼び手が渡す。
pub(crate) fn skin_is_reload(world: &World, kind: SkinKind, dir: &Path) -> bool {
    world.get_non_send::<ReloadInFlight>().is_some_and(|f| {
        matches!(&f.stage, ReloadStage::Skins { reserved: Some((k, d)), .. } if *k == kind && d == dir)
    })
}

/// 読み直しの差し替えが済んだ（[`skin_is_reload`] が真のときの `finish_*` から）。
pub(crate) fn skin_applied(world: &mut World) {
    if let Some(mut flight) = world.get_non_send_mut::<ReloadInFlight>()
        && let ReloadStage::Skins { applied, .. } = &mut flight.stage
    {
        *applied = true;
    }
}

/// 差し替えの相が終わった（`run_switch_phase` が印を残さなかったとき・鍵は終えた相の `kind`・`dir`）。
/// 印が `Skins` でなければ何もしない。窓口自身の予約で `applied` なら次の段へ、`applied` 無しなら
/// `reload_failed(skin_failed)`（理由は相の既存の記録が残している）、他人の予約なら
/// `reload_failed(overtaken)` で印を下ろす。
pub(crate) fn skin_phase_ended(world: &mut World, kind: SkinKind, dir: &Path) {
    let Some(applied) = world
        .get_non_send::<ReloadInFlight>()
        .and_then(|f| match &f.stage {
            ReloadStage::Skins { applied, .. } => Some(*applied),
            _ => None,
        })
    else {
        return;
    };
    match (skin_is_reload(world, kind, dir), applied) {
        (false, _) => end_skins(world, "overtaken"),
        (true, false) => end_skins(world, "skin_failed"),
        (true, true) => {
            if let Some(mut flight) = world.get_non_send_mut::<ReloadInFlight>()
                && let ReloadStage::Skins {
                    applied, reserved, ..
                } = &mut flight.stage
            {
                *applied = false;
                *reserved = None;
            }
            advance_skins(world);
        }
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

#[cfg(test)]
#[path = "reload_skin_tests.rs"]
mod skin_tests;
