//! UI 側の窓口（design「areka / install / desk」）。
//!
//! World に触る仕事を 1 か所に集める。窓口 [`InstallDesk`] は World の NonSend・プロセスに 1 つで、
//! ゴーストを起こし直しても作り直さない。取り出しの系 [`drain`] が毎 tick ⑴ 入口（台本の受け口・
//! 選ぶ画面のスレッド）からの生の要求を受付 [`submit`] へ流し ⑵ 背景のスレッドの頼み [`DeskAsk`] を
//! 捌き ⑶ 手元のイベントの頼み（高々 1 件）を条件を満たせば送り ⑷ 背景のスレッドが空いていれば
//! 次の依頼を渡す。入れた後の記録（受け皿・バルーンの記憶・置換語）は反映を待ってから答える。
//!
//! イベントを送るのは、切替の予約が無い・終了が始まっていない・置き場のゴーストに kanade への
//! 送出端がある tick で、送り先は送る時点の置き場のゴースト。定常かどうかの正本は kanade で、
//! 窓口は旗を持たず定常到達の回数だけを数える: 送り直しの頼みは、前に送った後に定常到達が
//! 届いているときだけ送る（設計で決めたこと 5）。起床の旗は立てない（設計で決めたこと 16）。

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SendError, Sender};

use areka_actor::ReplySender;
use areka_ghost::catalog::{list_balloons, list_ghosts};
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod};
use areka_nar::InstallKind;
use areka_sylphya::{PersistKey, PersistScope};
use bevy_ecs::world::World;

use super::judge::GhostFacts;
use super::names;
use super::procedure::{InstalledRecord, Overwritten};
use super::worker::{DeskAsk, spawn_worker};
use super::{InstallOrder, RawInstallRequest, submit};
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{SwitchInFlight, record_last_installed};
use crate::exit_wait::WorkGate;
use crate::ghost_session::GhostSlot;

/// 手元に置いたイベントの頼み（送る条件がそろうまで待つ）。
struct HeldRaise {
    id: &'static str,
    references: Vec<String>,
    resend: bool,
    reply: ReplySender<RaiseOutcome>,
}

/// UI 側の窓口（World の NonSend・プロセスに 1 つ）。
pub(crate) struct InstallDesk {
    /// 入口へ配る生の要求の送出端と、その受信端。
    raw_tx: Sender<RawInstallRequest>,
    raw_rx: Receiver<RawInstallRequest>,
    /// 待っている依頼（届いた順）。
    pub(super) queue: VecDeque<InstallOrder>,
    /// 背景のスレッドへの依頼の送出端（最初の依頼で 1 度だけ起こす）。
    worker: Option<Sender<InstallOrder>>,
    /// 背景のスレッドが依頼を扱っている最中か。
    pub(super) busy: bool,
    /// 背景のスレッドの頼みの送出端（スレッドへ渡す）と、その受信端。
    asks_tx: Sender<DeskAsk>,
    asks_rx: Receiver<DeskAsk>,
    /// 手元に置いたイベントの頼み（高々 1 件）。
    held: Option<HeldRaise>,
    /// 届いた定常到達の回数。
    steady_count: u64,
    /// 最後にイベントを送ったときの定常到達の回数（まだ送っていなければ None）。
    sent_at_steady: Option<u64>,
    /// 終了の待ちへ登記した門（背景のスレッドと共有）。
    pub(super) gate: Arc<WorkGate>,
}

impl InstallDesk {
    pub(super) fn new(gate: Arc<WorkGate>) -> Self {
        let (raw_tx, raw_rx) = mpsc::channel();
        let (asks_tx, asks_rx) = mpsc::channel();
        InstallDesk {
            raw_tx,
            raw_rx,
            queue: VecDeque::new(),
            worker: None,
            busy: false,
            asks_tx,
            asks_rx,
            held: None,
            steady_count: 0,
            sent_at_steady: None,
            gate,
        }
    }
}

/// 取り出しの系（Input の段・`dispatch_pointer_events` の後）。窓口が無ければ無操作。
pub(crate) fn drain(world: &mut World) {
    let Some(desk) = world.get_non_send::<InstallDesk>() else {
        return;
    };
    let raws: Vec<RawInstallRequest> = desk.raw_rx.try_iter().collect();
    let asks: Vec<DeskAsk> = desk.asks_rx.try_iter().collect();
    for raw in raws {
        submit(
            world,
            InstallOrder {
                archives: vec![raw.path],
                origin: raw.origin,
            },
        );
    }
    for ask in asks {
        answer(world, ask);
    }
    send_held(world);
    hand_next_order(world);
}

/// 定常到達の通知を受けた（`ghost_switch::on_notice` の定常到達の腕の末尾から）。回数を数えるだけ。
pub(crate) fn on_steady(world: &mut World) {
    if let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() {
        desk.steady_count += 1;
    }
}

/// 台本の受け口と選ぶ画面のスレッドへ配る送出端（窓口が無ければ受信端の無い送出端）。
// 配り先（台本の受け口・選ぶ画面）は 7.1・7.2 で結ぶ。結んだら外す。
#[allow(dead_code)]
pub(crate) fn raw_sender(world: &World) -> Sender<RawInstallRequest> {
    match world.get_non_send::<InstallDesk>() {
        Some(desk) => desk.raw_tx.clone(),
        None => mpsc::channel().0,
    }
}

