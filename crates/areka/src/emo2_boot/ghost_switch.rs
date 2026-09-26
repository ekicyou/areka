//! ゴーストの切替の入口（areka-P0-ghost-shell-balloon-switch・design「GhostSwitch」）。
//!
//! 切替要求は台本の `\![change,ghost,…]` とメニューの「ゴースト」枠の 2 つの出どころから、
//! ここの [`request_ghost_switch`] 1 本だけを通る（要件 1.1）。入口は降ろす前・`OnGhostChanging`
//! を送る前に目録と突き合わせ（要件 1.5〜1.7）、受理したら切替の予約 [`SwitchInFlight`] を
//! 立てて kanade へ切替の要求を送る。台本からの要求は受け口 `ChangeCueSink` から受信端
//! [`ChangeRx`] を経て、入力の段の取り出しの系 [`drain_change_requests`] が入口へ渡す。
//! 送り出しの握手が済んだら [`switch_to`] が降ろして切替先を起こし、失敗なら
//! [`switch_to_default`] が既定ゴーストへ 1 回だけ戻す（それも失敗なら致命で終了）。

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use areka_ghost::GhostEntry;
use areka_kanade::{
    BootOrigin, ChangeHandoff, ChangeOrigin, ChangeRequest, ChangeTarget, ChangedFrom, CloseReason,
    KanadeMsg, ShioriFault, ShioriFaultKind,
};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use crate::app_exit::{ExitOrigin, WindowsClosed, close_windows_for_restart, quit_app};
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost, resolve_balloon_for_ghost};
use crate::boot_resolve::{DEFAULT_GHOST_FOLDER, GhostDecision, GhostRoute, NoBalloon, pick_index};
use crate::ghost_session::{
    GhostBootInputsSource, GhostSlot, boot_ghost_strict, commit_ghost_windows, reopen_ghost_windows,
};

/// 台本の `\![change,ghost,名(,--option=raise-event)]` から届く切替要求（受け口
/// `ChangeCueSink` が talk スレッドから送る）。名前は台本の字面のまま（無変形）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeRequestRaw {
    pub name: String,
    pub raise_event: bool,
}

/// 台本からの切替要求の受信端（World の NonSend・ゴーストごとに新品）。
pub(crate) struct ChangeRx(Receiver<ChangeRequestRaw>);

/// 受信端を World へ据える（ゴーストごと・系は登録しない＝登録は [`register_change_drain`]）。
///
/// 呼び手は結線の成立後の `wire_emo2_boot`。前のゴーストの受信端は置き換わって落ちる。
pub(crate) fn wire_change_rx(world: &mut World, rx: Receiver<ChangeRequestRaw>) {
    world.insert_non_send(ChangeRx(rx));
}

/// 取り出しの系を入力の段へ登録する（プロセスに 1 回・持ち物は置かない）。
///
/// 並びは説明書の取り出しと同じく `dispatch_pointer_events` の後。呼び手は
/// `ghost_session::register_systems`。
pub(crate) fn register_change_drain(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain_change_requests.after(dispatch_pointer_events));
}

/// 溜まった台本の切替要求を全件取り出し、1 件ごとに入口へ「自動」の出どころで渡す（Input の系）。
///
/// 受信端が無ければ無操作（LogSink の起動・結線の前）。受け口の借用を切ってから入口を呼ぶ。
pub(crate) fn drain_change_requests(world: &mut World) {
    let Some(rx) = world.get_non_send::<ChangeRx>() else {
        tracing::trace!(
            event = "change_drain_no_wiring",
            "台本の切替要求の受信端が無い——取り出しは無操作"
        );
        return;
    };
    let pending: Vec<ChangeRequestRaw> = rx.0.try_iter().collect();
    for raw in pending {
        request_ghost_switch(
            world,
            SwitchRequest {
                ghost: GhostSpec::Name(raw.name),
                raise_event: raw.raise_event,
                origin: ChangeOrigin::Automatic,
            },
        );
    }
}

