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

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_actor::ReplyReceiver;
use areka_ghost::BasewareRoot;
use areka_ghost::catalog::{descript_name, homeurl, list_balloons, list_shells};
use areka_kanade::online::OnlineCounter;
use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{ChangeOrigin, KanadeMsg, ShioriMethod};
use bevy_ecs::world::World;

use super::refs::{ON_UPDATE_FAILURE, executing_refs};
use super::worker::{DeskAsk, NewFetch, UpdateJob, spawn_worker, winhttp_fetch};
use super::{RawUpdateRequest, SubmitVerdict, TargetKind, TargetSpec, UpdateReason};
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::exit_wait::WorkGate;
use crate::ghost_session::GhostSlot;
use crate::menu::captions::{QueryReply, send_query};

/// 後送りの列（ゴーストの成功の締め → 総括・決めたこと 21）。
type Tail = Vec<(&'static str, Vec<String>)>;

/// UI 側の窓口（World の NonSend・プロセスに 1 つ）。
pub(crate) struct UpdateDesk {
    /// 入口へ配る生の要求の送出端と、その受信端。
    raw_tx: Sender<RawUpdateRequest>,
    raw_rx: Receiver<RawUpdateRequest>,
    /// 背景スレッドの頼みの送出端（スレッドへ渡す）と、その受信端。
    asks_tx: Sender<DeskAsk>,
    asks_rx: Receiver<DeskAsk>,
    /// 背景スレッドへの仕事の送出端（最初の依頼で 1 度だけ起こす）。
    pub(super) worker: Option<Sender<UpdateJob>>,
    /// 背景スレッドへ渡す取得口の作り方（本番は [`winhttp_fetch`]・テストは起こす前に偽物へ差し替える）。
    pub(super) new_fetch: NewFetch,
    /// 背景スレッドへ渡す通信中の数（本番はプロセスの数・テストは起こす前に自分の数へ差し替える）。
    pub(super) online: &'static OnlineCounter,
    /// 手続きの段。
    pub(super) stage: Stage,
    /// 答え待ちの間に届いた要求の預かり（高々 1 件・対象はまだ解かない）。
    pub(super) held: Option<(RawUpdateRequest, UpdateReason)>,
    /// 終了の待ちへ登記した門（背景スレッドと共有）。
    pub(super) gate: Arc<WorkGate>,
    /// 定常到達のたびに照会した SHIORI の `homeurl` の写し（灰色の判定用）。
    ghost_homeurl: Option<String>,
    /// `homeurl` の照会の返事待ち（高々 1 件）。
    homeurl_query: Option<ReplyReceiver<QueryReply>>,
    /// 受け付けた読み直しのゴーストのフォルダと後送りの列の残り（その切替が終わったら成功した走行の
    /// 残りを消し、列の残りを送る・失敗や中止なら捨てる・要件 5.9・決めたこと 24）。
    pub(super) after_switch: Option<(PathBuf, Tail)>,
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
            new_fetch: winhttp_fetch(),
            online: default_online(),
            stage: Stage::Idle,
            held: None,
            gate,
            ghost_homeurl: None,
            homeurl_query: None,
            after_switch: None,
        }
    }
}

/// 窓口の通信中の数の既定。本番はプロセスの数。テストのビルドでは、窓口を据えるだけの他のテスト
/// （メニュー・台本の受け口・切替の土台）が背景スレッドを起こしてもプロセスの数に触れないよう、
/// 誰も読まないテスト専用の数にする（読む側のテストは起こす前に自分の数へ差し替える）。
#[cfg(not(test))]
fn default_online() -> &'static OnlineCounter {
    &areka_kanade::online::PROCESS
}

#[cfg(test)]
fn default_online() -> &'static OnlineCounter {
    static UNOBSERVED: OnlineCounter = OnlineCounter::new();
    &UNOBSERVED
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
/// 断る（答え待ち以外では無操作）／`OrderDone`＝走っていないへ戻し、預かりを受付に掛け直す／
/// `Reload`＝[`reload`]（条件を満たせば同じゴーストへの切替を 1 回頼む）。正しさは
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
        DeskAsk::Reload { ghost_dir, tail } => reload(world, &ghost_dir, tail),
    }
}

