//! UI 側の窓口（design「窓口（`update/desk.rs`）」）。
//!
//! 窓口 [`UpdateDesk`] は World の NonSend・プロセスに 1 つで、ゴーストを起こし直しても作り直さない
//! （据えるのは [`super::register`]）。取り出しの系 [`drain`] が毎 tick ⑴ 背景スレッドの頼み
//! [`DeskAsk`] ⑵ 入口からの生の要求 [`RawUpdateRequest`] ⑶ `homeurl` の照会の返事 を順に捌く。
//! 終了が始まったら [`discard_for_exit`] が照会の返事待ちを捨てる（走っている依頼は門が待つ）。
//!
//! 対象の解決 [`resolve_targets`] は、置き場のゴーストのフォルダ・起動時に解いたシェルとバルーンの
//! フォルダから今の 3 つを、ゴーストのシェルの目録と根のバルーンの目録の `name` から `updateother`
//! の名前を引く（要件 1.4・1.7）。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_actor::ReplyReceiver;
use areka_ghost::BasewareRoot;
use areka_ghost::catalog::{descript_name, homeurl, list_balloons, list_shells};
use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::world::World;

use super::refs::{ON_UPDATE_FAILURE, executing_refs};
use super::worker::{DeskAsk, UpdateJob, spawn_worker, winhttp_fetch};
use super::{RawUpdateRequest, SubmitVerdict, TargetKind, TargetSpec, UpdateReason};
use crate::boot_config::BootContext;
use crate::exit_wait::WorkGate;
use crate::ghost_session::GhostSlot;
use crate::menu::captions::QueryReply;

/// UI 側の窓口（World の NonSend・プロセスに 1 つ）。
pub(crate) struct UpdateDesk {
    /// 入口へ配る生の要求の送出端と、その受信端。
    // 読むのは `raw_sender` だけ＝台本の受け口へ配るタスク 7.1 で外す。
    #[allow(dead_code)]
    raw_tx: Sender<RawUpdateRequest>,
    raw_rx: Receiver<RawUpdateRequest>,
    /// 背景スレッドの頼みの送出端（スレッドへ渡す）と、その受信端。
    asks_tx: Sender<DeskAsk>,
    asks_rx: Receiver<DeskAsk>,
    /// 背景スレッドへの仕事の送出端（最初の依頼で 1 度だけ起こす）。
    pub(super) worker: Option<Sender<UpdateJob>>,
    /// 手続きの段。
    pub(super) stage: Stage,
    /// 答え待ちの間に届いた要求の預かり（高々 1 件・対象はまだ解かない）。
    pub(super) held: Option<(RawUpdateRequest, UpdateReason)>,
    /// 終了の待ちへ登記した門（背景スレッドと共有）。
    pub(super) gate: Arc<WorkGate>,
    /// `homeurl` の照会の返事待ち（高々 1 件）。
    homeurl_query: Option<ReplyReceiver<QueryReply>>,
}

impl UpdateDesk {
    pub(super) fn new(gate: Arc<WorkGate>) -> Self {
        let (raw_tx, raw_rx) = mpsc::channel();
        let (asks_tx, asks_rx) = mpsc::channel();
        UpdateDesk {
            raw_tx,
            raw_rx,
            asks_tx,
            asks_rx,
            worker: None,
            stage: Stage::Idle,
            held: None,
            gate,
            homeurl_query: None,
        }
    }
}

/// 手続きの段。`Idle`＝走っていない／`AwaitingExec`＝メニューの要求が `OnUpdateProcessExec` の答えを
/// 待っている／`Running`＝標準の手続きが走っている。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    Idle,
    AwaitingExec,
    Running,
}

/// 台本の受け口へ配る生の要求の送出端（窓口が無ければ受信端の無い送出端）。
// 呼び手（台本の受け口の列への登記）はタスク 7.1 が足す。そのとき外す。
#[allow(dead_code)]
pub(crate) fn raw_sender(world: &World) -> Sender<RawUpdateRequest> {
    match world.get_non_send::<UpdateDesk>() {
        Some(desk) => desk.raw_tx.clone(),
        None => mpsc::channel().0,
    }
}

