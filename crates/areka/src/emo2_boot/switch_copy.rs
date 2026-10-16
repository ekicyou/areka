//! 切替の写し（spec: areka-P0-user-data-root 要件 4.3・4.7・design「SwitchHook」）。
//!
//! 切替の受理の直後に [`start`] が、写す要が在れば背景のスレッドで `copy_for_write` を走らせ、
//! 予約に見込み [`SwitchCopy`] を持たせる。前のゴーストが止まった時点（`switch_to`）で
//! [`boot_dir`] が起こすフォルダを答え、写し中なら段を「写し待ち」にして握手の中身を預かる。
//! [`poll`] は今の取り出しの系の先頭で毎回呼ばれ、写しの答えを受け取り、写し待ちなら続きを走らせる。
//! UI スレッドは写しを待って止まらない。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use areka_ghost::{BasewareRoots, StoreKind};
use areka_kanade::ChangeHandoff;
use bevy_ecs::world::World;

use super::ghost_switch::{SwitchInFlight, SwitchStage, switch_to};
use crate::roots::user_root::{CopyPlan, copy_for_write, plan_copy};

/// 写しの見込み（予約 [`SwitchInFlight`] に持たせる）。
#[derive(Debug)]
pub(crate) enum SwitchCopy {
    /// 写す要が無い（`plan_copy` が `AsIs`）。
    None,
    /// 背景のスレッドで写している（答えのフォルダが届く受信端）。
    Running(Receiver<PathBuf>),
    /// 写し終えた（写しのフォルダ・失敗なら元のフォルダ）。
    Done(PathBuf),
}

/// 写し待ちの間に預かった握手の中身（[`poll`] が受け取った時点で `switch_to` へ渡す）。
struct CopyWaitHandoff(ChangeHandoff);

/// 受理の直後に呼ぶ。`plan_copy` が `Copy` なら `std::thread` で `copy_for_write` を走らせる。
pub(crate) fn start(roots: &BasewareRoots, exe_dir: Option<&Path>, dir: &Path) -> SwitchCopy {
    if plan_copy(roots, exe_dir, StoreKind::Ghost, dir) == CopyPlan::AsIs {
        return SwitchCopy::None;
    }
    let (tx, rx) = mpsc::channel();
    let (roots, exe_dir, dir) = (
        roots.clone(),
        exe_dir.map(Path::to_path_buf),
        dir.to_path_buf(),
    );
    std::thread::spawn(move || {
        let used = copy_for_write(&roots, exe_dir.as_deref(), StoreKind::Ghost, &dir);
        // 受け手が居ない（予約が下りた）なら答えは捨てる（写しは書ける根に残り、次の切替で使われる）。
        let _ = tx.send(used);
    });
    SwitchCopy::Running(rx)
}

/// 写しの答えを 1 回だけ覗く（届いていなければ `None`）。受信端が切れていたら元のフォルダ（error）。
fn try_take(flight: &mut SwitchInFlight) -> Option<PathBuf> {
    let SwitchCopy::Running(rx) = &flight.copy else {
        return None;
    };
    let dir = match rx.try_recv() {
        Ok(dir) => dir,
        Err(TryRecvError::Empty) => return None,
        Err(TryRecvError::Disconnected) => {
            tracing::error!(
                event = "ghost_switch_copy_lost",
                ghost = %flight.target.folder,
                "写しのスレッドが答えを返さずに終わった——元のフォルダから起こす"
            );
            flight.target.dir.clone()
        }
    };
    flight.copy = SwitchCopy::Done(dir.clone());
    Some(dir)
}

/// `switch_to` の先頭で呼ぶ。起こすフォルダを答える（写す要なし＝切替先のまま・写し終えた＝答えの
/// フォルダ）。写し中なら段を「写し待ち」にして握手の中身を預かり `None`（呼び手はそのまま戻る）。
pub(crate) fn boot_dir(world: &mut World, handoff: &ChangeHandoff) -> Option<PathBuf> {
    let mut flight = world.get_non_send_mut::<SwitchInFlight>()?;
    let _ = try_take(&mut flight);
    match &flight.copy {
        SwitchCopy::None => Some(flight.target.dir.clone()),
        SwitchCopy::Done(dir) => Some(dir.clone()),
        SwitchCopy::Running(_) => {
            flight.stage = SwitchStage::CopyWait;
            tracing::info!(
                event = "ghost_switch_copy_wait",
                ghost = %flight.target.folder,
                "前のゴーストは止まったが写しが終わっていない——写し待ちにする"
            );
            world.insert_non_send(CopyWaitHandoff(handoff.clone()));
            None
        }
    }
}

/// `drain_change_requests` の先頭で毎回呼ぶ。写しの答えが届けば受け取り、段が「写し待ち」なら
/// `switch_to` の続きを走らせる。
pub(crate) fn poll(world: &mut World) {
    let Some(mut flight) = world.get_non_send_mut::<SwitchInFlight>() else {
        return;
    };
    if try_take(&mut flight).is_none() || flight.stage != SwitchStage::CopyWait {
        return;
    }
    let Some(CopyWaitHandoff(handoff)) = world.remove_non_send::<CopyWaitHandoff>() else {
        tracing::error!(
            event = "ghost_switch_copy_wait_lost",
            "写し待ちの握手の中身が無い——続きを走らせられない"
        );
        return;
    };
    switch_to(world, handoff);
}

#[cfg(test)]
#[path = "switch_copy_tests.rs"]
mod tests;
