//! 致命の机（areka-P0-write-failure-notice・design「致命の机 write_failure.rs」）。
//!
//! 記憶を書けなかった失敗を 1 つの道へ集める。ゴーストの書き手（sylphya のアクター）は
//! 投函の書き込みの失敗を受け口 [`WriteFailureTx`] へ送り、UI スレッドの汲み出しの系
//! （[`drain_system`]・`Update` の 1 本）が 1 件ずつ [`fatal`] へ渡す。
//! [`fatal`] は記録 `memory_write_failed` を 1 件残し、出所
//! [`ExitOrigin::MemoryWriteFailed`] で [`quit_app`] を呼ぶ（巻き戻しはしない）。
//! 最初の出所が勝つ規則は [`quit_app`] が持つので、2 件目以降は記録だけが増える。

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_sylphya::{PersistFailure, PersistFailureCause, PersistScope};
use bevy_ecs::prelude::*;
use wintf::ecs::Update;

use crate::alert::{self, AlertScene, WriteMoment, remembered_text};
use crate::app_exit::{ExitOrigin, quit_app};
use crate::boot_config::ConfigInputs;

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
/// 別の出所で終わり始めた後（OS のセッションの終了を除く）は取り出さずに受け手へ残す。
/// [`fatal`] へ渡しても最初の出所が勝って告知が出ないので、後始末の [`tell_at_close`] が
/// 降ろす途中の失敗として告げる。
fn drain_system(world: &mut World) {
    let Some(rx) = world.get_non_send::<WriteFailureRx>() else {
        return;
    };
    let other_origin = world
        .get_resource::<crate::app_exit::FirstExit>()
        .is_some_and(|first| !matches!(first.0, ExitOrigin::MemoryWriteFailed(_)));
    if other_origin && !world.contains_resource::<crate::session_end::SessionEnded>() {
        return;
    }
    let failures: Vec<PersistFailure> = rx.0.try_iter().collect();
    for failure in failures {
        fatal(world, failure);
    }
}

/// `main` が返す失敗（終了コード 1）。
fn e_fail() -> windows::core::Error {
    windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)
}

/// 起動を止める: `memory_write_failed` を記録し、`MemoryWrite { moment: Boot }` を告げ、
/// `main` が返す `E_FAIL` を返す。`AREKA_NO_ALERT` を読む薄い口。
pub(crate) fn stop_boot(failure: PersistFailure) -> windows::core::Error {
    stop_boot_with(failure, alert::suppressed())
}

/// [`stop_boot`] の中身。抑止でも失敗を返す（画面だけを抑える・要件 2.6）。
pub(crate) fn stop_boot_with(failure: PersistFailure, suppressed: bool) -> windows::core::Error {
    log_failure(&failure, WriteMoment::Boot);
    alert::raise(
        &AlertScene::MemoryWrite {
            failure,
            moment: WriteMoment::Boot,
        },
        suppressed,
    );
    e_fail()
}

/// 書けるかの確かめ: `dir`（無ければ在るいちばん深い親）を選び、フォルダでなければ Err、
/// フォルダなら `areka-user-data` の確かめの口へ渡す（作る物 0・フォルダも作らない）。
pub(crate) fn probe_dir(dir: &Path) -> std::io::Result<()> {
    let Some((found, meta)) = dir
        .ancestors()
        .find_map(|p| std::fs::metadata(p).ok().map(|m| (p, m)))
    else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("在るフォルダが見つからない: {}", dir.display()),
        ));
    };
    if !meta.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("フォルダでない物が在る: {}", found.display()),
        ));
    }
    areka_user_data::open_dir_for_add(found)
}

/// 起動の前の確かめの入口（readonly-mode 要件 5.1・5.2・6.4）。`readonly` なら何も開かず
/// 作らずに通す（info 1 件）。そうでなければ今の [`check_boot_dirs`]。
pub(crate) fn check_boot(
    readonly: bool,
    cfg: &ConfigInputs,
    app_profile_dir: &Path,
) -> Result<(), PersistFailure> {
    if readonly {
        tracing::info!(
            event = "readonly_boot_check_skipped",
            "[write_failure] 読み取りだけなので、記憶の置き場が書けるかを確かめません"
        );
        return Ok(());
    }
    check_boot_dirs(cfg, app_profile_dir)
}

