//! ゴーストの切替の入口（areka-P0-ghost-shell-balloon-switch・design「GhostSwitch」）。
//!
//! 切替要求は台本の `\![change,ghost,…]` とメニューの「ゴースト」枠の 2 つの出どころから、
//! ここの [`request_ghost_switch`] 1 本だけを通る（要件 1.1）。入口は降ろす前・`OnGhostChanging`
//! を送る前に目録と突き合わせ（要件 1.5〜1.7）、受理したら切替の予約 [`SwitchInFlight`] を
//! 立てて kanade へ切替の要求を送る。台本からの要求は受け口 `ChangeCueSink` から受信端
//! [`ChangeRx`] を経て、入力の段の取り出しの系 [`drain_change_requests`] が入口へ渡す。
//! 送り出しの握手が済んだら [`switch_to`] が降ろして切替先を起こし、失敗なら
//! [`switch_to_default`] が既定ゴーストへ 1 回だけ戻す（それも失敗なら致命で終了）。
//! 台本の特別な名前（`random`・`sequential`・`lastinstalled`）は、入口が目録を読んだ直後に
//! [`resolve_special_name`] で目録のフォルダ名へ解いてから突き合わせる（areka-P0-ghost-change-name-resolution）。

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use areka_ghost::GhostEntry;
use areka_kanade::{
    BootOrigin, ChangeHandoff, ChangeOrigin, ChangeRequest, ChangeTarget, ChangedFrom, CloseReason,
    KanadeMsg, KanadeNotice, KanadeStopCause, KanadeStopped, ShioriFault, ShioriFaultKind,
};
use areka_sylphya::{PersistKey, PersistScope};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use crate::app_exit::{ExitOrigin, WindowsClosed, close_windows_for_restart, quit_app};
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost, resolve_balloon_for_ghost};
use crate::boot_resolve::{
    DEFAULT_GHOST_FOLDER, GhostDecision, GhostRoute, NoBalloon, pick_index, running_name,
    write_switch_drop,
};
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

/// 特別な名前の解決の結果（areka-P0-ghost-change-name-resolution）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NameResolution {
    /// 特別な名前ではない（今日どおり名指しとして目録と突き合わせる）。
    Plain,
    /// 解けた。`position` は `sequential` のときの今のゴーストの位置（記録用・他は `None`）。
    Resolved {
        folder: String,
        position: Option<usize>,
    },
    /// 特別な名前だが解けない（理由は記録の語彙）。
    Unresolved(UnresolvedReason),
}

/// 解けない理由（`ghost_switch_unknown` の `reason` 欄の語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnresolvedReason {
    RandomEmpty,
    SequentialEmpty,
    LastInstalledNone,
    LastInstalledMissing,
}

impl UnresolvedReason {
    /// `reason` 欄の語（`random_empty` など）。
    pub(crate) fn as_ref_str(self) -> &'static str {
        match self {
            Self::RandomEmpty => "random_empty",
            Self::SequentialEmpty => "sequential_empty",
            Self::LastInstalledNone => "lastinstalled_none",
            Self::LastInstalledMissing => "lastinstalled_missing",
        }
    }

    /// 人が読む本文（理由ごとに 1 文）。
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::RandomEmpty => "目録にゴーストが 1 体も無い——random の切替を無視する",
            Self::SequentialEmpty => "目録にゴーストが 1 体も無い——sequential の切替を無視する",
            Self::LastInstalledNone => {
                "このプロセスでゴーストを入れていない——lastinstalled の切替を無視する"
            }
            Self::LastInstalledMissing => {
                "最後に入れたゴーストが目録に無い——lastinstalled の切替を無視する"
            }
        }
    }
}

