//! OS のセッションの終了の受け手 [`on_os_session_end`] のテスト（areka-P0-ghost-shell-balloon-switch
//! task 11.4・要件 12.9〜12.11・12.12 ⑹）。
//!
//! 受け手は窓の手続きの中で同期に後始末まで済ませるので、偽の SHIORI の土台（`SwitchRig`）で
//! 起こした A に直接呼んで、呼出列（`OnClose` の Ref0＝`system`）・最初の出所・終了の指示・置き場・
//! 起動中の印を見る。実行系の要らない場面（予約・先に届いた失敗）は置き場に実行系の無い単位を
//! 入れた素の World で見る。`run()` の後の後始末（`after_run`）が済みの印を見て告知の場面を組まず、
//! 終了コードを最初の出所から決めることもここで見る。

use std::sync::mpsc;

use areka_ghost::BasewareRoot;
use areka_kanade::{
    CancelReason, ChangeHandoff, KanadeNotice, KanadeStopCause, KanadeStopped, ShioriFault,
    ShioriFaultKind,
};
use bevy_ecs::prelude::*;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::{SessionEnded, on_os_session_end};
use crate::ConfigInputs;
use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_config::{BootContext, CurrentGhost};
use crate::boot_resolve::{
    BalloonDecision, BalloonRoute, GhostDecision, GhostRoute, read_session_mark, write_session_mark,
};
use crate::emo2_boot::frame::KanadeNoticeRx;
use crate::emo2_boot::ghost_switch::{PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::{after_run, finish_after_run};

fn fault() -> ShioriFault {
    ShioriFault {
        kind: ShioriFaultKind::Disconnected,
        reason: "テストの失敗".to_owned(),
    }
}

/// `event` の記録の水準（届いた順）。
fn levels_of(events: &[CapturedEvent], event: &str) -> Vec<tracing::Level> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .map(|e| e.level)
        .collect()
}

/// 呼出列のうち `OnClose` の NOTIFY の Reference0（届いた分だけ）。
fn on_close_ref0(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::Notify { id, references } if id == "OnClose" => {
                Some(references.first().cloned().unwrap_or_default())
            }
            _ => None,
        })
        .collect()
}

/// 実行系の無い単位を置き場に持つ素の World（記憶の置き場は `tmp` の下・印＝A を書いておく）。
/// 戻りの送出端から運行の通知を受け口へ積める。
fn bare_world(tmp: &TempPath) -> (World, mpsc::Sender<KanadeNotice>) {
    let ghost_dir = tmp.child("ghost").join("A");
    let balloon_dir = tmp.child("balloon").join("b");
    let mut world = World::new();
    world.insert_non_send(AppExit::new());
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(KanadeNoticeRx(rx));
    world.insert_resource(BootContext {
        root: BasewareRoot::new(tmp.path().to_path_buf()),
        app_profile_dir: tmp.child("app-profile"),
        helper_exe: tmp.child("shiori-host32-helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: ghost_dir.clone(),
                balloon_root: balloon_dir.clone(),
            },
            ghost: GhostDecision {
                route: GhostRoute::Memory,
                dir: ghost_dir.clone(),
                folder: Some("A".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: balloon_dir,
                folder: Some("b".to_owned()),
            },
        },
    });
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(None, ghost_dir))));
    write_session_mark(&tmp.child("app-profile"), "A");
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