/// 起動の前の確かめ: アプリの記憶の置き場 → ゴーストの記憶の置き場
/// （`<cfg.ghost_root>\ghost\master\profile\areka`）の順で、最初の失敗を返す。
/// 見るのは渡された `cfg`（`resolve_boot` が返した＝写しの後の置き場）だけ。シェルの記憶は見ない。
pub(crate) fn check_boot_dirs(
    cfg: &ConfigInputs,
    app_profile_dir: &Path,
) -> Result<(), PersistFailure> {
    let ghost_dir = profile_areka_root(&cfg.ghost_root.join("ghost").join("master"));
    for (scope, dir) in [
        (PersistScope::App, app_profile_dir.to_path_buf()),
        (PersistScope::Ghost, ghost_dir),
    ] {
        if let Err(e) = probe_dir(&dir) {
            return Err(PersistFailure {
                scope,
                dir: Some(dir),
                keys: vec![],
                cause: PersistFailureCause::Io(e.to_string()),
            });
        }
    }
    Ok(())
}

/// 後始末で受け手を取り出す（OS のセッションの終了を処理済みなら `None`＝告げない）。
pub(crate) fn take_receiver(world: &mut World) -> Option<Receiver<PersistFailure>> {
    if world.contains_resource::<crate::session_end::SessionEnded>() {
        return None;
    }
    world.remove_non_send::<WriteFailureRx>().map(|rx| rx.0)
}

/// OS のセッションの終了の中の後始末: 印を消せなかった失敗と、受け手に残っていた失敗を
/// `memory_write_failed` で 1 件ずつ記録するだけ（告知は出さない・要件 2.3 の例外）。
pub(crate) fn log_only(world: &mut World, clear_failed: Option<PersistFailure>) {
    if let Some(failure) = &clear_failed {
        log_failure(failure, WriteMoment::Running);
    }
    if let Some(rx) = world.get_non_send::<WriteFailureRx>() {
        for failure in rx.0.try_iter() {
            log_failure(&failure, WriteMoment::Running);
        }
    }
}

/// 後始末の告知を 1 つだけ選んで告げる: 出所の場面 → 印を消せなかった失敗 → 降ろす途中の
/// 失敗（受け手に残っていた最初の 1 件）の順。2・3 を告げたときは `Err(E_FAIL)`、1 のときの
/// 終了コードは呼び手の旗が決める（`Ok`）。印を消せなかった失敗と残っていた失敗は、告げるかに
/// 関わらず 1 件ずつ記録する。
pub(crate) fn tell_at_close(
    scene: Option<AlertScene>,
    clear_failed: Option<PersistFailure>,
    late: Option<Receiver<PersistFailure>>,
    suppressed: bool,
) -> windows::core::Result<()> {
    let late: Vec<PersistFailure> = late.map(|rx| rx.try_iter().collect()).unwrap_or_default();
    for failure in clear_failed.iter().chain(&late) {
        log_failure(failure, WriteMoment::Running);
    }
    if let Some(scene) = scene {
        alert::raise(&scene, suppressed);
        return Ok(());
    }
    let Some(failure) = clear_failed.or_else(|| late.into_iter().next()) else {
        return Ok(());
    };
    alert::raise(
        &AlertScene::MemoryWrite {
            failure,
            moment: WriteMoment::Running,
        },
        suppressed,
    );
    Err(e_fail())
}

#[cfg(test)]
#[path = "write_failure_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "write_failure_sites_tests.rs"]
pub(crate) mod sites_tests;

#[cfg(test)]
#[path = "write_failure_probe_tests.rs"]
mod probe_tests;

#[cfg(test)]
#[path = "write_failure_late_tests.rs"]
mod late_tests;
