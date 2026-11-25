//! 起動中のゴーストへ入れる一周（窓口の子）。
//!
//! 背景のスレッドから預かった書庫（高々 1 件）を段で進める: 預かった → 切替を頼んだ → 展開中 →
//! 起こし直した。預かったら既存の切替の入口へ「同じフォルダ・知らせなし・出どころ＝自動」で頼む
//! （再生中の台詞の終わりは kanade の保留が待つ）。`switch_to` が全窓を閉じた直後・起こす前に
//! 呼ぶ [`run_between`] が「降ろした」を背景のスレッドへ返して起こすのを待たせ、背景のスレッドが
//! 展開し終えた答え [`extracted`] で切替を続け、定常到達 [`on_steady`] で背景のスレッドへ返事をする。
//! 展開の成否によらず、切替の道筋はそのまま進む。

use areka_actor::ReplySender;
use areka_ghost::same_folder_name;
use areka_kanade::ChangeOrigin;
use areka_nar::NarArchive;
use bevy_ecs::world::World;

use super::{InstallDesk, reply_to};
use crate::install::worker::OverwriteAnswer;
use areka_session::app_exit::FirstExit;
use areka_session::boot_config::BootContext;
use areka_session::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, hold_between, request_ghost_switch,
    resume_between,
};

/// 預かった書庫の段。
enum Stage {
    /// 預かった（切替はまだ頼んでいない・次の定常到達で頼む）。
    Held(NarArchive, ReplySender<OverwriteAnswer>),
    /// 預かったが別の切替の最中だった（その予約が下りた tick か次の定常到達で頼み直す）。
    Busy(NarArchive, ReplySender<OverwriteAnswer>),
    /// 切替を頼んだ（切替の入口が受理した）。
    Requested(NarArchive, ReplySender<OverwriteAnswer>),
    /// 降ろしたと返した（背景のスレッドが展開している・切替は起こす前で待っている）。
    Extracting,
    /// 展開の答えを受けて起こし直した（定常到達で背景のスレッドへ返事をする）。
    Resumed(ReplySender<()>),
}

/// 預かった書庫と段（窓口が高々 1 件持つ）。
pub(super) struct Pending {
    /// 宛先のゴーストのフォルダ名（`ghost` は書庫の `directory`・`supplement` は宛先のゴースト）。
    folder: String,
    stage: Stage,
}

/// 背景のスレッドの上書きの頼みを預かり、切替を頼む。
pub(super) fn take(
    world: &mut World,
    archive: NarArchive,
    target_ghost: Option<String>,
    reply: ReplySender<OverwriteAnswer>,
) {
    let folder = target_ghost.unwrap_or_else(|| archive.manifest().directory.clone());
    let pending = Pending {
        folder,
        stage: Stage::Held(archive, reply),
    };
    if let Some(old) = world
        .non_send_mut::<InstallDesk>()
        .overwrite
        .replace(pending)
    {
        // 背景のスレッドは返事を待って止まるので 2 件目は来ない。来たら古い方を閉じる。
        tracing::warn!(
            event = "install_overwrite_replaced",
            old = %old.folder,
            "[install] 預かった書庫が 2 件になったので、古い方を閉じます"
        );
    }
    request(world);
}