/// 終了が始まった（`exit_wait::begin_close` が呼ぶ）: 待っている依頼と手元のイベントの頼みを捨てて
/// 記録に残す（頼みの返信端を落とすので、背景のスレッドは閉じた扱いで止まる）。
pub(crate) fn discard_for_exit(world: &mut World) {
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let orders: Vec<InstallOrder> = desk.queue.drain(..).collect();
    let held = desk.held.take();
    if orders.is_empty() && held.is_none() {
        return;
    }
    let paths: Vec<_> = orders.iter().flat_map(|o| o.archives.iter()).collect();
    tracing::warn!(
        event = "install_pending_discarded",
        count = orders.len(),
        paths = ?paths,
        held = held.map(|h| h.id),
        "[install] 終了が始まったので、待っている依頼と手元のイベントの頼みを捨てます"
    );
}

/// 背景のスレッドの頼みを 1 件捌く。終了が始まった後は返信端をそのまま落とす。
fn answer(world: &mut World, ask: DeskAsk) {
    let closing = world.non_send::<InstallDesk>().gate.is_closing();
    match ask {
        DeskAsk::OrderDone => world.non_send_mut::<InstallDesk>().busy = false,
        ask if closing => {
            drop(ask);
            tracing::debug!(
                event = "install_ask_dropped",
                "[install] 終了が始まっているので、背景のスレッドの頼みに答えません"
            );
        }
        DeskAsk::Raise {
            id,
            references,
            resend,
            reply,
        } => {
            let new = HeldRaise {
                id,
                references,
                resend,
                reply,
            };
            if let Some(old) = world.non_send_mut::<InstallDesk>().held.replace(new) {
                // 背景のスレッドは返事を待って止まるので 2 件目は来ない。来たら古い方を閉じる。
                tracing::warn!(
                    event = "install_raise_replaced",
                    old = old.id,
                    new = id,
                    "[install] 手元のイベントの頼みが 2 件になったので、古い方を閉じます"
                );
            }
        }
        DeskAsk::Facts { reply } => {
            let facts = ghost_facts(world);
            if facts.is_none() {
                tracing::debug!(
                    event = "install_facts_none",
                    "[install] 起動の文脈か置き場のゴーストが無いので、素性は無しと答えます"
                );
            }
            reply_to(reply, facts);
        }
        // 起動中のゴーストへ入れる一周は 8.1。それまでは書庫を返し、よそへの展開へ進ませる。
        DeskAsk::Overwrite {
            archive,
            target_ghost,
            reply,
        } => {
            tracing::debug!(
                event = "install_overwrite_returned",
                target_ghost = ?target_ghost,
                "[install] 起動中のゴーストへ入れる一周はまだ無いので、書庫を返します"
            );
            reply_to(reply, Overwritten::NotRunning(archive));
        }
        DeskAsk::Record { record, reply } => {
            tracing::debug!(
                event = "install_record_received",
                record = ?record,
                "[install] 入れた後の記録を受けました"
            );
            record_installed(world, record);
            reply_to(reply, ());
        }
    }
}

/// 入れた後の記録（要件 6.1・6.6・6.7・6.8・設計で決めたこと 14）: `ghost` は受け皿へ、`balloon`
/// だけなら今のゴーストの「最後に使ったバルーン」の記憶へフォルダ名を書き、シェル・追加ファイルは
/// 置換語だけ（表示は替えない）。フォルダ名と名前は入れた後の目録の綴り（`areka-nar` の綴りと
/// 大文字小文字を無視して突き合わせる・目録に無ければ `areka-nar` の綴りと `install.txt` の `name`）。
/// 最後に今のゴーストの記憶の書き手の反映を待つ（背景のスレッドへ答えるのはその後）。
fn record_installed(world: &mut World, record: InstalledRecord) {
    let root = world
        .get_resource::<BootContext>()
        .map(|ctx| ctx.root.clone());
    let ghosts = match (&root, record.kind) {
        (Some(root), InstallKind::Ghost | InstallKind::Shell | InstallKind::Supplement) => {
            list_ghosts(root)
        }
        _ => Vec::new(),
    };
    let ghost_entry = |folder: &str| {
        ghosts
            .iter()
            .find(|entry| entry.identity.folder.eq_ignore_ascii_case(folder))
    };
    let ghost_name = record.ghost_folder.as_deref().map(|folder| {
        match ghost_entry(folder).and_then(|entry| entry.identity.name.clone()) {
            Some(name) => name,
            None if record.kind == InstallKind::Ghost => record.object_name.clone(),
            None => folder.to_owned(),
        }
    });
    match record.kind {
        InstallKind::Ghost => {
            let folder = ghost_entry(&record.folder)
                .map_or_else(|| record.folder.clone(), |e| e.identity.folder.clone());
            record_last_installed(world, folder);
        }
        InstallKind::Balloon => {
            let balloons = root.as_ref().map(list_balloons).unwrap_or_default();
            let folder = balloons
                .iter()
                .find(|entry| entry.identity.folder.eq_ignore_ascii_case(&record.folder))
                .map_or_else(|| record.folder.clone(), |e| e.identity.folder.clone());
            remember_balloon(world, folder);
        }
        InstallKind::Shell | InstallKind::Supplement => {}
    }
    names::update(world, record.object_name, ghost_name);
    if let Some(runtime) = names::current_runtime(world)
        && let Err(err) = runtime.sylphya_publisher().barrier()
    {
        tracing::error!(
            event = "install_record_unsettled",
            error = %err,
            "[install] 今のゴーストの記憶の書き手が止まっているので、入れた後の記録の反映を確かめられません"
        );
    }
}