/// 切替要求（要件 1.1 の入口の型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwitchRequest {
    pub ghost: GhostSpec,
    /// 真なら送り出す側へ `OnGhostChanging` を送る。
    pub raise_event: bool,
    pub origin: ChangeOrigin,
}

/// 切替先の指し方。台本は名前（`descript.txt` の `name` → フォルダ名の順）、メニューはフォルダ名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GhostSpec {
    Name(String),
    Folder(String),
}

/// 入口の判定（記録の語彙・呼び手は分岐しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwitchVerdict {
    Accepted,
    Busy,
    NotFound,
    NoContext,
}

/// 切替先（`OnGhostChanging` の Ref0〜3 の材料）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwitchTarget {
    pub dir: PathBuf,
    pub folder: String,
    /// `descript.txt` の `name`（無ければフォルダ名）。
    pub name: String,
    /// `descript.txt` の `sakura.name`（切替先だけ読む・要件 8.7）。
    pub sakura_name: Option<String>,
}

/// 直前のゴースト（`OnGhostChanged` の Ref0・2・3 の材料）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrevGhost {
    pub dir: PathBuf,
    pub name: Option<String>,
    pub sakura_name: Option<String>,
}

/// 切替の段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwitchStage {
    /// kanade へ切替の要求を送り、送り出しの握手の終わり（停止通知）を待っている。
    SendOff,
    /// 起こしたゴーストの定常到達を待っている（起こしたのが切替先か既定か）。
    Welcoming { attempt: WelcomeAttempt },
}

/// 迎え入れているゴースト（既定へ戻す試みは 1 回だけ＝`Default` からは戻らない・要件 6.5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WelcomeAttempt {
    Target,
    Default,
}

/// 切替の予約（World の NonSend・在れば切替中）。二重要求の判定はこの有無で行う（要件 1.9）。
#[derive(Debug)]
pub(crate) struct SwitchInFlight {
    pub target: SwitchTarget,
    pub prev: PrevGhost,
    pub stage: SwitchStage,
}

/// 純粋: 目録の項目と指し方から切替先を決める（要件 1.5・1.8・11.8）。
///
/// 名前は `descript.txt` の `name` と先に突き合わせ、一致が無ければフォルダ名と突き合わせる。
/// フォルダ名の指し方はフォルダ名とだけ突き合わせる。比較は大文字小文字を区別し、今のゴーストも
/// 除外しない。`sakura_name` は fs を読むので入口が埋める（ここでは `None`）。
pub(crate) fn resolve_switch_target(
    entries: &[GhostEntry],
    spec: &GhostSpec,
) -> Option<SwitchTarget> {
    let by_folder = |want: &str| entries.iter().find(|e| e.identity.folder == want);
    let hit = match spec {
        GhostSpec::Name(want) => entries
            .iter()
            .find(|e| e.identity.name.as_deref() == Some(want.as_str()))
            .or_else(|| by_folder(want)),
        GhostSpec::Folder(want) => by_folder(want),
    }?;
    Some(SwitchTarget {
        dir: hit.dir.clone(),
        folder: hit.identity.folder.clone(),
        name: hit
            .identity
            .name
            .clone()
            .unwrap_or_else(|| hit.identity.folder.clone()),
        sakura_name: None,
    })
}

