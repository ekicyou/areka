//! UI 側の窓口。
//!
//! World に触る仕事を 1 か所に集める。窓口 [`InstallDesk`] は World の NonSend・プロセスに 1 つで、
//! ゴーストを起こし直しても作り直さない。取り出しの系 [`drain`] が毎 tick ⑴ 入口（台本の受け口・
//! 選ぶ画面と取得のスレッド）からの生の要求のうち、パスは受付 [`submit`] へ流し、URL は取得の待ち行列へ
//! 積み ⑵ 背景のスレッドの頼み [`DeskAsk`] を捌き ⑶ 手元のイベントの頼み（高々 1 件）を条件を満たせば
//! 送り ⑷ 背景のスレッドが空いていれば次の依頼を、取得のスレッドが空いていれば次の URL を渡し
//! ⑸ 預かった書庫の段を切替の予約の有無で見直す。入れた後の記録（受け皿・バルーンの
//! 記憶・置換語）は反映を待ってから答える。
//! 起動中のゴーストへ入れる一周（預かった書庫と段）は子の `overwrite` が持ち、ここは切替の道筋から
//! 呼ばれる口 [`run_overwrite_between`]・[`on_steady`] を開けるだけ。
//!
//! イベントを送るのは、切替の予約が無い・終了が始まっていない・置き場のゴーストに kanade への
//! 送出端がある tick で、送り先は送る時点の置き場のゴースト。定常かどうかの正本は kanade で、
//! 窓口は旗を持たず定常到達の回数だけを数える: 送り直しの頼みは、前に送った後に定常到達が
//! 届いているときだけ送る。起床の旗は立てない。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::{self, JoinHandle};

use areka_actor::ReplySender;
use areka_ghost::catalog_roots::{balloons_for, find_balloon, find_ghost, same_folder_name};
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod};
use areka_nar::InstallKind;
use areka_sylphya::{PersistKey, PersistScope, SylphyaPublisher};
use bevy_ecs::world::World;

use super::fetch_url::{self, FetchTools};
use super::judge::GhostFacts;
use super::names;
use super::pick::{self, PickError};
use super::procedure::InstalledRecord;
use super::startup::CarriedInstall;
use super::worker::{DeskAsk, spawn_worker};
use super::{InstallOrder, InstallOrigin, RawInstallRequest, submit};
use areka_runtime::shell_balloon_resolve::{record_installed_balloon, record_installed_shell};
use areka_session::boot_config::BootContext;
use areka_session::exit_wait::{EXIT_WAIT_LIMIT, WorkGate};
use areka_session::ghost_session::GhostSlot;
use areka_session::ghost_switch::{SwitchInFlight, record_last_installed};

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
    /// 選ぶ画面が出ているか（選ぶ画面のスレッドと共有・スレッドが終わると降りる）。
    picking: Arc<AtomicBool>,
    /// 起動中のゴーストへ入れる、預かった書庫と段（高々 1 件・中身は [`overwrite`]）。
    overwrite: Option<overwrite::Pending>,
    /// 取得を待っている URL（届いた順）。
    fetch_queue: VecDeque<String>,
    /// 取得のスレッドへの URL の送出端（最初の URL で起こす・倒れたら捨てて起こし直す）。
    fetch_tx: Option<Sender<String>>,
    /// 取得のスレッドが URL を扱っている最中か。
    fetching: bool,
    /// 終了の待ちへ登記した取得の門（取得のスレッドと共有）。
    fetch_gate: Arc<WorkGate>,
    fetch_tools: FetchTools,
}

impl InstallDesk {
    pub(super) fn new(gate: Arc<WorkGate>, fetch_gate: Arc<WorkGate>) -> Self {
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
            picking: Arc::new(AtomicBool::new(false)),
            overwrite: None,
            fetch_queue: VecDeque::new(),
            fetch_tx: None,
            fetching: false,
            fetch_gate,
            fetch_tools: FetchTools::real(),
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
        match raw {
            RawInstallRequest::Path { path, origin } => {
                submit(
                    world,
                    InstallOrder {
                        archives: vec![path],
                        origin,
                    },
                );
            }
            RawInstallRequest::Url(url) => queue_fetch(world, url),
        }
    }
    for ask in asks {
        answer(world, ask);
    }
    send_held(world);
    hand_next_order(world);
    hand_next_fetch(world);
    overwrite::on_tick(world);
}