/// 取り出しの系（Input の段・`dispatch_pointer_events` の後）: 背景スレッドの頼み → 生の要求 →
/// 照会の返事の順に捌く。窓口が無ければ無操作。
pub(crate) fn drain(world: &mut World) {
    let Some(desk) = world.get_non_send::<UpdateDesk>() else {
        return;
    };
    let asks: Vec<DeskAsk> = desk.asks_rx.try_iter().collect();
    let raws: Vec<RawUpdateRequest> = desk.raw_rx.try_iter().collect();
    for ask in asks {
        answer(world, ask);
    }
    for raw in raws {
        take_raw(world, raw);
    }
    peek_homeurl(world);
}

/// 背景スレッドの頼みを 1 件捌く。`Started`＝答え待ちなら走っているへ移し、預かりを `executing` で
/// 断る（答え待ち以外では無操作）／`OrderDone`＝走っていないへ戻し、預かりを受付に掛け直す。正しさは
/// 段の判定が持ち、頼みと生の要求の届く順に依らない。
fn answer(world: &mut World, ask: DeskAsk) {
    tracing::debug!(event = "update_desk_ask", ask = ?ask, "[update] 背景スレッドの頼みを受けました");
    let Some(mut desk) = world.get_non_send_mut::<UpdateDesk>() else {
        return;
    };
    match ask {
        DeskAsk::Started => {
            if desk.stage != Stage::AwaitingExec {
                return;
            }
            set_stage(&mut desk, Stage::Running);
            if let Some((raw, reason)) = desk.held.take() {
                refuse_executing(world, &raw, reason);
            }
        }
        DeskAsk::OrderDone => {
            set_stage(&mut desk, Stage::Idle);
            if let Some((raw, reason)) = desk.held.take() {
                super::submit(world, raw, reason);
            }
        }
        // 読み直しは 6.5 が足す。
        DeskAsk::Reload { .. } => {}
    }
}

/// 入口（台本の受け口）からの生の要求を 1 件、理由 `script` で受付へ掛ける。
fn take_raw(world: &mut World, raw: RawUpdateRequest) {
    tracing::debug!(event = "update_desk_raw", raw = ?raw, "[update] 入口からの要求を受けました");
    super::submit(world, raw, UpdateReason::Script);
}

/// 段を移す（変わったときだけ `debug!(update_stage)`）。
fn set_stage(desk: &mut UpdateDesk, to: Stage) {
    if desk.stage != to {
        tracing::debug!(
            event = "update_stage",
            from = ?desk.stage,
            to = ?to,
            "[update] 手続きの段が移りました"
        );
        desk.stage = to;
    }
}

/// 二重起動を断る: 置き場のゴーストへ `OnUpdateFailure(executing)` を返事なしで 1 件送り、
/// `warn!(update_refused)` を 1 件残す（送れたかを添える）。種別は要求の先頭の対象（空ならゴースト）。
pub(super) fn refuse_executing(world: &World, raw: &RawUpdateRequest, reason: UpdateReason) {
    let kind = match raw {
        RawUpdateRequest::Current(kinds) => kinds.first().copied(),
        RawUpdateRequest::Other(names) => names.first().map(|(kind, _)| *kind),
    }
    .unwrap_or(TargetKind::Ghost);
    let kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|s| s.kanade());
    let notified = kanade.is_some_and(|kanade| {
        kanade
            .send(KanadeMsg::RaiseEvent {
                id: ON_UPDATE_FAILURE.to_owned(),
                references: executing_refs(kind, reason),
                method: ShioriMethod::Get,
                reply: None,
            })
            .is_ok()
    });
    tracing::warn!(
        event = "update_refused",
        verdict = ?SubmitVerdict::Executing,
        kind = kind.as_ref_str(),
        reason = reason.as_ref_str(),
        notified,
        "[update] 更新が走っているので要求を断ります（notified＝OnUpdateFailure(executing) を送れたか）"
    );
}