/// 切替要求の唯一の入口（要件 1.1・1.6・1.7・1.9）。
///
/// ⑴ 予約が在れば `warn!(ghost_switch_busy)`。⑵ 起動の文脈の根で目録を読み突き合わせ、該当なしは
/// `warn!(ghost_switch_unknown)`（降ろさず kanade へ何も送らない）。⑶ 置き場のゴーストの送出端が
/// 無ければ `warn!(ghost_switch_no_context)`。⑷ 切替先の `sakura.name` だけを読み（要件 8.7）、
/// kanade へ切替の要求を送って予約を立てる。送出に失敗したら `error!(ghost_switch_send_failed)` で
/// 予約は立てない（送ってから立てるので、下ろすべき予約は残らない）。
pub(crate) fn request_ghost_switch(world: &mut World, req: SwitchRequest) -> SwitchVerdict {
    if world.get_non_send::<SwitchInFlight>().is_some() {
        tracing::warn!(
            event = "ghost_switch_busy",
            spec = ?req.ghost,
            "切替の途中に新しい切替要求が届いた——重ねずに無視する"
        );
        return SwitchVerdict::Busy;
    }
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::warn!(
            event = "ghost_switch_no_context",
            reason = "boot_context",
            "起動の文脈が無いので切替要求を無視する"
        );
        return SwitchVerdict::NoContext;
    };
    let entries = areka_ghost::catalog::list_ghosts(&ctx.root);
    let Some(mut target) = resolve_switch_target(&entries, &req.ghost) else {
        tracing::warn!(
            event = "ghost_switch_unknown",
            spec = ?req.ghost,
            "切替先が目録のどのゴーストにも一致しない——切替を無視する（降ろさず知らせも送らない）"
        );
        return SwitchVerdict::NotFound;
    };
    let current_folder = ctx.current.ghost.folder.clone();

    let session = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref());
    let Some((kanade, prev)) = session.and_then(|s| {
        let names = s.names();
        let prev = PrevGhost {
            dir: s.ghost_dir().to_path_buf(),
            name: names.and_then(|n| n.name.clone()).or(current_folder),
            sakura_name: names.and_then(|n| n.sakura_name.clone()),
        };
        s.kanade().map(|k| (k.clone(), prev))
    }) else {
        tracing::warn!(
            event = "ghost_switch_no_context",
            reason = "ghost_slot",
            "切替の要求を送る先のゴーストが居ないので切替要求を無視する"
        );
        return SwitchVerdict::NoContext;
    };

    target.sakura_name = areka_ghost::catalog::sakura_name(&target.dir);
    let change = ChangeRequest {
        target: ChangeTarget {
            sakura_name: target.sakura_name.clone().unwrap_or_default(),
            name: target.name.clone(),
            dir: std::path::absolute(&target.dir)
                .unwrap_or_else(|_| target.dir.clone())
                .display()
                .to_string(),
        },
        origin: req.origin,
        raise_event: req.raise_event,
    };
    if kanade.send(KanadeMsg::ChangeGhost(change)).is_err() {
        tracing::error!(
            event = "ghost_switch_send_failed",
            to = %target.name,
            "kanade へ切替の要求を送れなかった（kanade は止まっている）——切替を行わない"
        );
        return SwitchVerdict::NoContext;
    }
    tracing::info!(
        event = "ghost_switch_requested",
        from = ?prev.name,
        to = %target.name,
        raise_event = req.raise_event,
        origin = req.origin.as_ref_str(),
        "切替の要求を kanade へ送った"
    );
    world.insert_non_send(SwitchInFlight {
        target,
        prev,
        stage: SwitchStage::SendOff,
    });
    SwitchVerdict::Accepted
}

