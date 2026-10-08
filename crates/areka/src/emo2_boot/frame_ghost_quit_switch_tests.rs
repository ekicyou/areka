//! 終了の相が運行の通知を切替の段で振り分けること（areka-P0-ghost-shell-balloon-switch task 8.4・
//! 要件 2.9・3.3・3.5・8.4・10.7）。
//!
//! 確かめること:
//!
//! 1. 切替の予約が在る間に届いた停止（中身あり）は切替として捌かれ、終了は指示されない。偽の
//!    SHIORI 2 体で A → B を 1 周させ、委譲先（切替の経路）の記録「降ろした所要 ms」が 1 件だけ残る。
//! 2. 予約が無ければ同じ通知は今日どおり終了を指示する。
//! 3. 送り出しの段で中身の無い停止（kanade が切替の要求を受理しないまま止まった）は、
//!    `warn!(ghost_switch_not_accepted)` 1 件の上で予約を下ろし今日どおり終了する。
//! 4. 保留の切替が台本の終了（`\-`）で捨てられたときは、kanade が停止より先に切替の中止
//!    （終了要求）を知らせる＝予約は `info!(ghost_switch_cancelled)` で下り、後の停止は今日の
//!    終了になる（警告は出ない・要件 5.5・8.11）。

use std::sync::mpsc;

use areka_kanade::{
    CancelReason, ChangeHandoff, ChangeOrigin, KanadeMsg, KanadeNotice, KanadeStopCause,
    KanadeStopped, TalkDone, TalkEndReason, TalkId,
};
use log_capture_kit::{CapturedEvent, capture};
use shiori_host32_host::ExitKind;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::*;
use crate::app_exit::FirstExit;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, PrevGhost, SwitchInFlight, SwitchRequest, SwitchStage, SwitchTarget, SwitchVerdict,
    request_ghost_switch,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig};
use crate::emo2_boot::spine::{Progress, ScriptedShioriBackend, wait_recv};
use crate::ghost_session::GhostSlot;

fn levels_of(events: &[CapturedEvent], event: &str) -> Vec<tracing::Level> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .map(|e| e.level)
        .collect()
}

/// 台詞を返さない台本（`OnBoot`・`OnGhostChanged` とも 204）。ただし起動記録の無いゴーストは
/// 初回起動の記録を書く台詞（id 1）を再生するので、フレームを回さないこのテストでは
/// その完了（`TalkDone`）を手で送ってから切替を頼む。
fn silent_script() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnFirstBoot", Ok(None))
            .get("OnGhostChanged", Ok(None))
            .get("OnBoot", Ok(None))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
    }))
}

/// 受け口から定常到達を 1 件待って読み捨てる（切替の要求を定常で受けさせるため）。先に届いた別の通知は
/// 読み飛ばす。受け口を World から外して眠って待ち（`wait_recv`）、打ち切りは足場の進みの目印で決める
/// （areka-P0-ghost-session-test-load-flake 要件 2.1・2.2）。届かなければ打ち切りの文言を標準エラーへ
/// 1 行出して `false`。
fn wait_steady(rig: &mut SwitchRig) -> bool {
    let probe = rig.progress_probe();
    let rx = rig
        .world
        .remove_non_send::<KanadeNoticeRx>()
        .expect("土台が受け口を据えている");
    let steady = loop {
        match wait_recv("A の定常到達", Progress::Count(&probe), &rx.0) {
            Ok(KanadeNotice::Steady) => break true,
            Ok(_) => {}
            Err(failure) => {
                eprintln!("{failure}");
                break false;
            }
        }
    };
    rig.world.insert_non_send(rx);
    steady
}

fn reservation(stage: SwitchStage) -> SwitchInFlight {
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
        stage,
        boot_event: None,
    }
}

/// 予約（送り出し）の下で A が中身つきの停止を出す → 終了の相は切替へ委譲し、B を起こして
/// 定常到達で予約が消える。終了は指示されず、委譲先の「降ろした所要 ms」は 1 件（要件 3.3・10.7）。
///
/// # 非空虚性
/// 終了の相が予約を見ずに今日どおり終了すれば終了が指示され、降ろした記録は 0 件になる。
#[test]
fn stop_with_handoff_under_reservation_switches_without_exit() {
    let mut rig = SwitchRig::new(vec![("A", silent_script()), ("B", silent_script())]);
    // 窓を作る閉包の投函先（切替先の窓の準備に要る・閉包は走らせない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("B");
    rig.boot("A");
    let steady = wait_steady(&mut rig);
    let verdict = request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Name("B".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: None,
        },
    );
    // A の起動記録のトーク（採番 1）は再生の完了まで切替を保留させる。フレームを回さない土台では
    // 再生が終わらないので、再生の完了を kanade へ直に届けて保留を消化させる。
    let talk_done_sent = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.kanade())
        .is_some_and(|k| {
            k.send(KanadeMsg::TalkDone(TalkDone {
                talk_id: TalkId(1),
                reason: TalkEndReason::Ended,
                quit_reserved: false,
            }))
            .is_ok()
        });
    let (done, events) = capture(|| {
        rig.pump_until(|rig| {
            rig.exit_requested() || rig.world.get_non_send::<SwitchInFlight>().is_none()
        })
    });
    let exit_requested = rig.exit_requested();
    let b_boots = rig.calls("B").len();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            steady,
            verdict,
            talk_done_sent,
            done,
            exit_requested,
            levels_of(&events, "ghost_switch_down_ms"),
            levels_of(&events, "ghost_quit").len(),
            levels_of(&events, "ghost_switch_done"),
            b_boots,
            shutdown_ok,
        ),
        (
            true,
            SwitchVerdict::Accepted,
            true,
            true,
            false,
            vec![tracing::Level::INFO],
            0,
            vec![tracing::Level::INFO],
            1,
            true,
        ),
        "(A の定常・受理・再生の完了を届けた・予約が消えた・終了の指示・降ろした記録・今日の終了・切替の完了・\
         B を起こした回数・降ろせた): {events:?}"
    );
}