/// 預かった書庫（「預かった」か「別の切替の最中だった」）について切替を頼む。宛先がもう起動中の
/// ゴーストでなければ書庫を返す。
fn request(world: &mut World) {
    let running = world
        .get_resource::<BootContext>()
        .map(|ctx| ctx.current.ghost.folder.clone());
    let Some(pending) = world.non_send_mut::<InstallDesk>().overwrite.take() else {
        return;
    };
    let (Stage::Held(archive, reply) | Stage::Busy(archive, reply)) = pending.stage else {
        world.non_send_mut::<InstallDesk>().overwrite = Some(pending);
        return;
    };
    // 目録と同じ綴り（起動中のゴーストのフォルダ名）で名指しする。起動の文脈が無ければ預かった宛先の
    // まま頼み、切替の入口の判定（`NoContext`）に任せる。
    let folder = match running {
        None => pending.folder.clone(),
        Some(running) => match running.filter(|r| same_folder_name(r, &pending.folder)) {
            Some(running) => running,
            None => {
                tracing::debug!(
                    event = "install_overwrite_returned",
                    folder = %pending.folder,
                    "[install] 宛先はもう起動中のゴーストではないので、書庫を返します"
                );
                reply_to(reply, OverwriteAnswer::NotRunning(archive));
                return;
            }
        },
    };
    let verdict = request_ghost_switch(
        world,
        SwitchRequest {
            ghost: GhostSpec::Folder(folder),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: None,
        },
    );
    let stage = match verdict {
        SwitchVerdict::Accepted => {
            tracing::info!(
                event = "install_overwrite_requested",
                folder = %pending.folder,
                "[install] 起動中のゴーストを降ろして入れるために、同じゴーストへの切替を頼みました"
            );
            Stage::Requested(archive, reply)
        }
        // 別の切替の最中: 預かったまま、その予約が下りた tick か次の定常到達で頼み直す。
        SwitchVerdict::Busy => Stage::Busy(archive, reply),
        SwitchVerdict::NotFound | SwitchVerdict::NoContext | SwitchVerdict::Updating => {
            tracing::warn!(
                event = "install_overwrite_unavailable",
                verdict = ?verdict,
                folder = %pending.folder,
                "[install] 起動中のゴーストを降ろして入れられないので、書庫を返してよそへの展開へ進ませます"
            );
            reply_to(reply, OverwriteAnswer::NotRunning(archive));
            return;
        }
    };
    world.non_send_mut::<InstallDesk>().overwrite = Some(Pending { stage, ..pending });
}

/// 降ろして全窓を閉じた直後・起こす前（`switch_to` から）: 段が「切替を頼んだ」で、切替の予約の
/// 切替先が預かった宛先と同じ（フォルダ名の 1 つの規則）ときだけ、「降ろした」を背景のスレッドへ
/// 返して起こすのを待たせる。返せなければ待たせない（`switch_to` がそのまま起こす）。
pub(super) fn run_between(world: &mut World) {
    let target = world
        .get_non_send::<SwitchInFlight>()
        .map(|f| f.target.folder.clone());
    // 並びと、起動中のゴーストの場所（根の下のゴーストのときだけ）。
    let roots = world.get_resource::<BootContext>().map(|ctx| {
        let ghost = &ctx.current.ghost;
        let running = ghost.folder.as_ref().map(|_| ghost.dir.clone());
        (ctx.roots.clone(), running)
    });
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let Some(pending) = desk.overwrite.as_ref() else {
        return;
    };
    if !matches!(pending.stage, Stage::Requested(..)) {
        return;
    }
    let ours = target.is_some_and(|t| same_folder_name(&t, &pending.folder));
    let (true, Some((roots, running))) = (ours, roots) else {
        tracing::debug!(
            event = "install_overwrite_skipped",
            folder = %pending.folder,
            "[install] 預かった宛先への切替ではない（または起動の文脈が無い）ので、展開しません"
        );
        return;
    };
    let Some(Pending {
        folder,
        stage: Stage::Requested(archive, reply),
    }) = desk.overwrite.take()
    else {
        return;
    };
    let down = OverwriteAnswer::Down {
        archive,
        roots,
        running,
    };
    if !reply_to(reply, down) {
        return;
    }
    desk.overwrite = Some(Pending {
        folder,
        stage: Stage::Extracting,
    });
    hold_between(world);
}

/// 背景のスレッドが展開し終えた: 段が「展開中」なら待たせた切替を続けて起こし、定常到達を待つ。
/// 段が合わなければ（終了の後・切替の中止の後）warn を 1 件残してその場で返事をする。
pub(super) fn extracted(world: &mut World, reply: ReplySender<()>) {
    let mut desk = world.non_send_mut::<InstallDesk>();
    let Some(pending) = desk
        .overwrite
        .as_mut()
        .filter(|p| matches!(p.stage, Stage::Extracting))
    else {
        tracing::warn!(
            event = "install_overwrite_unexpected",
            folder = desk.overwrite.as_ref().map(|p| p.folder.as_str()),
            "[install] 展開中でないのに展開し終えた知らせが来たので、切替は続けずに返事だけします"
        );
        reply_to(reply, ());
        return;
    };
    pending.stage = Stage::Resumed(reply);
    resume_between(world);
}