/// 受けた依頼を背景スレッドへ渡す（最初の依頼でスレッドを起こす）。渡せたらメニューは答え待ち、
/// 台本は走っているの段へ（台本は `OnUpdateProcessExec` を送らないので待つ段が無い）。
pub(super) fn hand_over(world: &mut World, job: UpdateJob) -> SubmitVerdict {
    let Some(mut desk) = world.get_non_send_mut::<UpdateDesk>() else {
        return SubmitVerdict::NoDesk;
    };
    let desk = &mut *desk;
    let worker = desk.worker.get_or_insert_with(|| {
        tracing::debug!(
            event = "update_worker_spawned",
            "[update] 背景スレッドを起こします"
        );
        // 取っ手は持たない（落としても join しない）。終了で待つのは門の役目。
        spawn_worker(desk.asks_tx.clone(), desk.gate.clone(), winhttp_fetch()).0
    });
    let reason = job.order.reason;
    let names: Vec<String> = job.order.targets.iter().map(|t| t.name.clone()).collect();
    if worker.send(job).is_err() {
        desk.worker = None;
        tracing::error!(
            event = "update_worker_gone",
            targets = ?names,
            "[update] 背景スレッドが居ないので依頼を扱えません（次の依頼で起こし直します）"
        );
        return SubmitVerdict::WorkerGone;
    }
    tracing::info!(
        event = "update_order_started",
        origin = reason.as_ref_str(),
        targets = ?names,
        "[update] 更新の依頼を受けました"
    );
    let to = match reason {
        UpdateReason::Manual => Stage::AwaitingExec,
        UpdateReason::Script => Stage::Running,
    };
    set_stage(desk, to);
    SubmitVerdict::Started
}

/// `homeurl` の照会の返事を覗く（写しへ置くのは 6.4 が足す）。
fn peek_homeurl(_world: &mut World) {}

/// 終了が始まった（`exit_wait::begin_close` が呼ぶ）: 照会の返事待ちを捨てる。走っている依頼は門が待つ。
pub(crate) fn discard_for_exit(world: &mut World) {
    let Some(mut desk) = world.get_non_send_mut::<UpdateDesk>() else {
        return;
    };
    if desk.homeurl_query.take().is_some() {
        tracing::debug!(
            event = "update_query_discarded",
            "[update] 終了が始まったので、homeurl の照会の返事待ちを捨てます"
        );
    }
}

/// 今の 3 つを解く材料（World から写す）。
struct Here {
    root: BasewareRoot,
    /// 置き場のゴーストのフォルダ（`ghost/<フォルダ名>`）。
    ghost_dir: PathBuf,
    /// 実行系が読んだゴーストの名前（実行系が無ければ None）。
    ghost_name: Option<String>,
    /// 起動時に解いたシェルのフォルダ（実行系が無ければ None）。
    shell_dir: Option<PathBuf>,
    /// 起動時に解いたバルーンのフォルダ。
    balloon_dir: PathBuf,
}

/// 対象を解く（起動の文脈と置き場のゴーストを読む・どちらかが無ければ 0 件）。引けない名前・無い
/// フォルダは `warn!` で飛ばす。
pub(super) fn resolve_targets(world: &World, raw: &RawUpdateRequest) -> Vec<TargetSpec> {
    let Some(here) = here(world) else {
        return Vec::new();
    };
    resolve(&here, raw)
}

fn here(world: &World) -> Option<Here> {
    let ctx = world.get_resource::<BootContext>()?;
    let session = world.get_non_send::<GhostSlot>()?.0.as_ref()?;
    Some(Here {
        root: ctx.root.clone(),
        ghost_dir: session.ghost_dir().to_path_buf(),
        ghost_name: session.names().and_then(|n| n.name.clone()),
        shell_dir: session.runtime().map(|r| r.mount().shell.dir.clone()),
        balloon_dir: ctx.current.balloon.dir.clone(),
    })
}