/// 読み直しの頼み: 条件を満たせば既存の切替の入口へ「同じフォルダ・知らせなし・出どころ＝自動・
/// 起動の知らせ＝列の先頭」で 1 回頼み、判定を残す（`Accepted` は `info!`・他は `warn!`）。満たさなければ
/// 理由つきの `warn!(update_reload_skipped)` で頼まない。頼み直しはしない（終了の後に届いた頼みも落とす）。
/// 列の行き先（決めたこと 24）: `Accepted` ならゴーストのフォルダと列の残りを覚え、その切替が終わったら
/// [`on_switch_end`] が残りを消して列の残りを送る／`argv`・`switching`・入口が受け付けない
/// （`Busy`・`NotFound`・`NoContext`）なら列を今のゴーストへ送る／`closing`・`other_ghost` なら捨てる。
fn reload(world: &mut World, ghost_dir: &Path, mut tail: Tail) {
    let folder = match reload_folder(world, ghost_dir) {
        Ok(folder) => folder,
        Err(reason) => {
            tracing::warn!(
                event = "update_reload_skipped",
                reason,
                ghost_dir = %ghost_dir.display(),
                "[update] 読み直しの条件を満たさないので、読み直しません（更新した中身は次の起動で効きます）"
            );
            match reason {
                // 終了の後（要件 7.4）・別のゴースト（要件 5.7）へは送らない。
                "closing" | "other_ghost" => drop_tail(&tail, reason),
                _ => send_tail(world, tail, "no_reload"),
            }
            return;
        }
    };
    let head = (!tail.is_empty()).then(|| tail.remove(0));
    let verdict = request_ghost_switch(
        world,
        SwitchRequest {
            ghost: GhostSpec::Folder(folder.clone()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: head.clone(),
        },
    );
    if verdict == SwitchVerdict::Accepted {
        if let Some(mut desk) = world.get_non_send_mut::<UpdateDesk>() {
            desk.after_switch = Some((ghost_dir.to_path_buf(), tail));
        }
        tracing::info!(
            event = "update_reload_requested",
            verdict = ?verdict,
            folder = %folder,
            "[update] 更新した中身を読むために、同じゴーストへの切替を頼みました"
        );
    } else {
        tracing::warn!(
            event = "update_reload_requested",
            verdict = ?verdict,
            folder = %folder,
            "[update] 同じゴーストへの切替が受け付けられなかったので、読み直しません"
        );
        tail.splice(0..0, head);
        send_tail(world, tail, "no_reload");
    }
}

/// 後送りの列を置き場のゴーストへ順に送る（返事を待たない・`refuse_executing` と同じ形）。空なら何もしない。
/// 送れたら `info!(update_tail_sent, when, count)`、送出端が無い・途中で送れなくなったら残りを捨てる。
fn send_tail(world: &World, tail: Tail, when: &'static str) {
    if tail.is_empty() {
        return;
    }
    let Some(kanade) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|s| s.kanade())
    else {
        return drop_tail(&tail, "no_ghost");
    };
    let count = tail.len();
    let mut rest = tail.into_iter();
    while let Some((id, references)) = rest.next() {
        let raise = KanadeMsg::RaiseEvent {
            id: id.to_owned(),
            references,
            method: ShioriMethod::Get,
            reply: None,
        };
        if kanade.send(raise).is_err() {
            let left: Tail = std::iter::once((id, Vec::new())).chain(rest).collect();
            return drop_tail(&left, "no_ghost");
        }
    }
    tracing::info!(
        event = "update_tail_sent",
        when,
        count,
        "[update] 後送りの列を今のゴーストへ送りました"
    );
}