/// 純粋: 特別な名前を目録のフォルダ名へ解く（要件 2〜5・9・design「SpecialNameResolver」）。
///
/// 特別な名前は `random`・`sequential`・`lastinstalled` の完全一致だけ（大文字小文字を区別）で、
/// 目録に同じ名前のゴーストがいても特別な名前として解く。`entries` は `list_ghosts` の並び
/// （フォルダ名の昇順）のまま渡す。`current`＝今のゴーストのフォルダ名（`None` は目録に無い扱い）、
/// `last_installed`＝最後に入れたゴーストのフォルダ名の記録。
/// - `random`: 今のゴーストを除いた候補から `pick(候補数)` で 1 体。候補が 0 体なら今のゴースト
///   自身（目録に残っているのはそれだけ）。`pick` を呼ぶのは候補が 1 体以上のときだけ。
/// - `sequential`: 今の位置の次（末尾なら先頭）。今のゴーストが目録に無ければ先頭。
/// - `lastinstalled`: 記録のフォルダ名で目録を引く（今のゴースト自身でも解ける）。
///
/// fs・World・時計は読まない。
pub(crate) fn resolve_special_name(
    name: &str,
    entries: &[GhostEntry],
    current: Option<&str>,
    last_installed: Option<&str>,
    pick: impl FnOnce(usize) -> usize,
) -> NameResolution {
    let resolved = |e: &GhostEntry, position| NameResolution::Resolved {
        folder: e.identity.folder.clone(),
        position,
    };
    let is_current = |e: &&GhostEntry| Some(e.identity.folder.as_str()) == current;
    match name {
        "random" => {
            let others: Vec<&GhostEntry> = entries.iter().filter(|e| !is_current(e)).collect();
            match (others.is_empty(), entries.first()) {
                (false, _) => resolved(others[pick(others.len())], None),
                (true, Some(itself)) => resolved(itself, None),
                (true, None) => NameResolution::Unresolved(UnresolvedReason::RandomEmpty),
            }
        }
        "sequential" => {
            if entries.is_empty() {
                return NameResolution::Unresolved(UnresolvedReason::SequentialEmpty);
            }
            let position = entries.iter().position(|e| is_current(&e));
            let next = position.map_or(0, |i| (i + 1) % entries.len());
            resolved(&entries[next], position)
        }
        "lastinstalled" => match last_installed {
            None => NameResolution::Unresolved(UnresolvedReason::LastInstalledNone),
            Some(want) => entries.iter().find(|e| e.identity.folder == want).map_or(
                NameResolution::Unresolved(UnresolvedReason::LastInstalledMissing),
                |e| resolved(e, None),
            ),
        },
        _ => NameResolution::Plain,
    }
}

/// 同じプロセスで最後に入れたゴーストのフォルダ名の記録（`lastinstalled` の解決に使う）。
///
/// プロセスの中だけ（ファイル・記憶へは書かない）。無ければ「このプロセスで何も入れていない」。
/// 入口は読むだけで、切替に使っても消さない。
#[derive(Resource)]
pub(crate) struct LastInstalledGhost(pub String);

/// 最後に入れたゴーストのフォルダ名を記録する（前の記録は置き換わる＝最後の 1 件だけ残る）。
pub(crate) fn record_last_installed(world: &mut World, folder: String) {
    tracing::info!(
        event = "last_installed_recorded",
        folder = %folder,
        "最後に入れたゴーストを記録した"
    );
    world.insert_resource(LastInstalledGhost(folder));
}

/// 切替要求の唯一の入口（要件 1.1・1.6・1.7・1.9）。乱数は本番の `pick_index`。
pub(crate) fn request_ghost_switch(world: &mut World, req: SwitchRequest) -> SwitchVerdict {
    request_ghost_switch_with(world, req, pick_index)
}