/// 生の要求を対象の列へ（並んだ順）。今の 3 つ: ゴーストは置き場のフォルダ（名前は実行系の名前 →
/// `descript.txt` の `name` → フォルダ名）、シェル・バルーンは起動時に解いたフォルダ（名前は
/// `descript.txt` の `name` → フォルダ名）。`updateother` はゴーストのシェルの目録と根のバルーンの
/// 目録の `name` に完全一致（大文字小文字を区別）で引く（隠しシェルは目録に無い）。
fn resolve(here: &Here, raw: &RawUpdateRequest) -> Vec<TargetSpec> {
    match raw {
        RawUpdateRequest::Current(kinds) => kinds
            .iter()
            .filter_map(|&kind| match kind {
                TargetKind::Ghost => target(kind, here.ghost_dir.clone(), here.ghost_name.clone()),
                TargetKind::Shell => match &here.shell_dir {
                    Some(dir) => target(kind, dir.clone(), None),
                    None => skip(kind, "no_shell", ""),
                },
                TargetKind::Balloon => target(kind, here.balloon_dir.clone(), None),
            })
            .collect(),
        RawUpdateRequest::Other(names) => names
            .iter()
            .filter_map(|(kind, name)| {
                let named = |entry_name: &Option<String>| entry_name.as_deref() == Some(name);
                let dir = match kind {
                    TargetKind::Shell => list_shells(&here.ghost_dir)
                        .into_iter()
                        .find(|e| named(&e.identity.name))
                        .map(|e| e.dir),
                    TargetKind::Balloon => list_balloons(&here.root)
                        .into_iter()
                        .find(|e| named(&e.identity.name))
                        .map(|e| e.dir),
                    // 入口（`updateother`）はシェル・バルーンしか作らない。
                    TargetKind::Ghost => None,
                };
                match dir {
                    Some(dir) => target(*kind, dir, Some(name.clone())),
                    None => skip(*kind, "name_not_found", name),
                }
            })
            .collect(),
    }
}

/// 対象 1 つを組む（フォルダが無ければ `warn!` で飛ばす）。`first_name` が無ければ `descript.txt` の
/// `name`、それも無ければフォルダ名。倒れ先の `homeurl` は同じ `descript.txt` のもの。
fn target(kind: TargetKind, dir: PathBuf, first_name: Option<String>) -> Option<TargetSpec> {
    if !dir.is_dir() {
        return skip(kind, "no_folder", &dir.display().to_string());
    }
    let descript_dir = match kind {
        TargetKind::Ghost => dir.join("ghost").join("master"),
        TargetKind::Shell | TargetKind::Balloon => dir.clone(),
    };
    let name = first_name
        .or_else(|| descript_name(&descript_dir))
        .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| dir.display().to_string());
    let spec = TargetSpec {
        kind,
        name,
        descript_homeurl: homeurl(&descript_dir),
        dir,
    };
    tracing::debug!(
        event = "update_target_resolved",
        kind = kind.as_ref_str(),
        name = %spec.name,
        dir = %spec.dir.display(),
        homeurl = spec.descript_homeurl.as_deref(),
        "[update] 対象を解きました"
    );
    Some(spec)
}

/// 解けない対象を飛ばす（`warn!` を 1 件）。
fn skip(kind: TargetKind, reason: &'static str, what: &str) -> Option<TargetSpec> {
    tracing::warn!(
        event = "update_target_skipped",
        kind = kind.as_ref_str(),
        reason,
        what,
        "[update] 対象を解けないので飛ばします"
    );
    None
}

#[cfg(test)]
#[path = "desk_resolve_tests.rs"]
mod resolve_tests;

#[cfg(test)]
#[path = "desk_tests.rs"]
mod tests;