/// 定常の A にセッションの終了 → `OnClose`（NOTIFY・Ref0＝`system`）が 1 件・最初の出所は
/// セッションの終了・終了の指示あり・置き場が空・印が消え・済みの印が在る・所要 ms の記録が 1 件。
/// 2 回目（守りの判定）は呼出列が増えず `debug!(os_session_end_again)` 1 件だけ。
#[test]
fn session_end_takes_ghost_down_with_system_close_and_clears_mark() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.plant_boot_record("A");
    rig.boot("A");
    let app = rig.world.resource::<BootContext>().app_profile_dir.clone();
    write_session_mark(&app, "A");
    let steady = rig.wait_steady();

    let ((), first_events) = capture(|| on_os_session_end(&mut rig.world, Entity::PLACEHOLDER));
    let calls_after_first = rig.calls("A").last().cloned().unwrap_or_default();
    let observed = (
        steady,
        on_close_ref0(&calls_after_first),
        first_exit(&rig.world),
        rig.exit_requested(),
        slot_empty(&rig.world),
        read_session_mark(&app),
        rig.world.contains_resource::<SessionEnded>(),
        levels_of(&first_events, "os_session_end_begin"),
        levels_of(&first_events, "session_mark_cleared"),
        levels_of(&first_events, "os_session_end_done"),
        first_events
            .iter()
            .find(|e| e.field_str("event") == Some("os_session_end_done"))
            .is_some_and(|e| e.field("ms").is_some()),
    );

    let ((), again_events) = capture(|| on_os_session_end(&mut rig.world, Entity::PLACEHOLDER));
    let calls_after_again = rig.calls("A").last().cloned().unwrap_or_default();
    let again = (
        calls_after_again.len() == calls_after_first.len(),
        levels_of(&again_events, "os_session_end_again"),
        levels_of(&again_events, "os_session_end_begin"),
        levels_of(&again_events, "os_session_end_done"),
    );
    let down = rig.shutdown();

    assert_eq!(
        observed,
        (
            true,
            vec!["system".to_owned()],
            Some(ExitOrigin::SessionEnd),
            true,
            true,
            None,
            true,
            vec![tracing::Level::INFO],
            vec![tracing::Level::INFO],
            vec![tracing::Level::INFO],
            true,
        ),
        "セッションの終了がきれいな終わりにならない（定常・OnClose の Ref0・最初の出所・終了の指示・\
         置き場・印・済みの印・開始・印の消去・所要・ms）: {first_events:?}"
    );
    assert_eq!(
        (again, down),
        ((true, vec![tracing::Level::DEBUG], vec![], vec![]), true),
        "2 回目が無操作でない（呼出列・読み捨ての記録・開始・所要）／置き場が空で降ろせる: {again_events:?}"
    );
}