/// 定常到達の通知を受けた（`ghost_switch::on_notice` の定常到達の腕の末尾から）。回数を数え、
/// 預かった書庫の段を進める（展開した結果を返す・預かったままなら切替を頼み直す）。
pub(crate) fn on_steady(world: &mut World) {
    if let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() {
        desk.steady_count += 1;
    }
    overwrite::on_steady(world);
}

/// 降ろして全窓を閉じた直後・起こす前（`ghost_switch::switch_to` から）。預かった宛先への切替の
/// ときだけ UI スレッドで同期に展開する（それ以外は無操作）。
pub(crate) fn run_overwrite_between(world: &mut World) {
    overwrite::run_between(world);
}

/// 台本の受け口と選ぶ画面のスレッドへ配る送出端（窓口が無ければ受信端の無い送出端）。
pub(crate) fn raw_sender(world: &World) -> Sender<RawInstallRequest> {
    match world.get_non_send::<InstallDesk>() {
        Some(desk) => desk.raw_tx.clone(),
        None => mpsc::channel().0,
    }
}

/// インストールの窓口が忙しいか（背景のスレッドが依頼を扱っている・待っている依頼が在る・起動中の
/// ゴーストへ入れる書庫を預かっている）。読み直しの入口が断るのに使う。
pub(crate) fn is_busy(world: &World) -> bool {
    world
        .get_non_send::<InstallDesk>()
        .is_some_and(|desk| desk.busy || !desk.queue.is_empty() || desk.overwrite.is_some())
}

/// 依頼を扱っている最中の窓口を据える（テスト用・背景スレッドは起こさない）。
#[cfg(test)]
pub(crate) fn insert_busy_desk_for_test(world: &mut World) {
    let mut desk = InstallDesk::new(Arc::new(WorkGate::default()), Arc::new(WorkGate::default()));
    desk.busy = true;
    world.insert_non_send(desk);
}

/// 取得のスレッドの道具を差し替える（テスト用・偽の作り口と一時フォルダを差す）。
#[cfg(test)]
pub(crate) fn set_fetch_tools_for_test(world: &mut World, tools: FetchTools) {
    world.non_send_mut::<InstallDesk>().fetch_tools = tools;
}

/// メニュー「インストール…」を選べるか（窓口が在り・終了が始まっておらず・選ぶ画面が出ていない）。
pub(crate) fn can_pick(world: &World) -> bool {
    world
        .get_non_send::<InstallDesk>()
        .is_some_and(|desk| !desk.gate.is_closing() && !desk.picking.load(Ordering::Acquire))
}

/// メニューの動作: 選ぶ画面のスレッド `install-pick` を起こす。出ている最中なら
/// `debug!` で無視し、`AREKA_NO_ALERT` で抑止されていればスレッドを起こさず取り消しと同じに扱う。
pub(crate) fn pick_and_submit(world: &mut World) {
    start_pick(
        world,
        areka_session::alert::suppressed(),
        pick::pick_archive,
    );
}