/// 切替先を起こす（要件 2.7・3.1・3.2・3.7・3.8・4.6・4.7・6.1・design Flow 1）。
///
/// 予約の切替先へ: 置き場のゴーストを同期で降ろす → 全窓を閉じる → [`boot_into`]（由来＝切替で
/// 来た・経路＝切替）。成功なら段を「迎え入れ（切替先）」に。同期の失敗は、切替先が既定なら致命、
/// そうでなければ [`switch_to_default`]（窓は投函していないので壊れた切替先の窓は生えない）。
// 本番の呼び手は 8.3 の停止通知の振り分け（送り出しの段で切替の中身あり）。それまでは test からだけ呼ぶ。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn switch_to(world: &mut World, handoff: ChangeHandoff) {
    let Some((target, prev)) = world
        .get_non_send::<SwitchInFlight>()
        .map(|f| (f.target.clone(), f.prev.clone()))
    else {
        tracing::error!(
            event = "ghost_switch_no_reservation",
            "切替の予約が無いのに切替先を起こす指示が来た——何もしない"
        );
        return;
    };
    take_down(world);
    let closed = close_windows_for_restart(world);
    let origin = BootOrigin::ChangedFrom(ChangedFrom {
        sakura_name: prev.sakura_name.unwrap_or_default(),
        script: handoff.script.unwrap_or_default(),
        name: prev.name.unwrap_or_default(),
        dir: std::path::absolute(&prev.dir)
            .unwrap_or(prev.dir)
            .display()
            .to_string(),
    });
    let ghost = GhostDecision {
        route: GhostRoute::Switched,
        dir: target.dir.clone(),
        folder: Some(target.folder.clone()),
    };
    if boot_into(world, closed, ghost, origin) {
        set_stage(world, WelcomeAttempt::Target);
    } else if target.folder == DEFAULT_GHOST_FOLDER {
        fatal(
            world,
            format!("切替先の既定ゴースト {} を起こせなかった", target.name),
        );
    } else {
        switch_to_default(world, &target.name);
    }
}

/// 既定ゴーストへ戻す（要件 6.1〜6.5・design Flow 3）。試みは 1 回だけで、失敗は致命。告知は出さない。
///
/// 既定が目録に無ければ致命。在れば置き場のゴースト（切替先）が残っていれば降ろして全窓を閉じ、
/// [`boot_into`]（由来＝前回落ちた・経路＝既定）。成功なら段を「迎え入れ（既定）」に。
pub(crate) fn switch_to_default(world: &mut World, fallen_name: &str) {
    let default_dir = world.get_resource::<BootContext>().and_then(|ctx| {
        areka_ghost::catalog::list_ghosts(&ctx.root)
            .into_iter()
            .find(|e| e.identity.folder == DEFAULT_GHOST_FOLDER)
            .map(|e| e.dir)
    });
    let Some(dir) = default_dir else {
        fatal(
            world,
            format!(
                "既定ゴースト {DEFAULT_GHOST_FOLDER} が目録に無い（落ちたゴースト: {fallen_name}）"
            ),
        );
        return;
    };
    take_down(world);
    let closed = close_windows_for_restart(world);
    let ghost = GhostDecision {
        route: GhostRoute::Default,
        dir,
        folder: Some(DEFAULT_GHOST_FOLDER.to_owned()),
    };
    let origin = BootOrigin::Halted {
        ghost_name: fallen_name.to_owned(),
    };
    if boot_into(world, closed, ghost, origin) {
        set_stage(world, WelcomeAttempt::Default);
    } else {
        fatal(
            world,
            format!(
                "既定ゴースト {DEFAULT_GHOST_FOLDER} を起こせなかった（落ちたゴースト: {fallen_name}）"
            ),
        );
    }
}

/// 置き場のゴーストを同期で降ろす（SHIORI の解放を待つ・要件 2.7）。所要 ms を残す（要件 3.8）。
/// 置き場が空なら何もしない。降ろす失敗は `shutdown` が `error!` 済みで、戻す先が無いので続ける。
fn take_down(world: &mut World) {
    let Some(session) = world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.take())
    else {
        tracing::debug!(
            event = "ghost_switch_nothing_to_down",
            "置き場が空——降ろすゴーストは無い"
        );
        return;
    };
    let started = Instant::now();
    let result = session.shutdown(CloseReason::System);
    let ms = started.elapsed().as_millis() as u64;
    if result.is_err() {
        tracing::error!(
            event = "ghost_switch_down_failed",
            ms,
            "ゴーストを降ろすのに失敗した——戻す先が無いので切替を続ける"
        );
    }
    tracing::info!(event = "ghost_switch_down_ms", ms, "ゴーストを降ろした");
}