/// 後送りの列を捨てる（空でなければ `warn!(update_tail_dropped, reason)` を 1 件）。
fn drop_tail(tail: &[(&'static str, Vec<String>)], reason: &'static str) {
    if tail.is_empty() {
        return;
    }
    let ids: Vec<&str> = tail.iter().map(|(id, _)| *id).collect();
    tracing::warn!(
        event = "update_tail_dropped",
        reason,
        ids = ?ids,
        "[update] 受け取るゴーストが居ないので、後送りの列を捨てます"
    );
}

/// 切替が終わった（`ghost_switch` の切替を終える腕・中止の腕から）。読み直しを覚えていれば、`finished`
/// （頼んだゴーストが定常に入った＝古い SHIORI は降りている）なら成功した走行の残りを消して記録し、
/// 列の残りを新しいゴーストへ送る（`when = "after_switch"`・起動の知らせの台詞の終わりは待たない＝
/// 要件 5.3 の既知の制限）。そうでなければ（既定へ戻った・中止）覚えた物を捨てる（残りは次の走行の始めに
/// エンジンが消す・要件 5.9／列は既定ゴーストへ送らず `warn!(update_tail_dropped, reason =
/// "switch_failed")`・要件 3.6・5.6）。消せた件数は `info!(update_purge_done)`（0 件なら `debug!`）、
/// 消せなかった物は 1 件ずつ `warn!(update_purge_held)`。
pub(crate) fn on_switch_end(world: &mut World, finished: bool) {
    let Some((dir, rest)) = world
        .get_non_send_mut::<UpdateDesk>()
        .and_then(|mut desk| desk.after_switch.take())
    else {
        return;
    };
    if !finished {
        tracing::debug!(
            event = "update_purge_dropped",
            dir = %dir.display(),
            "[update] 読み直しの切替が終わらなかったので、残りは次の走行の始めに消します"
        );
        // 列の先頭（起動の知らせ）も届いていない＝残りが空でも列は捨てた。
        tracing::warn!(
            event = "update_tail_dropped",
            reason = "switch_failed",
            rest = rest.len(),
            "[update] 読み直しの切替が終わらなかったので、後送りの列を捨てます（既定ゴーストへは送りません）"
        );
        return;
    }
    let purge = areka_update::purge_committed(&dir);
    for path in &purge.held {
        tracing::warn!(
            event = "update_purge_held",
            path = %path.display(),
            "[update] 古い SHIORI を降ろした後も更新の残りを消せませんでした（次の走行の始めにもう一度消します）"
        );
    }
    let removed = purge.removed.len();
    if removed > 0 {
        tracing::info!(event = "update_purge_done", removed, dir = %dir.display(), "[update] 読み直しの後に更新の残りを消しました");
    } else {
        tracing::debug!(event = "update_purge_done", removed, dir = %dir.display(), "[update] 読み直しの後に消す更新の残りはありませんでした");
    }
    send_tail(world, rest, "after_switch");
}

/// 読み直す先のフォルダ名。終了が始まった → 切替の予約が在る → 置き場のゴーストが頼みのゴーストと
/// 違う → フォルダ名が無い（引数の起動・裁定 17）の順に理由を返す。
fn reload_folder(world: &World, ghost_dir: &Path) -> Result<String, &'static str> {
    if world
        .get_non_send::<UpdateDesk>()
        .is_none_or(|desk| desk.gate.is_closing())
    {
        return Err("closing");
    }
    if world.get_non_send::<SwitchInFlight>().is_some() {
        return Err("switching");
    }
    let running = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .map(|s| s.ghost_dir());
    if running != Some(ghost_dir) {
        return Err("other_ghost");
    }
    world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone())
        .ok_or("argv")
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
        spawn_worker(
            desk.asks_tx.clone(),
            desk.gate.clone(),
            desk.new_fetch.clone(),
            desk.online,
        )
        .0
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

/// 定常到達（`ghost_switch::on_notice` の定常到達の腕から）: 写しを消し、`homeurl` の照会を 1 件送る。
/// 送れなければ（kanade が居ない・切れている）照会は持たず `debug!` を 1 件。窓口が無い・終了が
/// 始まった後は何もしない（捨てた返事待ちを掛け直さない）。
pub(crate) fn on_steady(world: &mut World) {
    if !world
        .get_non_send::<UpdateDesk>()
        .is_some_and(|desk| !desk.gate.is_closing())
    {
        return;
    }
    let query = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|s| s.kanade())
        .and_then(|kanade| send_query(kanade, vec!["homeurl"]).ok());
    let Some(mut desk) = world.get_non_send_mut::<UpdateDesk>() else {
        return;
    };
    desk.ghost_homeurl = None;
    if query.is_none() {
        tracing::debug!(
            event = "update_homeurl_unqueried",
            "[update] kanade へ homeurl を照会できません（写しは空のまま）"
        );
    }
    desk.homeurl_query = query;
}

