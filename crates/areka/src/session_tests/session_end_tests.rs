//! OS のセッションの終了の受け手 [`on_os_session_end`] のテスト。
//!
//! 受け手は窓の手続きの中で同期に後始末まで済ませるので、偽の SHIORI の土台（`SwitchRig`）で
//! 起こした A に直接呼んで、呼出列（`OnClose` の Ref0＝`system`）・最初の出所・終了の指示・置き場・
//! 起動中の印を見る。未処理の失敗の通知と `run()` の後の後始末は置き場に実行系の無い単位を
//! 入れた素の World で見る（予約と先に指示した失敗の場面は `areka-session` の `session_end_tests.rs`）。

use std::sync::mpsc;
use std::time::Duration;

use areka_kanade::{KanadeNotice, KanadeStopCause, KanadeStopped, ShioriFault, ShioriFaultKind};
use bevy_ecs::prelude::*;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::AppExit;

use crate::sample_test_support::switch_rig;
use crate::{after_run, finish_after_run};
use areka_runtime::spine::RecordedCall;
use areka_session::app_exit::{ExitOrigin, FirstExit};
use areka_session::boot_config::ConfigInputs;
use areka_session::boot_config::{BootContext, CurrentGhost};
use areka_session::boot_resolve::{
    BalloonDecision, BalloonRoute, GhostDecision, GhostRoute, read_session_mark, write_session_mark,
};
use areka_session::ghost_quit::KanadeNoticeRx;
use areka_session::ghost_session::{GhostSession, GhostSlot};
use areka_session::ghost_switch_test_support::{FakeShiori, standard_script};
use areka_session::session_end::{SessionEnded, end_session_within, on_os_session_end};

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
        roots: areka_ghost::BasewareRoots::single(tmp.path().to_path_buf()),
        roots_origin: areka_session::roots::RootsOrigin::EnvVar,
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
    write_session_mark(&tmp.child("app-profile"), "A").expect("印を書く");
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
/// セッションの終了・終了の指示あり・置き場が空・印が消え・済みの印が在る・所要 ms と打ち切りなし
/// （`shiori_cut=false`）の記録が 1 件。上限は負荷で揺れないよう大きく取って引数の入口で渡す。
/// 2 回目（守りの判定）は呼出列が増えず `debug!(os_session_end_again)` 1 件だけ。
#[test]
fn session_end_takes_ghost_down_with_system_close_and_clears_mark() {
    let mut rig = switch_rig(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.plant_boot_record("A");
    rig.boot("A");
    let app = rig.world.resource::<BootContext>().app_profile_dir.clone();
    write_session_mark(&app, "A").expect("印を書く");
    let steady = rig.wait_steady();

    let ((), first_events) =
        capture(|| end_session_within(&mut rig.world, Duration::from_secs(60)));
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
            .map(|e| (e.field("ms").is_some(), e.field("shiori_cut"))),
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
            Some((true, Some("false"))),
        ),
        "セッションの終了がきれいな終わりにならない（定常・OnClose の Ref0・最初の出所・終了の指示・\
         置き場・印・済みの印・開始・印の消去・所要・(ms, 打ち切り)）: {first_events:?}"
    );
    assert_eq!(
        (again, down),
        ((true, vec![tracing::Level::DEBUG], vec![], vec![]), true),
        "2 回目が無操作でない（呼出列・読み捨ての記録・開始・所要）／置き場が空で降ろせる: {again_events:?}"
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
