//! 起動中のゴーストへ入れる一周（窓口の子・design「System Flows／起動中のゴーストへ入れる一周」）。
//!
//! 背景のスレッドから預かった書庫（高々 1 件）を段で進める: 預かった → 切替を頼んだ → 展開した。
//! 預かったら既存の切替の入口へ「同じフォルダ・知らせなし・出どころ＝自動」で頼む（再生中の台詞の
//! 終わりは kanade の保留が待つ・設計で決めたこと 4）。`switch_to` が全窓を閉じた直後・起こす前に
//! 呼ぶ [`run_between`] が UI スレッドで同期に展開し（設計で決めたこと 3）、定常到達 [`on_steady`] で
//! 結果を背景のスレッドへ返す。展開の成否によらず、切替の道筋は今日のまま進む。

use std::time::Instant;

use areka_actor::ReplySender;
use areka_kanade::ChangeOrigin;
use areka_nar::{InstallOutcome, InstallRequest, NarArchive, NarError};
use bevy_ecs::world::World;

use super::{InstallDesk, reply_to};
use crate::app_exit::FirstExit;
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::install::procedure::Overwritten;

/// 預かった書庫の段。
enum Stage {
    /// 預かった（切替はまだ頼んでいない・次の定常到達で頼む）。
    Held(NarArchive),
    /// 預かったが別の切替の最中だった（その予約が下りた tick か次の定常到達で頼み直す）。
    Busy(NarArchive),
    /// 切替を頼んだ（切替の入口が受理した）。
    Requested(NarArchive),
    /// 展開した（定常到達で背景のスレッドへ返す）。
    Ran(Result<InstallOutcome, NarError>),
}

/// 預かった書庫と段（窓口が高々 1 件持つ）。
pub(super) struct Pending {
    /// 宛先のゴーストのフォルダ名（`ghost` は書庫の `directory`・`supplement` は宛先のゴースト）。
    folder: String,
    /// 展開に渡す宛先のゴースト（`supplement` だけ在る）。
    target_ghost: Option<String>,
    reply: ReplySender<Overwritten>,
    stage: Stage,
}