/// 今のゴーストの「最後に使ったバルーン」の記憶を書き換える（次に起こしたときから使われる＝要件 6.6）。
fn remember_balloon(world: &World, folder: String) {
    let Some(runtime) = names::current_runtime(world) else {
        tracing::warn!(
            event = "install_balloon_not_remembered",
            folder = %folder,
            "[install] 今のゴーストが居ないので、入れたバルーンを記憶へ書けません"
        );
        return;
    };
    runtime.sylphya_publisher().persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::LastBalloon, folder.clone())],
    );
    tracing::info!(
        event = "install_balloon_remembered",
        folder = %folder,
        "[install] 今のゴーストの「最後に使ったバルーン」を入れたバルーンへ書き換えました"
    );
}

/// 背景のスレッドへ答える。スレッドが居なければ記録だけ残す。
fn reply_to<T: Send>(reply: ReplySender<T>, value: T) {
    if reply.send(value).is_err() {
        tracing::debug!(
            event = "install_reply_unread",
            "[install] 背景のスレッドが答えを待っていません"
        );
    }
}

/// 今のゴーストの素性（起動の文脈と置き場のゴーストから引く・どちらかが無ければ None）。
fn ghost_facts(world: &World) -> Option<GhostFacts> {
    let ctx = world.get_resource::<BootContext>()?;
    let session = world.get_non_send::<GhostSlot>()?.0.as_ref()?;
    let dir = session.ghost_dir();
    let folder = ctx.current.ghost.folder.clone();
    let name = session
        .names()
        .and_then(|n| n.name.clone())
        .or_else(|| folder.clone())
        .unwrap_or_else(|| dir.display().to_string());
    Some(GhostFacts {
        root: ctx.root.dir().to_path_buf(),
        folder,
        name,
        sakura_name: areka_ghost::catalog::sakura_name(dir),
        install_accept: areka_ghost::catalog::install_accept(dir),
    })
}

/// 手元のイベントの頼みを、条件がそろっていれば送る時点の置き場のゴーストへ GET で送る。
fn send_held(world: &mut World) {
    let switching = world.get_non_send::<SwitchInFlight>().is_some();
    let kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|s| s.kanade().cloned());
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let Some(kanade) = kanade else {
        return;
    };
    if switching || desk.gate.is_closing() {
        return;
    }
    let steady = desk.steady_count;
    let waiting_steady = desk.sent_at_steady == Some(steady);
    let Some(held) = desk.held.take_if(|h| !(h.resend && waiting_steady)) else {
        return;
    };
    desk.sent_at_steady = Some(steady);
    let (id, resend) = (held.id, held.resend);
    let msg = KanadeMsg::RaiseEvent {
        id: id.to_owned(),
        references: held.references,
        method: ShioriMethod::Get,
        reply: Some(held.reply),
    };
    // 送れなければ返信端は落ちるので、背景のスレッドは閉じた扱いで止まる。
    match kanade.send(msg) {
        Ok(()) => tracing::debug!(
            event = "install_event_sent",
            id,
            resend,
            "[install] イベントを今のゴーストへ送りました"
        ),
        Err(_) => tracing::warn!(
            event = "install_event_send_failed",
            id,
            "[install] kanade が止まっているので、イベントを送れません"
        ),
    }
}

/// 背景のスレッドが空いていれば、待っている依頼の先頭を渡す（最初の依頼でスレッドを起こす）。
fn hand_next_order(world: &mut World) {
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let desk = &mut *desk;
    if desk.busy || desk.gate.is_closing() {
        return;
    }
    let Some(order) = desk.queue.pop_front() else {
        return;
    };
    let worker = desk.worker.get_or_insert_with(|| {
        tracing::debug!(
            event = "install_worker_spawned",
            "[install] 背景のスレッドを起こします"
        );
        // 取っ手は持たない（落としても join しない）。終了で待つのは門の役目。
        spawn_worker(desk.asks_tx.clone(), desk.gate.clone()).0
    });
    match worker.send(order) {
        Ok(()) => desk.busy = true,
        Err(SendError(order)) => tracing::error!(
            event = "install_worker_gone",
            archives = ?order.archives,
            "[install] 背景のスレッドが居ないので、依頼を扱えません"
        ),
    }
}

#[cfg(test)]
#[path = "desk_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "desk_record_tests.rs"]
mod record_tests;
