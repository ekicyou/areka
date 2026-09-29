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
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::install::procedure::Overwritten;

/// 預かった書庫の段。
enum Stage {
    /// 預かった（切替はまだ頼んでいない）。
    Held(NarArchive),
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

/// 「預かった」の書庫について切替を頼む。宛先がもう起動中のゴーストでなければ書庫を返す。
fn request(world: &mut World) {
    let running = world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone());
    let Some(pending) = world.non_send_mut::<InstallDesk>().overwrite.take() else {
        return;
    };
    let Stage::Held(archive) = pending.stage else {
        world.non_send_mut::<InstallDesk>().overwrite = Some(pending);
        return;
    };
    let Some(running) = running.filter(|r| r.eq_ignore_ascii_case(&pending.folder)) else {
        tracing::debug!(
            event = "install_overwrite_returned",
            folder = %pending.folder,
            "[install] 宛先はもう起動中のゴーストではないので、書庫を返します"
        );
        reply_to(pending.reply, Overwritten::NotRunning(archive));
        return;
    };
    // 目録と同じ綴り（起動中のゴーストのフォルダ名）で名指しする。
    let verdict = request_ghost_switch(
        world,
        SwitchRequest {
            ghost: GhostSpec::Folder(running),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
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
        // 別の切替の最中: 預かったまま、次の定常到達で頼み直す。
        SwitchVerdict::Busy => Stage::Held(archive),
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
}

/// 定常到達: 「展開した」なら結果を背景のスレッドへ返して消し、「預かった」なら切替を頼み直す。
pub(super) fn on_steady(world: &mut World) {
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
                stage: Stage::Held(_),
                ..
            },
        ) => {
            desk.overwrite = Some(pending);
            request(world);
        }
        other => desk.overwrite = other,
    }
}