/// 入口の中身。乱数を外から受ける（決定論テスト用・本番は [`request_ghost_switch`] が `pick_index` を渡す）。
///
/// ⑴ 予約が在れば `warn!(ghost_switch_busy)`。⑵ 起動の文脈の根で目録を読み、名前の要求だけ特別な
/// 名前を解いてから（解けたら `info!(ghost_switch_resolved)` でフォルダの名指しへ読み替える）突き合わせ、
/// 切替先が決まらない（該当なし・特別な名前が解けない）ときは `warn!(ghost_switch_unknown)` を
/// `reason` つきで 1 件（降ろさず kanade へ何も送らない）。⑶ 置き場のゴーストの送出端が
/// 無ければ `warn!(ghost_switch_no_context)`。⑷ 切替先の `sakura.name` だけを読み（要件 8.7）、
/// kanade へ切替の要求を送って予約を立てる。送出に失敗したら `error!(ghost_switch_send_failed)` で
/// 予約は立てない（送ってから立てるので、下ろすべき予約は残らない）。
pub(crate) fn request_ghost_switch_with(
    world: &mut World,
    req: SwitchRequest,
    pick: impl FnOnce(usize) -> usize,
) -> SwitchVerdict {
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
    let current_folder = ctx.current.ghost.folder.clone();
    // 特別な名前は同名のゴーストより先に解く。メニューのフォルダの名指しは素通し。記録は読むだけ。
    let spec = match &req.ghost {
        GhostSpec::Name(name) => {
            let last = world
                .get_resource::<LastInstalledGhost>()
                .map(|r| r.0.as_str());
            match resolve_special_name(name, &entries, current_folder.as_deref(), last, pick) {
                NameResolution::Plain => req.ghost.clone(),
                NameResolution::Resolved { folder, position } => {
                    tracing::info!(
                        event = "ghost_switch_resolved",
                        name = %name,
                        to = %folder,
                        position = ?position,
                        "特別な名前を目録のフォルダ名へ解いた"
                    );
                    GhostSpec::Folder(folder)
                }
                NameResolution::Unresolved(reason) => {
                    tracing::warn!(
                        event = "ghost_switch_unknown",
                        reason = reason.as_ref_str(),
                        spec = ?req.ghost,
                        "{}",
                        reason.describe()
                    );
                    return SwitchVerdict::NotFound;
                }
            }
        }
        GhostSpec::Folder(_) => req.ghost.clone(),
    };
    let Some(mut target) = resolve_switch_target(&entries, &spec) else {
        tracing::warn!(
            event = "ghost_switch_unknown",
            reason = "name",
            spec = ?req.ghost,
            "切替先が目録のどのゴーストにも一致しない——切替を無視する（降ろさず知らせも送らない）"
        );
        return SwitchVerdict::NotFound;
    };

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

/// 予約の下で届いた停止通知を切替の段で振り分ける（要件 2.4・2.8・3.3・6.6・design Flow 3）。
///
/// 送り出し＋中身あり → [`switch_to`]（握手の途中の失敗でも中身があれば続ける・要件 2.8）。
/// 送り出し＋中身なし → `warn!(ghost_switch_not_accepted)`・予約を下ろし今日どおり終了。
/// 迎え入れ（切替先）＋失敗 → `error!(ghost_switch_target_fault)` の上で [`switch_to_default`]。
/// 迎え入れ（既定）か、既定ゴーストへの切替の迎え入れ＋失敗 → 予約を下ろし今日の失敗の経路
/// （告知・終了コード 1）で終了（戻す試みは 1 回だけ・要件 6.5）。記録は既定ゴーストの失敗を表す
/// `error!(ghost_switch_default_fault)`（切替先の失敗 `ghost_switch_target_fault` とは別の語）。
/// 迎え入れ＋失敗以外 → `info!(ghost_switch_target_quit)`・予約を下ろし今日どおり終了。
/// 予約が無ければ今日どおり終了（呼び手は予約が在るときだけ呼ぶ）。エラー応答は停止通知を
/// 生まないので、ここに判断は無い（要件 6.7）。
pub(crate) fn on_ghost_stopped(world: &mut World, stopped: KanadeStopped) {
    let Some((stage, target_name, target_is_default)) =
        world.get_non_send::<SwitchInFlight>().map(|f| {
            let is_default = f.target.folder == DEFAULT_GHOST_FOLDER;
            (f.stage, f.target.name.clone(), is_default)
        })
    else {
        tracing::debug!(
            event = "ghost_switch_no_reservation",
            cause = ?stopped.cause,
            "切替の予約が無い停止通知——今日どおり終了する"
        );
        quit_app(world, ExitOrigin::KanadeStopped(stopped.cause));
        return;
    };
    let cause = stopped.cause;
    match (stage, stopped.handoff) {
        (SwitchStage::SendOff, Some(handoff)) => switch_to(world, handoff),
        (SwitchStage::SendOff, None) => {
            tracing::warn!(
                event = "ghost_switch_not_accepted",
                cause = ?cause,
                "kanade が切替の要求を受理しないまま止まった（終了要求と競合）——切替をやめて今日どおり終了する"
            );
            world.remove_non_send::<SwitchInFlight>();
            quit_app(world, ExitOrigin::KanadeStopped(cause));
        }
        (SwitchStage::Welcoming { attempt }, _) => match (attempt, cause) {
            (WelcomeAttempt::Target, KanadeStopCause::Fault(fault)) if !target_is_default => {
                tracing::error!(
                    event = "ghost_switch_target_fault",
                    ghost = %target_name,
                    reason = %fault.reason,
                    "切替先の SHIORI が失敗した——既定ゴーストへ戻す"
                );
                switch_to_default(world, &target_name);
            }
            (_, KanadeStopCause::Fault(fault)) => {
                tracing::error!(
                    event = "ghost_switch_default_fault",
                    ghost = %DEFAULT_GHOST_FOLDER,
                    attempt = ?attempt,
                    reason = %fault.reason,
                    "既定ゴーストの SHIORI が失敗した（戻す試みは済んだ／切替先が既定）——今日の失敗の経路で終了する"
                );
                world.remove_non_send::<SwitchInFlight>();
                quit_app(
                    world,
                    ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault)),
                );
            }
            (attempt, cause) => {
                tracing::info!(
                    event = "ghost_switch_target_quit",
                    attempt = ?attempt,
                    cause = ?cause,
                    "迎え入れたゴーストが失敗以外で止まった——今日どおり終了する"
                );
                world.remove_non_send::<SwitchInFlight>();
                quit_app(world, ExitOrigin::KanadeStopped(cause));
            }
        },
    }
}