/// 送り出しの段の切替の予約（A → B）。
fn send_off_reservation() -> SwitchInFlight {
    SwitchInFlight {
        target: SwitchTarget {
            dir: "ghost/B".into(),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: "ghost/A".into(),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
    }
}

/// 切替の予約が在る World → 予約が消え `info!(ghost_switch_cancelled, reason=session_end)` 1 件・
/// 最初の出所はセッションの終了・印が消える。
#[test]
fn session_end_drops_switch_reservation() {
    let tmp = TempPath::new("session-end-reservation");
    let (mut world, _tx) = bare_world(&tmp);
    world.insert_non_send(send_off_reservation());

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));
    let cancelled_reason = events
        .iter()
        .find(|e| e.field_str("event") == Some("ghost_switch_cancelled"))
        .and_then(|e| e.field_str("reason").map(str::to_owned));

    assert_eq!(
        (
            world.get_non_send::<SwitchInFlight>().is_some(),
            levels_of(&events, "ghost_switch_cancelled"),
            cancelled_reason,
            first_exit(&world),
            slot_empty(&world),
            read_session_mark(&tmp.child("app-profile")),
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
/// 先に下ろしてから捌くので、停止は今日どおり終了（最初の出所は `KanadeStopped(Quit)`）になり、
/// 切替の経路（降ろす・切替先を起こす）へは入らない。
///
/// # 非空虚性
/// 捌くのを予約を下ろすより先にすると、停止は切替として捌かれ、置き場のゴーストを降ろして
/// 切替先を起こそうとする（`ghost_switch_down_ms` が 1 件・起こす文脈が無いので `ghost_switch_no_context`）。
#[test]
fn pending_switch_stop_is_quit_not_switch() {
    let tmp = TempPath::new("session-end-pending-switch-stop");
    let (mut world, tx) = bare_world(&tmp);
    world.insert_non_send(send_off_reservation());
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
        "未処理の切替の停止が切替として捌かれた（最初の出所・切替の経路の記録 [降ろした・起こした・起こせない・文脈なし]・予約・今日の終了・置き場）: {events:?}"
    );
}

/// 予約を下ろした後に届いていた切替の中止の通知は、セッションの終了の中では警告にしない
/// （`debug!`・予約は既にセッションの終了で下ろした）。
#[test]
fn pending_change_cancelled_is_quiet_during_session_end() {
    let tmp = TempPath::new("session-end-pending-cancel");
    let (mut world, tx) = bare_world(&tmp);
    world.insert_non_send(send_off_reservation());
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
    let fault_origin = ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault()));
    quit_app(&mut world, fault_origin.clone());

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));
    let kept_reason = events
        .iter()
        .find(|e| e.field_str("event") == Some("session_mark_kept"))
        .and_then(|e| e.field_str("reason").map(str::to_owned));

    assert_eq!(
        (
            first_exit(&world),
            slot_empty(&world),
            read_session_mark(&tmp.child("app-profile")),
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

/// 受け口に未処理の SHIORI の失敗の停止通知が積まれた World → 受け手が先に捌くので最初の出所は
/// 失敗・印が残る・`run()` の後の後始末は告知の場面を組まず、終了コードの材料は失敗（1）。
///
/// # 非空虚性
/// 未処理の通知を捌かずに終了を指示すると最初の出所はセッションの終了になり、印が消え、
/// 終了コードの材料は 0 になる。
#[test]
fn pending_fault_notice_wins_over_session_end() {
    let tmp = TempPath::new("session-end-pending-fault");
    let (mut world, tx) = bare_world(&tmp);
    tx.send(KanadeNotice::Stopped(KanadeStopped {
        cause: KanadeStopCause::Fault(fault()),
        handoff: None,
    }))
    .expect("停止通知を積める");

    let ((), events) = capture(|| on_os_session_end(&mut world, Entity::PLACEHOLDER));
    let (after, after_events) = capture(|| after_run(&mut world));
    let code_failed = finish_after_run(Ok(()), after.fault, || Ok(())).is_err();

    assert_eq!(
        (
            first_exit(&world),
            read_session_mark(&tmp.child("app-profile")),
            after.scene.is_none(),
            after.mark.is_none(),
            after.session.is_none(),
            after.fault,
            code_failed,
            levels_of(&after_events, "session_end_already_handled"),
        ),
        (
            Some(ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault()))),
            Some("A".to_owned()),
            true,
            true,
            true,
            true,
            true,
            vec![tracing::Level::INFO],
        ),
        "未処理の失敗がセッションの終了に覆われた（最初の出所・印・告知の場面なし・印の判定なし・\
         置き場・終了コードの材料・終了コード 1・後始末の記録）: {events:?} / {after_events:?}"
    );
}

/// 済みの印が在る後始末: 失敗でない出所なら告知の場面も印の判定も組まず、終了コードは 0。
#[test]
fn after_run_after_session_end_skips_alert_and_mark() {
    let tmp = TempPath::new("session-end-after-run");
    let (mut world, _tx) = bare_world(&tmp);
    on_os_session_end(&mut world, Entity::PLACEHOLDER);

    let (after, events) = capture(|| after_run(&mut world));
    let code_zero = finish_after_run(Ok(()), after.fault, || Ok(())).is_ok();

    assert_eq!(
        (
            after.session.is_none(),
            after.scene.is_none(),
            after.mark.is_none(),
            after.fault,
            code_zero,
            levels_of(&events, "session_end_already_handled"),
        ),
        (true, true, true, false, true, vec![tracing::Level::INFO]),
        "セッションの終了の後の後始末が崩れた（置き場・告知・印の判定・終了コードの材料・終了コード 0・記録）: {events:?}"
    );
}
