//! 停止通知と運行の通知を切替の段で振り分ける判断の決定論テスト（areka-P0-ghost-shell-balloon-switch
//! task 8.3）。
//!
//! 確かめること: 中止の通知で予約が消え `info!` 1 件（要件 5.2）／迎え入れ（既定）で失敗が届くと
//! 終了が指示され最初の出所が SHIORI の失敗（要件 6.4 の今日の失敗の経路）／定常到達で迎え入れの
//! 予約が消え、送り出しの段の予約は残る（要件 3.5）／送り出しの握手の途中の失敗でも中身があれば
//! 切替を続ける（要件 2.8）／迎え入れ（切替先）の失敗は既定へ戻す経路へ入る（要件 6.6）／
//! 迎え入れで失敗以外の停止は今日どおり終了（要件 3.5）。実ゴーストは起こさない。

use areka_kanade::{
    CancelReason, ChangeHandoff, KanadeNotice, KanadeStopCause, KanadeStopped, ShioriFault,
    ShioriFaultKind,
};
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::fallback_tests::{count_event, root_with, switching_world};
use super::*;
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::boot_resolve::DEFAULT_GHOST_FOLDER;

fn set_stage_to(world: &mut World, stage: SwitchStage) {
    world.non_send_mut::<SwitchInFlight>().stage = stage;
}

fn fault() -> KanadeStopCause {
    KanadeStopCause::Fault(ShioriFault {
        kind: ShioriFaultKind::ConnectFailed,
        reason: "接続できない".to_owned(),
    })
}

fn stopped(cause: KanadeStopCause, handoff: Option<ChangeHandoff>) -> KanadeStopped {
    KanadeStopped { cause, handoff }
}

fn level_of(events: &[log_capture_kit::CapturedEvent], event: &str) -> Vec<tracing::Level> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .map(|e| e.level)
        .collect()
}

/// 中止の通知 → 予約が消え `info!(ghost_switch_cancelled)` 1 件・終了は指示しない（要件 5.2）。
/// 予約が無いときの同じ通知は `warn!` 1 件。
#[test]
fn cancel_notice_clears_reservation_with_one_info() {
    let tmp = TempPath::new("ghost-switch-cancel");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");
    let notice = || KanadeNotice::ChangeCancelled {
        reason: CancelReason::UserBreak,
    };

    let ((), events) = capture(|| on_notice(&mut world, notice()));
    let ((), again) = capture(|| on_notice(&mut world, notice()));

    assert_eq!(
        (
            world.get_non_send::<SwitchInFlight>().is_some(),
            level_of(&events, "ghost_switch_cancelled"),
            level_of(&again, "ghost_switch_cancelled"),
            world.non_send::<AppExit>().is_requested(),
        ),
        (
            false,
            vec![tracing::Level::INFO],
            vec![tracing::Level::WARN],
            false
        ),
        "(予約・1 度目の記録・予約なしの記録・終了の指示): {events:?} / {again:?}"
    );
}

/// 迎え入れ（既定）で失敗の停止 → 予約が消え、終了が指示され最初の出所が SHIORI の失敗
/// （今日の失敗の経路＝告知と終了コード 1・要件 6.4・6.5）。既定へ戻す試みはもうしない。
#[test]
fn default_welcome_fault_quits_with_shiori_fault_origin() {
    let tmp = TempPath::new("ghost-switch-default-fault");
    let root = root_with(&tmp, &["A", "B", DEFAULT_GHOST_FOLDER]);
    let mut world = switching_world(&root, "B");
    set_stage_to(
        &mut world,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Default,
        },
    );

    let ((), events) = capture(|| on_ghost_stopped(&mut world, stopped(fault(), None)));

    let origin = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    assert!(
        matches!(
            &origin,
            Some(ExitOrigin::KanadeStopped(KanadeStopCause::Fault(f)))
                if f.kind == ShioriFaultKind::ConnectFailed
        ),
        "最初の出所: {origin:?}"
    );
    assert_eq!(
        (
            world.non_send::<AppExit>().is_requested(),
            world.get_non_send::<SwitchInFlight>().is_some(),
            count_event(&events, "ghost_switch_down_ms"),
            level_of(&events, "ghost_switch_default_fault"),
            count_event(&events, "ghost_switch_target_fault"),
        ),
        (true, false, 0, vec![tracing::Level::ERROR], 0),
        "(終了の指示・予約・降ろした・既定の失敗・切替先の失敗): {events:?}"
    );
}

/// 定常到達: 迎え入れの予約は消え `info!(ghost_switch_done)` 1 件（要件 3.5）。送り出しの段の
/// 予約は残る（まだ握手の途中＝前のゴーストの起動の定常到達が遅れて届いた形）。予約が無ければ
/// `debug!` だけ。
#[test]
fn steady_clears_welcoming_reservation_only() {
    let tmp = TempPath::new("ghost-switch-steady");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");

    let ((), send_off) = capture(|| on_notice(&mut world, KanadeNotice::Steady));
    let kept_in_send_off = world.get_non_send::<SwitchInFlight>().is_some();
    set_stage_to(
        &mut world,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        },
    );
    let ((), welcoming) = capture(|| on_notice(&mut world, KanadeNotice::Steady));
    let ((), none) = capture(|| on_notice(&mut world, KanadeNotice::Steady));

    assert_eq!(
        (
            kept_in_send_off,
            level_of(&send_off, "ghost_switch_done"),
            world.get_non_send::<SwitchInFlight>().is_some(),
            level_of(&welcoming, "ghost_switch_done"),
            level_of(&none, "ghost_switch_done"),
        ),
        (
            true,
            vec![tracing::Level::DEBUG],
            false,
            vec![tracing::Level::INFO],
            vec![tracing::Level::DEBUG]
        ),
        "(送り出しで残る・送り出しの done・迎え入れ後の予約・迎え入れの done・予約なし): \
         {send_off:?} / {welcoming:?} / {none:?}"
    );
}

