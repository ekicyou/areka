//! 置き場にするフォルダを選ぶ画面の受け口（design「RootsDesk」・要件 6.3・6.5・6.9）。
//!
//! 選ぶ画面は短命のスレッド `roots-pick` で出し（UI スレッドを塞がない＝要件 6.3）、決めたフォルダ
//! だけを受け口へ送る。取り消しと失敗は選ぶスレッドで記録して送らない。UI スレッドの系 [`drain`] が
//! 受けて [`super::add_root`] を呼ぶ。2 つ目の画面を出さない旗はインストールの窓口の
//! [`PickingFlag`] を使い回す。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use crate::install::desk::PickingFlag;
use crate::install::pick::PickError;

/// 選ぶスレッドから受け口へ送るもの（決めたフォルダだけ）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PickedFolder {
    Chosen(PathBuf),
}

/// 受け口（World の NonSend・プロセスに 1 つ）。
pub(crate) struct RootsDesk {
    tx: Sender<PickedFolder>,
    rx: Receiver<PickedFolder>,
    /// 選ぶ画面が出ているか（選ぶスレッドと共有・スレッドが終わると降りる）。
    picking: Arc<AtomicBool>,
}

/// 受け口を据え、受ける系を Input の段（`dispatch_pointer_events` の後）へ足す（`main.rs` が系の
/// 登録の後に 1 回）。
pub(crate) fn install(world: &mut World) {
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(RootsDesk {
        tx,
        rx,
        picking: Arc::new(AtomicBool::new(false)),
    });
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain.after(dispatch_pointer_events));
}

/// 受ける系（UI スレッド）: 届いたフォルダを順に追加の入口へ渡す。受け口が無ければ無操作。
pub(crate) fn drain(world: &mut World) {
    let Some(desk) = world.get_non_send::<RootsDesk>() else {
        return;
    };
    let picked: Vec<PickedFolder> = desk.rx.try_iter().collect();
    for PickedFolder::Chosen(dir) in picked {
        super::add_root(world, dir);
    }
}

/// 「置き場を追加…」を選べるか（受け口が在り・選ぶ画面が出ていない）。
pub(crate) fn can_pick(world: &World) -> bool {
    world
        .get_non_send::<RootsDesk>()
        .is_some_and(|desk| !desk.picking.load(Ordering::Acquire))
}

/// 選ぶ画面のスレッド `roots-pick` を起こす（メニューの動作・テストは抑止と画面の代わりを差し込む）。
/// 出ている最中なら `debug!` で無視し、抑止のときは出さずに取り消しと同じに扱う。起こしたスレッドの
/// 取っ手を返す（本番は持たない）。
pub(crate) fn start_pick<F>(world: &mut World, suppressed: bool, pick: F) -> Option<JoinHandle<()>>
where
    F: FnOnce() -> Result<Option<PathBuf>, PickError> + Send + 'static,
{
    let Some(desk) = world.get_non_send::<RootsDesk>() else {
        tracing::debug!(
            event = "roots_pick_no_desk",
            "[RootsDesk] 受け口が無いので、フォルダを選ぶ画面を出しません"
        );
        return None;
    };
    if desk.picking.load(Ordering::Acquire) {
        tracing::debug!(
            event = "roots_pick_busy",
            "[RootsDesk] フォルダを選ぶ画面が出ているので、2 つ目は出しません"
        );
        return None;
    }
    if suppressed {
        tracing::info!(
            event = "roots_pick_cancelled",
            reason = "no_alert",
            "[RootsDesk] 告知が抑止されているので、フォルダを選ぶ画面を出さずに取り消しと同じに扱います"
        );
        return None;
    }
    desk.picking.store(true, Ordering::Release);
    // 旗はスレッドが終わるとき（panic も含む）と、スレッドを起こせずに閉包が落ちたときに降りる。
    let flag = PickingFlag(desk.picking.clone());
    let tx = desk.tx.clone();
    let spawned = thread::Builder::new()
        .name("roots-pick".to_owned())
        .spawn(move || {
            let _flag = flag;
            send_pick(pick(), &tx);
        });
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::error!(
                event = "roots_pick_failed",
                error = %err,
                "[RootsDesk] フォルダを選ぶ画面のスレッドを起こせません"
            );
            None
        }
    }
}

/// 選ぶ画面の結果を受け口へ流す（選ぶスレッド）。決めたら 1 件送り、取り消しは `info!`（要件 6.5）、
/// 出せなければ `error!`（要件 6.9）でどちらも送らない。
fn send_pick(result: Result<Option<PathBuf>, PickError>, tx: &Sender<PickedFolder>) {
    match result {
        Ok(Some(dir)) => {
            if let Err(err) = tx.send(PickedFolder::Chosen(dir)) {
                let PickedFolder::Chosen(dir) = err.0;
                tracing::warn!(
                    event = "roots_pick_send_failed",
                    path = %dir.display(),
                    "[RootsDesk] 選ばれたフォルダを受け口へ送れません（受け口が無い）"
                );
            }
        }
        Ok(None) => tracing::info!(
            event = "roots_pick_cancelled",
            reason = "cancelled",
            "[RootsDesk] フォルダを選ぶ画面が取り消されました"
        ),
        Err(err) => tracing::error!(
            event = "roots_pick_failed",
            error = ?err,
            "[RootsDesk] フォルダを選ぶ画面を出せません"
        ),
    }
}

#[cfg(test)]
#[path = "desk_tests.rs"]
mod tests;