/// 予約が無ければ中身つきの停止でも今日どおり終了を指示する（要件 3.5・8.4）。
#[test]
fn stop_with_handoff_without_reservation_quits() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    let mut world = World::new();
    world.insert_non_send(wintf::AppExit::new());
    world.insert_non_send(KanadeNoticeRx(rx));
    tx.send(KanadeNotice::Stopped(KanadeStopped {
        cause: KanadeStopCause::Quit,
        handoff: Some(ChangeHandoff { script: None }),
    }))
    .expect("停止通知を投函できる");

    let (consumed, events) = capture(|| run_ghost_quit_phase(&mut world));

    assert_eq!(
        (
            consumed,
            world.non_send::<wintf::AppExit>().is_requested(),
            levels_of(&events, "ghost_quit"),
        ),
        (true, true, vec![tracing::Level::INFO]),
        "(消化・終了の指示・今日の終了の記録): {events:?}"
    );
}

/// 送り出しの段で中身の無い停止 → `warn!(ghost_switch_not_accepted)` 1 件・予約が消え・
/// 今日どおり終了（終了要求が切替に勝つ・要件 2.9）。
#[test]
fn send_off_stop_without_handoff_quits_and_clears_reservation() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    let mut world = World::new();
    world.insert_non_send(wintf::AppExit::new());
    world.insert_non_send(KanadeNoticeRx(rx));
    world.insert_non_send(reservation(SwitchStage::SendOff));
    tx.send(KanadeNotice::Stopped(KanadeStopped {
        cause: KanadeStopCause::Quit,
        handoff: None,
    }))
    .expect("停止通知を投函できる");

    let (consumed, events) = capture(|| run_ghost_quit_phase(&mut world));

    assert_eq!(
        (
            consumed,
            world.non_send::<wintf::AppExit>().is_requested(),
            world.get_resource::<FirstExit>().map(|f| f.0.clone()),
            levels_of(&events, "ghost_switch_not_accepted"),
            world.get_non_send::<SwitchInFlight>().is_some(),
        ),
        (
            true,
            true,
            Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
            vec![tracing::Level::WARN],
            false,
        ),
        "(消化・終了の指示・最初の出所・受理されなかった記録・予約): {events:?}"
    );
}

/// 保留の切替を捨てた終了: kanade は停止より先に切替の中止（終了要求）を送る → 予約は
/// `info!(ghost_switch_cancelled)` で下り、停止は今日どおりの終了（受理されなかった警告は 0 件・
/// 要件 5.5・8.11）。
///
/// # 非空虚性
/// 中止の通知が無ければ停止が予約の下で届き、`warn!(ghost_switch_not_accepted)` が 1 件出る
/// （上の `send_off_stop_without_handoff_quits_and_clears_reservation` の場面）。
#[test]
fn cancel_notice_before_stop_clears_reservation_without_warning() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    let mut world = World::new();
    world.insert_non_send(wintf::AppExit::new());
    world.insert_non_send(KanadeNoticeRx(rx));
    world.insert_non_send(reservation(SwitchStage::SendOff));
    for notice in [
        KanadeNotice::ChangeCancelled {
            reason: CancelReason::CloseRequest,
        },
        KanadeNotice::Stopped(KanadeStopped {
            cause: KanadeStopCause::Quit,
            handoff: None,
        }),
    ] {
        tx.send(notice).expect("通知を投函できる");
    }

    let (consumed, events) = capture(|| run_ghost_quit_phase(&mut world));

    assert_eq!(
        (
            consumed,
            world.non_send::<wintf::AppExit>().is_requested(),
            world.get_resource::<FirstExit>().map(|f| f.0.clone()),
            levels_of(&events, "ghost_switch_not_accepted"),
            levels_of(&events, "ghost_switch_cancelled"),
            levels_of(&events, "ghost_quit"),
            world.get_non_send::<SwitchInFlight>().is_some(),
        ),
        (
            true,
            true,
            Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
            vec![],
            vec![tracing::Level::INFO],
            vec![tracing::Level::INFO],
            false,
        ),
        "(消化・終了の指示・最初の出所・受理されなかった記録・中止の記録・今日の終了・予約): {events:?}"
    );
}