/// 送り出しの握手の途中の失敗でも中身があれば切替を続ける（要件 2.8）: 前のゴーストを降ろし
/// （所要 ms 1 件）、今日の失敗の経路（出所 `KanadeStopped(Fault)`）へは流さない。作り口が無いので
/// 切替先も既定も起こせず致命で終える（出所は「既定ゴーストへ戻せなかった」）。
#[test]
fn send_off_fault_with_handoff_continues_switch() {
    let tmp = TempPath::new("ghost-switch-handshake-fault");
    let root = root_with(&tmp, &["A", "B", DEFAULT_GHOST_FOLDER]);
    let mut world = switching_world(&root, "B");

    let ((), events) = capture(|| {
        on_ghost_stopped(
            &mut world,
            stopped(fault(), Some(ChangeHandoff { script: None })),
        )
    });

    let origin = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    assert!(
        matches!(&origin, Some(ExitOrigin::GhostFallbackFailed(_))),
        "最初の出所: {origin:?} / {events:?}"
    );
    assert_eq!(
        count_event(&events, "ghost_switch_down_ms"),
        1,
        "前のゴーストを降ろす: {events:?}"
    );
}

/// 迎え入れ（切替先）で失敗 → `error!(ghost_switch_target_fault)` 1 件の上で既定へ戻す経路へ入る
/// （要件 6.6）。既定が目録に無い根なので致命で終わる（今日の失敗の経路へは流れない）。
#[test]
fn target_welcome_fault_falls_back_to_default() {
    let tmp = TempPath::new("ghost-switch-target-fault");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");
    set_stage_to(
        &mut world,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        },
    );

    let ((), events) = capture(|| on_ghost_stopped(&mut world, stopped(fault(), None)));

    let origin = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    assert!(
        matches!(&origin, Some(ExitOrigin::GhostFallbackFailed(f)) if f.reason.contains("落ちたゴースト: B")),
        "最初の出所: {origin:?}"
    );
    assert_eq!(
        (
            level_of(&events, "ghost_switch_target_fault"),
            count_event(&events, "ghost_switch_fatal"),
            count_event(&events, "ghost_switch_default_fault"),
        ),
        (vec![tracing::Level::ERROR], 1, 0),
        "(切替先の失敗・致命・既定の失敗): {events:?}"
    );
}

/// 切替先が既定ゴーストで迎え入れ中に失敗 → 既定へ戻さず（降ろさない）今日の失敗の経路で終了
/// （戻す試みは 1 回だけ・壊れた切替先が既定なら即時終了・要件 6.5）。
#[test]
fn default_as_target_fault_quits_without_retry() {
    let tmp = TempPath::new("ghost-switch-target-is-default");
    let root = root_with(&tmp, &["A", DEFAULT_GHOST_FOLDER]);
    let mut world = switching_world(&root, DEFAULT_GHOST_FOLDER);
    set_stage_to(
        &mut world,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        },
    );

    let ((), events) = capture(|| on_ghost_stopped(&mut world, stopped(fault(), None)));

    assert_eq!(
        (
            matches!(
                world.get_resource::<FirstExit>().map(|f| &f.0),
                Some(ExitOrigin::KanadeStopped(KanadeStopCause::Fault(_)))
            ),
            world.get_non_send::<SwitchInFlight>().is_some(),
            count_event(&events, "ghost_switch_down_ms"),
            level_of(&events, "ghost_switch_default_fault"),
            count_event(&events, "ghost_switch_target_fault"),
        ),
        (true, false, 0, vec![tracing::Level::ERROR], 0),
        "(出所が SHIORI の失敗・予約・降ろした・既定の失敗・切替先の失敗): {events:?}"
    );
}

/// 迎え入れで失敗以外の停止（切替先が終了を望んだ）→ `info!` の上で予約を下ろし今日どおり終了。
#[test]
fn welcoming_non_fault_stop_quits_as_today() {
    let tmp = TempPath::new("ghost-switch-target-quit");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");
    set_stage_to(
        &mut world,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        },
    );

    let ((), events) =
        capture(|| on_ghost_stopped(&mut world, stopped(KanadeStopCause::Quit, None)));

    assert_eq!(
        (
            world.get_resource::<FirstExit>().map(|f| f.0.clone()),
            world.get_non_send::<SwitchInFlight>().is_some(),
            level_of(&events, "ghost_switch_target_quit"),
        ),
        (
            Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
            false,
            vec![tracing::Level::INFO]
        ),
        "(最初の出所・予約・記録): {events:?}"
    );
}

/// 定常到達の記憶の書き手が無い（置き場が無い World）→ `warn!(steady_memory_not_recorded)` 1 件で
/// 続け、迎え入れの予約は下りる（task 11.2・要件 12.6・design Error Handling）。
#[test]
fn steady_without_slot_warns_once_and_clears_reservation() {
    let tmp = TempPath::new("ghost-switch-steady-no-slot");
    let root = root_with(&tmp, &["A", "B"]);
    let mut world = switching_world(&root, "B");
    world.remove_non_send::<GhostSlot>();
    set_stage_to(
        &mut world,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        },
    );

    let ((), events) = capture(|| on_notice(&mut world, KanadeNotice::Steady));

    assert_eq!(
        (
            level_of(&events, "steady_memory_not_recorded"),
            world.get_non_send::<SwitchInFlight>().is_some(),
            level_of(&events, "ghost_switch_done"),
        ),
        (
            vec![tracing::Level::WARN],
            false,
            vec![tracing::Level::INFO]
        ),
        "(記録・予約・切替の終わり): {events:?}"
    );
}