/// 運行の通知を切替の段で振り分ける（要件 3.5・5.2）。
///
/// 定常到達: 迎え入れの予約を下ろし `info!(ghost_switch_done)`。送り出しの段の予約は残す（まだ
/// 握手の途中）。予約が無ければ `debug!`（初回起動・切替の外）。切替の中止: 予約を下ろし
/// `info!(ghost_switch_cancelled)`。予約が無ければ `warn!`。停止は [`on_ghost_stopped`] へ。
pub(crate) fn on_notice(world: &mut World, notice: KanadeNotice) {
    let stage = world.get_non_send::<SwitchInFlight>().map(|f| f.stage);
    match notice {
        KanadeNotice::Steady => {
            match stage {
                Some(SwitchStage::Welcoming { attempt }) => {
                    record_steady_memory(world);
                    world.remove_non_send::<SwitchInFlight>();
                    let ghost = world
                        .get_resource::<BootContext>()
                        .and_then(|c| c.current.ghost.folder.clone());
                    tracing::info!(
                        event = "ghost_switch_done",
                        ghost = ?ghost,
                        attempt = ?attempt,
                        "切替で起こしたゴーストが定常に入った——切替を終える"
                    );
                }
                stage => tracing::debug!(
                    event = "ghost_switch_done",
                    stage = ?stage,
                    "迎え入れの予約の外の定常到達——予約は触らない"
                ),
            }
            // インストールの窓口へ（送り直しの頼みは定常到達の回数で見直す）。
            crate::install::desk::on_steady(world);
        }
        KanadeNotice::ChangeCancelled { reason } => {
            if world.remove_non_send::<SwitchInFlight>().is_some() {
                tracing::info!(
                    event = "ghost_switch_cancelled",
                    reason = ?reason,
                    "切替は中止された——予約を下ろす（元のゴーストは定常へ戻るか、終了要求が勝っていれば今日どおり終わる）"
                );
            } else if world.contains_resource::<crate::session_end::SessionEnded>() {
                tracing::debug!(
                    event = "ghost_switch_cancelled",
                    reason = ?reason,
                    "OS のセッションの終了で予約は下ろし済み——遅れて届いた中止の通知は読み捨てる"
                );
            } else {
                tracing::warn!(
                    event = "ghost_switch_cancelled",
                    reason = ?reason,
                    "切替の予約が無いのに中止の通知が届いた——無視する"
                );
            }
        }
        KanadeNotice::Stopped(stopped) => on_ghost_stopped(world, stopped),
    }
}