/// [`pick_and_submit`] の中身（テストは抑止の判定と選ぶ画面の代わりを差し込む）。起こしたスレッドの
/// 取っ手を返す（本番は持たない）。
fn start_pick<F>(world: &mut World, suppressed: bool, pick: F) -> Option<JoinHandle<()>>
where
    F: FnOnce() -> Result<Option<PathBuf>, PickError> + Send + 'static,
{
    let Some(desk) = world.get_non_send::<InstallDesk>() else {
        tracing::debug!(
            event = "install_pick_no_desk",
            "[install] 窓口が無いので、ファイルを選ぶ画面を出しません"
        );
        return None;
    };
    if desk.picking.load(Ordering::Acquire) {
        tracing::debug!(
            event = "install_pick_busy",
            "[install] ファイルを選ぶ画面が出ているので、2 つ目は出しません"
        );
        return None;
    }
    if suppressed {
        tracing::warn!(
            event = "install_pick_suppressed",
            "[install] 告知が抑止されているので、ファイルを選ぶ画面を出さずに取り消しと同じに扱います"
        );
        return None;
    }
    desk.picking.store(true, Ordering::Release);
    // 旗はスレッドが終わるとき（panic も含む）と、スレッドを起こせずに閉包が落ちたときに降りる。
    let flag = PickingFlag(desk.picking.clone());
    let tx = desk.raw_tx.clone();
    let spawned = thread::Builder::new()
        .name("install-pick".to_owned())
        .spawn(move || {
            let _flag = flag;
            send_pick(pick(), &tx);
        });
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::error!(
                event = "install_pick_failed",
                error = %err,
                "[install] ファイルを選ぶ画面のスレッドを起こせません"
            );
            None
        }
    }
}

/// 選ぶ画面が出ている旗（落ちると降りる・置き場を選ぶ画面の受け口 `roots::desk` も使い回す）。
pub(crate) struct PickingFlag(pub(crate) Arc<AtomicBool>);