/// 1 体を起こす共通の手順（同期）: 起動入力の作り口の有無 → バルーンの解決 → 窓の準備 → 厳格な起動。
/// 成功したときだけ窓を投函し、置き場へ入れ、起動の文脈の今のゴーストを更新して `true`。
/// 失敗は `error!(ghost_switch_boot_failed, stage)` の上で `false`（窓は投函していない）。
fn boot_into(
    world: &mut World,
    closed: WindowsClosed,
    ghost: GhostDecision,
    origin: BootOrigin,
) -> bool {
    let name = ghost.folder.clone().unwrap_or_default();
    let failed = |stage: &str, detail: String| {
        tracing::error!(
            event = "ghost_switch_boot_failed",
            ghost = %name,
            stage,
            detail = %detail,
            "ゴーストを起こせなかった（窓は投函していない）"
        );
        false
    };
    // 作り口と文脈は同期の I/O の前に確かめる（無ければ何を解いても起こせない）。
    let context = world.get_resource::<BootContext>().map(|c| c.root.clone());
    let has_source = world.contains_non_send::<GhostBootInputsSource>();
    let (Some(root), true) = (context, has_source) else {
        let reason = if has_source {
            "boot_context"
        } else {
            "inputs_source"
        };
        tracing::error!(
            event = "ghost_switch_no_context",
            reason,
            "起動の文脈か起動入力の作り口が無いのでゴーストを起こせない"
        );
        return failed("no_context", reason.to_owned());
    };
    let balloon = match resolve_balloon_for_ghost(&root, &ghost.dir, pick_index) {
        Ok(balloon) => balloon,
        Err(NoBalloon { balloon_store }) => {
            return failed("balloon", balloon_store.display().to_string());
        }
    };
    let cfg = ConfigInputs {
        ghost_root: ghost.dir.clone(),
        balloon_root: balloon.dir.clone(),
    };
    let prepared = match reopen_ghost_windows(world, &cfg, closed) {
        Ok(prepared) => prepared,
        Err(err) => return failed("windows", err.to_string()),
    };
    let inputs = (world.non_send::<GhostBootInputsSource>().0)(&cfg, origin);
    let Ok(session) = boot_ghost_strict(world, inputs, &prepared.descript, &ghost, &balloon) else {
        // 成立しなかった理由は起動の結線が記録済み。
        return failed("boot", String::new());
    };
    commit_ghost_windows(world, prepared);
    world.insert_non_send(GhostSlot(Some(session)));
    if let Some(mut ctx) = world.get_resource_mut::<BootContext>() {
        ctx.current = CurrentGhost {
            cfg,
            ghost,
            balloon,
        };
    }
    true
}

/// 段を「迎え入れ」にして `ghost_switch_booted` を残す。
fn set_stage(world: &mut World, attempt: WelcomeAttempt) {
    let ghost = world
        .get_resource::<BootContext>()
        .and_then(|c| c.current.ghost.folder.clone());
    match world.get_non_send_mut::<SwitchInFlight>() {
        Some(mut flight) => flight.stage = SwitchStage::Welcoming { attempt },
        None => tracing::warn!(
            event = "ghost_switch_no_reservation",
            "切替の予約が無い——段は記録しない"
        ),
    }
    tracing::info!(
        event = "ghost_switch_booted",
        ghost = ?ghost,
        attempt = ?attempt,
        "切替でゴーストを起こした（定常到達を待つ）"
    );
}

/// 致命: 予約を下ろし、出所「既定ゴーストへ戻せなかった」で終了を指示する（要件 6.4）。
/// 告知と終了コード 1 は `main` の後始末が今日の SHIORI の失敗の経路で行う。
fn fatal(world: &mut World, reason: String) {
    world.remove_non_send::<SwitchInFlight>();
    tracing::error!(
        event = "ghost_switch_fatal",
        reason = %reason,
        "既定ゴーストへ戻せない——終了する"
    );
    quit_app(
        world,
        ExitOrigin::GhostFallbackFailed(ShioriFault {
            kind: ShioriFaultKind::Internal,
            reason,
        }),
    );
}

#[cfg(test)]
#[path = "ghost_switch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ghost_switch_fallback_tests.rs"]
mod fallback_tests;