/// 迎え入れたゴーストが定常に入った時点の記憶（要件 4.6・4.7・12.6・12.7・design Flow 6）。
///
/// 段を問わず同じ手順で、置き場のゴーストの記憶の書き手へ最後に使ったもの（ゴースト・バルーン・
/// シェル）と、argv で始まったプロセスでなければ印＝今のゴーストの名前を投函する。実行系が動いて
/// いるので UI スレッドから App スコープへ直接は書かない（書き手の中で直列にする）。置き場・実行系・
/// 文脈が無ければ `warn!(steady_memory_not_recorded)` で続ける（最後のゴーストは既定のまま）。
fn record_steady_memory(world: &World) {
    let ctx = world.get_resource::<BootContext>();
    let slot = world.get_non_send::<GhostSlot>();
    let runtime = slot
        .as_ref()
        .and_then(|s| s.0.as_ref())
        .and_then(|s| s.runtime());
    let (Some(ctx), Some(runtime)) = (ctx, runtime) else {
        let reason = match (ctx.is_some(), slot.as_ref().map(|s| s.0.is_some())) {
            (false, _) => "boot_context",
            (true, None | Some(false)) => "ghost_slot",
            (true, Some(true)) => "runtime",
        };
        tracing::warn!(
            event = "steady_memory_not_recorded",
            reason,
            "定常に入ったゴーストの記憶を書く相手が無い（最後のゴーストは既定のまま・次の起動は既定で起きる）"
        );
        return;
    };
    let publisher = runtime.sylphya_publisher();
    crate::record_last_used(
        publisher,
        &runtime.mount().shell.dir,
        &ctx.current.ghost,
        &ctx.current.balloon,
    );
    if ctx.argv_session {
        tracing::debug!(
            event = "session_mark_untouched_argv",
            "argv で始まったプロセスなので定常到達でも起動中の印に触れない"
        );
        return;
    }
    let name = running_name(&ctx.root, &ctx.current.ghost);
    publisher.persist_put(
        PersistScope::App,
        vec![(PersistKey::LastRunning, name.clone())],
    );
    tracing::info!(
        event = "session_mark_steady",
        ghost = %name,
        "定常に入ったゴーストの名前を起動中の印へ投函した"
    );
}

/// 切替先を起こす（要件 2.7・3.1・3.2・3.7・3.8・4.6・4.7・6.1・design Flow 1）。
///
/// 予約の切替先へ: 置き場のゴーストを同期で降ろす → 全窓を閉じる → [`boot_into`]（由来＝切替で
/// 来た・経路＝切替）。成功なら段を「迎え入れ（切替先）」に。同期の失敗は、切替先が既定なら致命、
/// そうでなければ [`switch_to_default`]（窓は投函していないので壊れた切替先の窓は生えない）。
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
    // 降ろし終えた直後（前のゴーストの記憶の書き手は処理し切って join 済み＝動いている実行系は 0）に
    // 最後のゴースト＝既定・印＝切替先を 1 回で書く（要件 12.6）。UI スレッドが App スコープへ直接
    // 書くのは実行系が 1 つも動いていない間だけ（動いている間はそのゴーストの記憶の書き手を通す）。
    // 既定への戻しと致命はこれに触れない（印は切替先のまま・要件 12.7・12.8）。
    match world.get_resource::<BootContext>() {
        Some(ctx) => write_switch_drop(
            &ctx.app_profile_dir,
            (!ctx.argv_session).then_some(target.name.as_str()),
        ),
        None => tracing::warn!(
            event = "boot_context_missing",
            "起動の文脈が無いので降ろした直後の記憶を書けない（最後のゴーストと印は前のゴーストのまま）"
        ),
    }
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

#[cfg(test)]
#[path = "ghost_switch_notice_tests.rs"]
mod notice_tests;
