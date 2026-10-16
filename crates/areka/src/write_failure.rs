//! 致命の机（areka-P0-write-failure-notice・design「致命の机 write_failure.rs」）。
//!
//! 記憶を書けなかった失敗を 1 つの道へ集める。ゴーストの書き手（sylphya のアクター）は
//! 投函の書き込みの失敗を受け口 [`WriteFailureTx`] へ送り、UI スレッドの汲み出しの系
//! （[`drain_system`]・`Update` の 1 本）が 1 件ずつ [`fatal`] へ渡す。
//! [`fatal`] は記録 `memory_write_failed` を 1 件残し、出所
//! [`ExitOrigin::MemoryWriteFailed`] で [`quit_app`] を呼ぶ（巻き戻しはしない）。
//! 最初の出所が勝つ規則は [`quit_app`] が持つので、2 件目以降は記録だけが増える。

use std::sync::mpsc::{Receiver, Sender, channel};

use areka_sylphya::{PersistFailure, PersistFailureCause};
use bevy_ecs::prelude::*;
use wintf::ecs::Update;

use crate::alert::{WriteMoment, remembered_text};
use crate::app_exit::{ExitOrigin, quit_app};

/// 受け口の送り手（World の NonSend・プロセスに 1 つ）。
pub(crate) struct WriteFailureTx(pub(crate) Sender<PersistFailure>);

/// 受け口の受け手（World の NonSend・プロセスに 1 つ）。
pub(crate) struct WriteFailureRx(pub(crate) Receiver<PersistFailure>);

/// channel を 1 本作って両端を World に置き、汲み出しの系を `Update` に登録する。
/// 呼び手は `ghost_session::register_systems`（プロセスに 1 回）。
pub(crate) fn install(world: &mut World) {
    let (tx, rx) = channel();
    world.insert_non_send(WriteFailureTx(tx));
    world.insert_non_send(WriteFailureRx(rx));
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, drain_system);
}

/// 送り手の写し（据えていない World では `None`＝受け口なしで起こす）。
pub(crate) fn sender(world: &World) -> Option<Sender<PersistFailure>> {
    world
        .get_non_send::<WriteFailureTx>()
        .map(|tx| tx.0.clone())
}

/// 致命: `error!(event = "memory_write_failed")` を 1 件残し、
/// 出所 `ExitOrigin::MemoryWriteFailed(failure)` で `quit_app` を呼ぶ。巻き戻しはしない。
pub(crate) fn fatal(world: &mut World, failure: PersistFailure) {
    log_failure(&failure, WriteMoment::Running);
    quit_app(world, ExitOrigin::MemoryWriteFailed(failure));
}

/// 記録 `memory_write_failed`（`error!`・欄 scope・what・dir・reason・moment）を 1 件残す。
fn log_failure(failure: &PersistFailure, moment: WriteMoment) {
    let reason = match &failure.cause {
        PersistFailureCause::Io(reason) => reason.as_str(),
        PersistFailureCause::NoRoot => "no root",
    };
    let moment = match moment {
        WriteMoment::Boot => "boot",
        WriteMoment::Running => "running",
    };
    tracing::error!(
        event = "memory_write_failed",
        scope = ?failure.scope,
        what = %remembered_text(failure),
        dir = ?failure.dir,
        reason,
        moment,
        "[write_failure] 記憶を書けなかった——areka を終える"
    );
}

/// 毎フレーム: 受け手に溜まった失敗を全部取り出し、1 件ずつ [`fatal`] へ。
/// 受け口が World に無い（据えていない構成）なら何もしない。
fn drain_system(world: &mut World) {
    let Some(rx) = world.get_non_send::<WriteFailureRx>() else {
        return;
    };
    let failures: Vec<PersistFailure> = rx.0.try_iter().collect();
    for failure in failures {
        fatal(world, failure);
    }
}

#[cfg(test)]
#[path = "write_failure_tests.rs"]
mod tests;