/// 背景のスレッドが倒れた: 展開の最中なら書庫を捨て、待たせた切替を続けて起こす。
pub(super) fn on_worker_panicked(world: &mut World) {
    let extracting = world
        .non_send_mut::<InstallDesk>()
        .overwrite
        .take_if(|p| matches!(p.stage, Stage::Extracting))
        .is_some();
    if extracting {
        resume_between(world);
    }
}

/// 終了が始まった（`desk::discard_for_exit` から）: 預かった書庫を捨て、宛先のフォルダ名と段の語を
/// 返す。返信端は落とす（背景のスレッドは閉じた扱いで止まるか、書き終えていれば結果を手続きへ返す）。
/// 待たせた切替は解かない（プロセスは終わるので起こさない）。
pub(super) fn discard(desk: &mut InstallDesk) -> Option<(String, &'static str)> {
    let pending = desk.overwrite.take()?;
    let stage = match pending.stage {
        Stage::Held(..) => "held",
        Stage::Busy(..) => "busy",
        Stage::Requested(..) => "requested",
        Stage::Extracting => "extracting",
        Stage::Resumed(_) => "resumed",
    };
    Some((pending.folder, stage))
}

/// 定常到達: 「起こし直した」なら背景のスレッドへ返事をして消す。「切替を頼んだ」のまま予約が消えて
/// いれば「預かった」へ戻し、「預かった」「別の切替の最中だった」なら切替を頼み直す。
pub(super) fn on_steady(world: &mut World) {
    requeue_if_cancelled(world);
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    match desk.overwrite.take() {
        Some(Pending {
            stage: Stage::Resumed(reply),
            ..
        }) => {
            reply_to(reply, ());
        }
        Some(
            pending @ Pending {
                stage: Stage::Held(..) | Stage::Busy(..),
                ..
            },
        ) => {
            desk.overwrite = Some(pending);
            request(world);
        }
        other => desk.overwrite = other,
    }
}

/// 毎 tick（取り出しの系から）: 「切替を頼んだ」のまま予約が消えていれば「預かった」へ戻して次の
/// 定常到達を待つ（頼み直しても終了の保留や起動の途中で同じく断られるだけ）。「別の切替の最中だった」
/// なら、その予約が下りた tick に頼み直す（中止された切替は定常到達を伴わない）。終了が指示された後は
/// 頼み直さない。
pub(super) fn on_tick(world: &mut World) {
    requeue_if_cancelled(world);
    let busy = world.get_non_send::<InstallDesk>().is_some_and(|desk| {
        matches!(
            desk.overwrite,
            Some(Pending {
                stage: Stage::Busy(..),
                ..
            })
        )
    });
    if busy
        && world.get_non_send::<SwitchInFlight>().is_none()
        && !world.contains_resource::<FirstExit>()
    {
        request(world);
    }
}

/// 「切替を頼んだ」のまま切替の予約が消えていたら（切替が中止された）「預かった」へ戻す。
fn requeue_if_cancelled(world: &mut World) {
    if world.get_non_send::<SwitchInFlight>().is_some() {
        return;
    }
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let Some(pending) = desk.overwrite.take() else {
        return;
    };
    let stage = match pending.stage {
        Stage::Requested(archive, reply) => {
            tracing::info!(
                event = "install_overwrite_requeued",
                folder = %pending.folder,
                "[install] 頼んだ切替が中止されたので、書庫を預かり直して次の定常到達で頼み直します"
            );
            Stage::Held(archive, reply)
        }
        stage => stage,
    };
    desk.overwrite = Some(Pending { stage, ..pending });
}
