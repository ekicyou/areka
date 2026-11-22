//! OS のセッションの終了の受け手 [`on_os_session_end`] のうち、実行系の要らない場面のテスト。
//!
//! 置き場に実行系の無い単位を入れた素の World で、切替の予約・未処理の通知・先に届いた失敗を見る。
//! 偽の SHIORI の土台で起こした A に当てる場面は本体の `session_tests/session_end_tests.rs` が見る。

use std::sync::mpsc;

use areka_ghost::BasewareRoot;
use areka_kanade::{
    CancelReason, ChangeHandoff, KanadeNotice, KanadeStopCause, KanadeStopped, ShioriFaultKind,
};
use bevy_ecs::prelude::*;
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::on_os_session_end;
use super::test_support::{event_field, fault, levels_of, reservation};
use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_resolve::{read_session_mark, write_session_mark};
use crate::ghost_quit::KanadeNoticeRx;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::ghost_switch::{SwitchInFlight, SwitchStage};
use crate::reload::test_support::boot_context_named;

/// 実行系の無い単位を置き場に持つ素の World（記憶の置き場は `tmp` の下・印＝A を書いておく）。
/// 戻りの送出端から運行の通知を受け口へ積める。
fn bare_world(tmp: &TempPath) -> (World, mpsc::Sender<KanadeNotice>) {
    let ctx = boot_context_named(
        tmp.path(),
        &BasewareRoot::new(tmp.path().to_path_buf()),
        "A",
        "b",
    );
    let ghost_dir = ctx.current.ghost.dir.clone();
    let mut world = World::new();
    world.insert_non_send(AppExit::new());
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(KanadeNoticeRx(rx));
    write_session_mark(&ctx.app_profile_dir, "A").expect("印を書く");
    world.insert_resource(ctx);
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(None, ghost_dir))));
    (world, tx)
}

fn first_exit(world: &World) -> Option<ExitOrigin> {
    world.get_resource::<FirstExit>().map(|f| f.0.clone())
}

fn slot_empty(world: &World) -> bool {
    world
        .get_non_send::<GhostSlot>()
        .is_some_and(|slot| slot.0.is_none())
}

/// 切替の予約が在る World → 予約が消え `info!(ghost_switch_cancelled, reason=session_end)` 1 件・
/// 最初の出所はセッションの終了・印が消える。
#[test]
fn session_end_drops_switch_reservation() {
    let tmp = TempPath::new("session-end-reservation");
    let (mut world, _tx) = bare_world(&tmp);
    world.insert_non_send(reservation(SwitchStage::SendOff));

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));
    let cancelled_reason = event_field(&events, "ghost_switch_cancelled", "reason");

    assert_eq!(
        (
            world.get_non_send::<SwitchInFlight>().is_some(),
            levels_of(&events, "ghost_switch_cancelled"),
            cancelled_reason,
            first_exit(&world),
            slot_empty(&world),
            read_session_mark(&tmp.child("profile")),
        ),
        (
            false,
            vec![tracing::Level::INFO],
            Some("session_end".to_owned()),
            Some(ExitOrigin::SessionEnd),
            true,
            None,
        ),
        "予約が下りない（予約・記録・理由・最初の出所・置き場・印）: {events:?}"
    );
}

/// 予約（送り出し）の下で、切替の握手を終えた停止（中身あり）が未処理のまま受け口に在る → 予約を
/// 先に下ろしてから捌くので、停止は予約の無い停止として終了（最初の出所は `KanadeStopped(Quit)`）になり、
/// 切替の経路（降ろす・切替先を起こす）へは入らない。
///
/// # 非空虚性
/// 捌くのを予約を下ろすより先にすると、停止は切替として捌かれ、置き場のゴーストを降ろして
/// 切替先を起こそうとする（`ghost_switch_down_ms` が 1 件・起こす文脈が無いので `ghost_switch_no_context`）。
#[test]
fn pending_switch_stop_is_quit_not_switch() {
    let tmp = TempPath::new("session-end-pending-switch-stop");
    let (mut world, tx) = bare_world(&tmp);
    world.insert_non_send(reservation(SwitchStage::SendOff));
    tx.send(KanadeNotice::Stopped(KanadeStopped {
        cause: KanadeStopCause::Quit,
        handoff: Some(ChangeHandoff { script: None }),
    }))
    .expect("停止通知を積める");

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));
    let switch_path: Vec<usize> = [
        "ghost_switch_down_ms",
        "ghost_switch_booted",
        "ghost_switch_boot_failed",
        "ghost_switch_no_context",
    ]
    .iter()
    .map(|event| levels_of(&events, event).len())
    .collect();

    assert_eq!(
        (
            first_exit(&world),
            switch_path,
            world.get_non_send::<SwitchInFlight>().is_some(),
            levels_of(&events, "ghost_quit"),
            slot_empty(&world),
        ),
        (
            Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
            vec![0, 0, 0, 0],
            false,
            vec![tracing::Level::INFO],
            true,
        ),
        "未処理の切替の停止が切替として捌かれた（最初の出所・切替の経路の記録 [降ろした・起こした・起こせない・文脈なし]・予約・終了の記録・置き場）: {events:?}"
    );
}

/// 予約を下ろした後に届いていた切替の中止の通知は、セッションの終了の中では警告にしない
/// （`debug!`・予約は既にセッションの終了で下ろした）。
#[test]
fn pending_change_cancelled_is_quiet_during_session_end() {
    let tmp = TempPath::new("session-end-pending-cancel");
    let (mut world, tx) = bare_world(&tmp);
    world.insert_non_send(reservation(SwitchStage::SendOff));
    tx.send(KanadeNotice::ChangeCancelled {
        reason: CancelReason::CloseRequest,
    })
    .expect("中止の通知を積める");

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));

    assert_eq!(
        (
            levels_of(&events, "ghost_switch_cancelled"),
            first_exit(&world),
        ),
        (
            vec![tracing::Level::INFO, tracing::Level::DEBUG],
            Some(ExitOrigin::SessionEnd),
        ),
        "中止の通知の記録（予約を下ろした info・遅れた中止の debug）・最初の出所: {events:?}"
    );
}

/// 先に SHIORI の失敗で終了を指示した World → 最初の出所は失敗のまま・印が残る（`Keep("fault")`）。
#[test]
fn session_end_after_fault_exit_keeps_mark() {
    let tmp = TempPath::new("session-end-after-fault");
    let (mut world, _tx) = bare_world(&tmp);
    let fault_origin =
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault(ShioriFaultKind::Disconnected)));
    quit_app(&mut world, fault_origin.clone());

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));
    let kept_reason = event_field(&events, "session_mark_kept", "reason");

    assert_eq!(
        (
            first_exit(&world),
            slot_empty(&world),
            read_session_mark(&tmp.child("profile")),
            kept_reason,
            levels_of(&events, "os_session_end_done"),
        ),
        (
            Some(fault_origin),
            true,
            Some("A".to_owned()),
            Some("fault".to_owned()),
            vec![tracing::Level::INFO],
        ),
        "失敗が先の終了で印が消えた（最初の出所・置き場・印・残す理由・所要）: {events:?}"
    );
}