impl Drop for PickingFlag {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// 選ぶ画面の結果を窓口へ流す（選ぶ画面のスレッド）。選ばれたら出どころ「メニュー」の生の要求 1 件、
/// 取り消しは `info!`、出せなければ `error!` でどちらも 0 件。
fn send_pick(result: Result<Option<PathBuf>, PickError>, tx: &Sender<RawInstallRequest>) {
    match result {
        Ok(Some(path)) => {
            let shown = path.display().to_string();
            let request = RawInstallRequest::Path {
                path,
                origin: InstallOrigin::Menu,
            };
            if tx.send(request).is_err() {
                tracing::warn!(
                    event = "install_pick_send_failed",
                    path = shown,
                    "[install] 選ばれた書庫を窓口へ送れません（窓口が無い）"
                );
            }
        }
        Ok(None) => tracing::info!(
            event = "install_pick_cancelled",
            "[install] ファイルを選ぶ画面が取り消されました"
        ),
        Err(err) => tracing::error!(
            event = "install_pick_failed",
            error = ?err,
            "[install] ファイルを選ぶ画面を出せません"
        ),
    }
}

/// 終了が始まった（`exit_wait::begin_close` が呼ぶ）: 待っている依頼・手元のイベントの頼み・預かった
/// 書庫を捨てて記録に残す（頼みの返信端を落とすので、背景のスレッドは閉じた扱いで止まる）。件数は
/// まだ始めていない捨てた書庫の数。取得を待っている URL は 1 件ずつ記録して捨て、取得のスレッドへの
/// 送出端を落とす（スレッドは今の取得を終えると抜ける）。
pub(crate) fn discard_for_exit(world: &mut World) {
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    desk.fetch_tx = None;
    for url in desk.fetch_queue.drain(..) {
        record_fetch_abandoned(&url);
    }
    let paths: Vec<PathBuf> = desk.queue.drain(..).flat_map(|o| o.archives).collect();
    let held = desk.held.take();
    let overwrite = overwrite::discard(&mut desk);
    if paths.is_empty() && held.is_none() && overwrite.is_none() {
        return;
    }
    tracing::warn!(
        event = "install_pending_discarded",
        count = paths.len(),
        paths = ?paths,
        held = held.map(|h| h.id),
        overwrite = ?overwrite,
        "[install] 終了が始まったので、待っている依頼・手元のイベントの頼み・預かった書庫を捨てます"
    );
}

/// 背景のスレッドの頼みを 1 件捌く。終了が始まった後は返信端をそのまま落とす。
fn answer(world: &mut World, ask: DeskAsk) {
    let closing = world.non_send::<InstallDesk>().gate.is_closing();
    match ask {
        DeskAsk::OrderDone => {
            world.non_send_mut::<InstallDesk>().busy = false;
        }
        DeskAsk::WorkerPanicked => {
            let mut desk = world.non_send_mut::<InstallDesk>();
            desk.busy = false;
            desk.worker = None;
        }
        DeskAsk::FetchDone { panicked } => {
            let mut desk = world.non_send_mut::<InstallDesk>();
            desk.fetching = false;
            if panicked {
                desk.fetch_tx = None;
            }
        }
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
        DeskAsk::Overwrite {
            archive,
            target_ghost,
            reply,
        } => overwrite::take(world, archive, target_ghost, reply),
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

/// 入れた後の記録: `ghost` は受け皿へ、`balloon` だけなら今のゴーストの「最後に使ったバルーン」の
/// 記憶へフォルダ名を書き、シェル・追加ファイルは置換語だけ（表示は替えない）。シェルとバルーンは
/// `lastinstalled` の控えも置く。フォルダ名と名前は入れた後の和の目録の綴り（`areka-nar` の綴りと
/// フォルダ名の 1 つの規則で突き合わせる・目録に無ければ `areka-nar` の綴りと `install.txt` の `name`）。
/// 最後に今のゴーストの記憶の書き手の反映を上限まで待つ（背景のスレッドへ答えるのはその後）。
pub(super) fn record_installed(world: &mut World, record: InstalledRecord) {
    let roots = world
        .get_resource::<BootContext>()
        .map(|ctx| ctx.roots.clone());
    let (folder, ghost_name) = catalog_spelling(roots.as_ref(), &record);
    if record.kind == InstallKind::Balloon {
        remember_balloon(world, folder.clone());
        note_hidden_by_ghost(world, &record.folder);
    }
    place_last_installed(
        world,
        CarriedInstall {
            kind: record.kind,
            folder,
            ghost_folder: record.ghost_folder,
            object_name: record.object_name,
            ghost_name,
        },
    );
    if let Some(runtime) = names::current_runtime(world) {
        settle_record(runtime.sylphya_publisher());
    }
}

/// 今のゴーストの記憶の書き手の反映を、上限まで待つ。待てなければ error を 1 件残して続ける。
pub(super) fn settle_record(publisher: &SylphyaPublisher) {
    if let Err(err) = publisher.barrier_within(EXIT_WAIT_LIMIT) {
        tracing::error!(
            event = "install_record_unsettled",
            error = %err,
            "[install] 今のゴーストの記憶の書き手が止まっているか遅れているので、入れた後の記録の反映を確かめないまま続けます"
        );
    }
}

/// 「最後に入れた物」の受け皿（ゴースト・シェル・バルーン・追加ファイルは無し）と置換語へ置く
/// （窓口の記録と起動の前の控えの引き継ぎの共通の口・記憶へは書かない）。
pub(super) fn place_last_installed(world: &mut World, installed: CarriedInstall) {
    match installed.kind {
        InstallKind::Ghost => record_last_installed(world, installed.folder),
        InstallKind::Balloon => record_installed_balloon(world, installed.folder),
        InstallKind::Shell => {
            record_installed_shell(world, installed.ghost_folder, installed.folder)
        }
        InstallKind::Supplement => {}
    }
    names::update(world, installed.object_name, installed.ghost_name);
}

/// 目録の綴りに直したフォルダ名と、%lastghostname の値（窓口と起動の前の口の共通の決まり）。
/// フォルダ名はゴースト・バルーンなら根の目録の綴り（目録に無ければ書庫の綴りのまま）、他は書庫の
/// 綴りのまま。%lastghostname は関わったゴーストの目録の `name`（無ければゴーストは `install.txt` の
/// `name`・他はそのフォルダ名）で、関わったゴーストが無ければ None。
pub(super) fn catalog_spelling(
    roots: Option<&areka_ghost::BasewareRoots>,
    record: &InstalledRecord,
) -> (String, Option<String>) {
    let ghost_entry = |folder: &str| roots.and_then(|roots| find_ghost(roots, folder));
    let ghost_name = record.ghost_folder.as_deref().map(|folder| {
        match ghost_entry(folder).and_then(|entry| entry.identity.name) {
            Some(name) => name,
            None if record.kind == InstallKind::Ghost => record.object_name.clone(),
            None => folder.to_owned(),
        }
    });
    let folder = match record.kind {
        InstallKind::Ghost => ghost_entry(&record.folder).map(|e| e.identity.folder),
        InstallKind::Balloon => {
            roots.and_then(|roots| find_balloon(roots, &record.folder).map(|e| e.identity.folder))
        }
        InstallKind::Shell | InstallKind::Supplement => None,
    }
    .unwrap_or_else(|| record.folder.clone());
    (folder, ghost_name)
}

/// バルーンだけの書庫を根へ入れた後、今のゴーストの中の同じフォルダ名に隠れるなら info
/// `installed_balloon_hidden_by_ghost` を 1 件残す。
/// 起動の文脈が無いときは何もしない。
fn note_hidden_by_ghost(world: &World, folder: &str) {
    let Some(ctx) = world.get_resource::<BootContext>() else {
        return;
    };
    let hidden = balloons_for(&ctx.roots, &ctx.current.ghost.dir).hidden;
    if let Some(h) = hidden.iter().find(|h| same_folder_name(&h.folder, folder)) {
        tracing::info!(
            event = "installed_balloon_hidden_by_ghost",
            folder = h.folder.as_str(),
            installed = %h.hidden.display(),
            by = %h.by.display(),
            "[install] 入れたバルーンは今のゴーストの中の同じフォルダ名に隠れます"
        );
    }
}

/// 今のゴーストの「最後に使ったバルーン」の記憶を書き換える（次に起こしたときから使われる）。
pub(crate) fn remember_balloon(world: &World, folder: String) {
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
        "[install] 今のゴーストの「最後に使ったバルーン」を入れたバルーンへ書き換えるように頼みました"
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
        roots: ctx.roots.clone(),
        running_dir: folder.as_ref().map(|_| dir.to_path_buf()),
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
        Err(SendError(order)) => {
            desk.worker = None;
            tracing::error!(
                event = "install_worker_gone",
                archives = ?order.archives,
                "[install] 背景のスレッドが居ないので、依頼を扱えません"
            );
        }
    }
}

/// URL を取得の待ち行列の末尾へ積む。終了が始まっていれば積まずに記録だけ残す。
fn queue_fetch(world: &mut World, url: String) {
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    if desk.gate.is_closing() {
        return record_fetch_abandoned(&url);
    }
    desk.fetch_queue.push_back(url);
}

/// 取得のスレッドが空いていれば、待っている URL の先頭を渡す。スレッドが居なければ起こし直し、
/// 起こせなければその URL を捨てて次へ進む。
fn hand_next_fetch(world: &mut World) {
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let desk = &mut *desk;
    while !desk.fetching && !desk.gate.is_closing() {
        let Some(url) = desk.fetch_queue.pop_front() else {
            return;
        };
        let url = match &desk.fetch_tx {
            Some(tx) => match tx.send(url) {
                Ok(()) => {
                    desk.fetching = true;
                    continue;
                }
                Err(SendError(url)) => url,
            },
            None => url,
        };
        desk.fetch_tx = fetch_url::spawn_fetcher(
            url,
            desk.fetch_tools.clone(),
            desk.fetch_gate.clone(),
            desk.raw_tx.clone(),
            desk.asks_tx.clone(),
        );
        desk.fetching = desk.fetch_tx.is_some();
    }
}

fn record_fetch_abandoned(url: &str) {
    tracing::warn!(
        event = "install_fetch_abandoned",
        url,
        "[install] 終了が始まったので、取得を待っている URL を捨てます"
    );
}

#[path = "overwrite.rs"]
mod overwrite;

#[cfg(test)]
#[path = "desk_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "desk_record_tests.rs"]
mod record_tests;

#[cfg(test)]
#[path = "desk_pick_tests.rs"]
mod pick_tests;

#[cfg(test)]
#[path = "desk_overwrite_tests.rs"]
mod overwrite_tests;

#[cfg(test)]
#[path = "desk_worker_gone_tests.rs"]
mod worker_gone_tests;

#[cfg(test)]
#[path = "fetch_queue_tests.rs"]
mod fetch_queue_tests;