/// 背景のスレッドの上書きの頼みを預かり、切替を頼む。
pub(super) fn take(
    world: &mut World,
    archive: NarArchive,
    target_ghost: Option<String>,
    reply: ReplySender<Overwritten>,
) {
    let folder = target_ghost
        .clone()
        .unwrap_or_else(|| archive.manifest().directory.clone());
    let pending = Pending {
        folder,
        target_ghost,
        reply,
        stage: Stage::Held(archive),
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
    let (Stage::Held(archive) | Stage::Busy(archive)) = pending.stage else {
        world.non_send_mut::<InstallDesk>().overwrite = Some(pending);
        return;
    };
    // 目録と同じ綴り（起動中のゴーストのフォルダ名）で名指しする。起動の文脈が無ければ預かった宛先の
    // まま頼み、切替の入口の判定（`NoContext`）に任せる。
    let folder = match running {
        None => pending.folder.clone(),
        Some(running) => match running.filter(|r| r.eq_ignore_ascii_case(&pending.folder)) {
            Some(running) => running,
            None => {
                tracing::debug!(
                    event = "install_overwrite_returned",
                    folder = %pending.folder,
                    "[install] 宛先はもう起動中のゴーストではないので、書庫を返します"
                );
                reply_to(pending.reply, Overwritten::NotRunning(archive));
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
            Stage::Requested(archive)
        }
        // 別の切替の最中: 預かったまま、その予約が下りた tick か次の定常到達で頼み直す。
        SwitchVerdict::Busy => Stage::Busy(archive),
        SwitchVerdict::NotFound | SwitchVerdict::NoContext => {
            tracing::warn!(
                event = "install_overwrite_unavailable",
                verdict = ?verdict,
                folder = %pending.folder,
                "[install] 起動中のゴーストを降ろして入れられないので、書庫を返してよそへの展開へ進ませます"
            );
            reply_to(pending.reply, Overwritten::NotRunning(archive));
            return;
        }
    };
    world.non_send_mut::<InstallDesk>().overwrite = Some(Pending { stage, ..pending });
}

/// 降ろして全窓を閉じた直後・起こす前（`switch_to` から）: 段が「切替を頼んだ」で、切替の予約の
/// 切替先が預かった宛先と同じ（ASCII の大文字小文字を無視）ときだけ、UI スレッドで同期に展開する。
pub(super) fn run_between(world: &mut World) {
    let target = world
        .get_non_send::<SwitchInFlight>()
        .map(|f| f.target.folder.clone());
    let root = world
        .get_resource::<BootContext>()
        .map(|ctx| ctx.root.dir().to_path_buf());
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    let Some(pending) = desk.overwrite.as_mut() else {
        return;
    };
    let Stage::Requested(archive) = &pending.stage else {
        return;
    };
    let ours = target.is_some_and(|t| t.eq_ignore_ascii_case(&pending.folder));
    let (true, Some(root)) = (ours, root) else {
        tracing::debug!(
            event = "install_overwrite_skipped",
            folder = %pending.folder,
            "[install] 預かった宛先への切替ではない（または起動の文脈が無い）ので、展開しません"
        );
        return;
    };
    let started = Instant::now();
    let result = archive.install(&InstallRequest {
        root: &root,
        target_ghost: pending.target_ghost.as_deref(),
    });
    let ms = started.elapsed().as_millis() as u64;
    tracing::info!(
        event = "install_overwrite_done",
        ms,
        ok = result.is_ok(),
        folder = %pending.folder,
        "[install] 降ろした起動中のゴーストのフォルダへ展開しました"
    );
    pending.stage = Stage::Ran(result);
    // 書き終えたので門の名前を手放す（背景のスレッドは返事を待って止まっている）。定常到達の前に
    // 終了が始まっても、書く前にやめた書庫として記録させない。展開は UI スレッドで同期に走るので、
    // 同じ UI スレッドの後始末が展開の最中に始まることはなく、書く段の出入りは門へ知らせない。
    desk.gate.end();
}

/// 終了が始まった（`desk::discard_for_exit` から）: 預かった書庫を捨て、宛先のフォルダ名と段の語を
/// 返す。展開し終えていれば結果を背景のスレッドへ返し、それ以外は返信端を落とす（背景のスレッドは
/// 閉じた扱いで止まる）。以後の切替の道筋では展開しない。
pub(super) fn discard(desk: &mut InstallDesk) -> Option<(String, &'static str)> {
    let pending = desk.overwrite.take()?;
    let stage = match pending.stage {
        Stage::Held(_) => "held",
        Stage::Busy(_) => "busy",
        Stage::Requested(_) => "requested",
        Stage::Ran(result) => {
            reply_to(pending.reply, Overwritten::Ran(result));
            return Some((pending.folder, "ran"));
        }
    };
    Some((pending.folder, stage))
}

/// 定常到達: 「展開した」なら結果を背景のスレッドへ返して消す。「切替を頼んだ」のまま予約が消えて
/// いれば「預かった」へ戻し、「預かった」「別の切替の最中だった」なら切替を頼み直す。
pub(super) fn on_steady(world: &mut World) {
    requeue_if_cancelled(world);
    let Some(mut desk) = world.get_non_send_mut::<InstallDesk>() else {
        return;
    };
    match desk.overwrite.take() {
        Some(Pending {
            stage: Stage::Ran(result),
            reply,
            ..
        }) => reply_to(reply, Overwritten::Ran(result)),
        Some(
            pending @ Pending {
                stage: Stage::Held(_) | Stage::Busy(_),
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
                stage: Stage::Busy(_),
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
        Stage::Requested(archive) => {
            tracing::info!(
                event = "install_overwrite_requeued",
                folder = %pending.folder,
                "[install] 頼んだ切替が中止されたので、書庫を預かり直して次の定常到達で頼み直します"
            );
            Stage::Held(archive)
        }
        stage => stage,
    };
    desk.overwrite = Some(Pending { stage, ..pending });
}