/// メニューの項目を選べるか: 窓口が在り・終了が始まっておらず・段が `Idle`（答え待ちも灰色）・
/// 写しか 3 つの `descript.txt` のどれかに更新先が在る。
pub(crate) fn can_update(world: &World) -> bool {
    let Some(desk) = world.get_non_send::<UpdateDesk>() else {
        return false;
    };
    if desk.gate.is_closing() || desk.stage != Stage::Idle {
        return false;
    }
    if desk.ghost_homeurl.is_some() {
        return true;
    }
    here(world).is_some_and(|here| {
        [
            Some(here.ghost_dir.join("ghost").join("master")),
            here.shell_dir,
            Some(here.balloon_dir),
        ]
        .into_iter()
        .flatten()
        .any(|dir| homeurl(&dir).is_some())
    })
}

/// メニューの動作: 今の 3 つを対象に理由 `manual` で受付へ（判定の記録は受付が残す）。
pub(crate) fn update_current(world: &mut World) {
    super::submit(
        world,
        RawUpdateRequest::Current(vec![
            TargetKind::Ghost,
            TargetKind::Shell,
            TargetKind::Balloon,
        ]),
        UpdateReason::Manual,
    );
}

/// `homeurl` の照会の返事を覗く（待たない）。空でない値だけを写しに置く。返事が失敗・返信端が
/// 落ちたときは写しを空のまま `warn!` を 1 件。
fn peek_homeurl(world: &mut World) {
    let Some(mut desk) = world.get_non_send_mut::<UpdateDesk>() else {
        return;
    };
    let reply = match desk.homeurl_query.as_ref().map(ReplyReceiver::try_recv) {
        None | Some(Ok(None)) => return,
        Some(Ok(Some(reply))) => Ok(reply),
        Some(Err(e)) => Err(e.to_string()),
    };
    desk.homeurl_query = None;
    let outcome = reply.map(|reply| reply.into_iter().find(|(id, _)| *id == "homeurl"));
    match outcome {
        Ok(Some((_, ResourceOutcome::Value(v)))) if !v.is_empty() => {
            tracing::debug!(event = "update_homeurl_copied", homeurl = %v, "[update] SHIORI の homeurl を写しました");
            desk.ghost_homeurl = Some(v);
        }
        Ok(Some((_, ResourceOutcome::Failed(reason)))) | Err(reason) => tracing::warn!(
            event = "update_homeurl_query_failed",
            reason = %reason,
            "[update] SHIORI の homeurl を照会できませんでした（写しは空のまま）"
        ),
        Ok(_) => tracing::debug!(
            event = "update_homeurl_absent",
            "[update] SHIORI は homeurl を答えませんでした（写しは空のまま）"
        ),
    }
}

/// 更新の実行中か（窓口が要求を預かっている間＝段が `Idle` でないか、読み直しの切替の後送りを
/// 持っている）。シェル・バルーンの切替の入口が断るのに使う（shell-balloon-switch 要件 1.13）。
pub(crate) fn is_busy(world: &World) -> bool {
    world
        .get_non_send::<UpdateDesk>()
        .is_some_and(|desk| desk.stage != Stage::Idle || desk.after_switch.is_some())
}

/// 標準の手続きが走っている窓口を据える（テスト用・背景スレッドは起こさない）。
#[cfg(test)]
pub(crate) fn insert_running_desk_for_test(world: &mut World) {
    let mut desk = UpdateDesk::new(Arc::new(WorkGate::default()));
    desk.stage = Stage::Running;
    world.insert_non_send(desk);
}

/// 背景スレッドと同じ道（窓口の頼みの送出端）で、`ghost_dir` の読み直しを列つきで頼む（テスト用・
/// 捌くのは次の取り出しの系）。
#[cfg(test)]
pub(crate) fn ask_reload_for_test(world: &World, ghost_dir: PathBuf, tail: Tail) {
    world
        .non_send::<UpdateDesk>()
        .asks_tx
        .send(DeskAsk::Reload { ghost_dir, tail })
        .expect("窓口の頼みの受信端は生きている");
}

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
pub(crate) fn resolve_targets(world: &World, raw: &RawUpdateRequest) -> Vec<TargetSpec> {
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
#[path = "desk_reload_tests.rs"]
mod reload_tests;

#[cfg(test)]
#[path = "desk_tests.rs"]
mod tests;
