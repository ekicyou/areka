//! 終了相の檻（areka-P0-emo2-conformance-e2e タスク 6.9・R15.4・design D15 の 4）。
//!
//! kanade の終了系列が完了したことを知らせる停止通知を受けたら、UI は全ゴースト窓を
//! **ちょうど 1 度**閉じる。ここで固定するのはその判断だけで、GPU も実窓も要らない
//! （素の `World` に `GhostWindowMarker` を並べて相を直に叩く）。
//!
//! 4 本の構成:
//!
//! 1. 通知 1 件 → 窓 0・`info!(event="ghost_quit")` 1 行。
//! 2. 2 件目は `debug!` で打ち切る（窓を 2 度閉じない）。
//! 3. 通知なし・受信端なしのフレームは無操作（定常フレームで窓が消えない）。
//! 4. 窓が既に無ければ `debug!` で打ち切る（失敗にしない）。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, mpsc};

use areka_emo_text::state::TextLayerConfig;
use areka_kanade::{KanadeStopCause, KanadeStopped};
use bevy_ecs::prelude::With;
use log_capture_kit::{LineFormat, capture_lines};

use super::test_support::synth_assets;
use super::*;
use crate::placement::spawn::GhostWindowMarker;

/// 停止通知の受信端を据えた（あるいは据えない）結線状態を組む。
///
/// GPU 資源も窓の正本も持たない素の構築である——終了相は presenter にも資産にも触れない。
fn wiring_with_stop(rx: Option<Receiver<KanadeStopped>>) -> Emo2Wiring {
    let mut wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal>().1,
        mpsc::channel::<crate::emo2_boot::zorder_cue::ZOrderDirective>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        TalkClock::new(Arc::new(|| 0.0)),
        synth_assets(&[(0, 0), (1, 10)]),
    );
    if let Some(rx) = rx {
        wiring.set_kanade_stop(rx);
    }
    wiring
}

/// `GhostWindowMarker` 窓を `count` 枚並べた素の World を組む（無関係 entity を 1 つ混ぜる）。
fn world_with_ghost_windows(count: usize) -> World {
    let mut world = World::new();
    for _ in 0..count {
        world.spawn(GhostWindowMarker);
    }
    world.spawn_empty();
    world
}

/// `GhostWindowMarker` 窓の現数。
fn ghost_count(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .count()
}

/// 停止通知 1 件で全ゴースト窓が閉じ、`info!(event="ghost_quit")` が 1 行残る（R15.4）。
///
/// # 非空虚性
/// 相が通知を読まなければ窓が 3 枚のまま残り、記録も出ない。
#[test]
fn one_notification_closes_every_ghost_window_and_logs_once() {
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut wiring = wiring_with_stop(Some(rx));
    let mut world = world_with_ghost_windows(3);
    tx.send(KanadeStopped {
        cause: KanadeStopCause::Quit,
    })
    .expect("停止通知を投函できる");

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut wiring, &mut world)
    });

    assert!(consumed, "通知を消化したフレームは以後の相を飛ばす");
    assert_eq!(ghost_count(&mut world), 0, "全ゴースト窓が閉じる（R15.4）");
    let quit_lines: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("event=\"ghost_quit\"") || l.contains("event=ghost_quit"))
        .collect();
    assert_eq!(
        quit_lines.len(),
        1,
        "終了は 1 行だけ記録される（原因つき）: {logs:?}"
    );
    assert!(
        quit_lines[0].contains("Quit"),
        "記録は停止の原因を載せる: {quit_lines:?}"
    );
}

/// 2 件目以降の通知は `debug!` で打ち切られ、窓を 2 度閉じにいかない（R15.4）。
///
/// # 非空虚性
/// 全件 drain せずに 1 件ずつ処理していれば、2 件目の記録（`ghost_quit_extra`）が出ない。
#[test]
fn a_second_notification_is_cut_off_with_a_debug_line() {
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut wiring = wiring_with_stop(Some(rx));
    let mut world = world_with_ghost_windows(2);
    for cause in [KanadeStopCause::Quit, KanadeStopCause::Forced] {
        tx.send(KanadeStopped { cause })
            .expect("停止通知を投函できる");
    }

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut wiring, &mut world)
    });

    assert!(consumed);
    assert_eq!(ghost_count(&mut world), 0);
    assert_eq!(
        logs.iter()
            .filter(|l| l.contains("ghost_quit_extra"))
            .count(),
        1,
        "2 件目は打ち切りとして 1 行だけ記録される: {logs:?}"
    );
    assert_eq!(
        logs.iter()
            .filter(|l| l.contains("event=\"ghost_quit\"") || l.contains("event=ghost_quit"))
            .filter(|l| !l.contains("ghost_quit_extra"))
            .count(),
        1,
        "窓を閉じる記録は 1 行だけ（2 度閉じない）: {logs:?}"
    );

    // 次のフレームは通知が尽きているので無操作（窓が無くても記録が増えない）。
    let (consumed_again, logs_again) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut wiring, &mut world)
    });
    assert!(!consumed_again, "通知が尽きたフレームは無操作");
    assert!(
        logs_again.is_empty(),
        "無操作のフレームは 1 行も記録しない: {logs_again:?}"
    );
}

/// 通知が無いフレーム・受信端を持たない結線は、いずれも完全な無操作である（定常フレーム）。
///
/// # 非空虚性
/// 相が窓に触れば件数が減り、記録を書けば行が残る。
#[test]
fn no_notification_and_no_receiver_are_both_complete_no_ops() {
    // ⑴ 受信端は在るが通知が来ていない。
    let (_tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut wiring = wiring_with_stop(Some(rx));
    let mut world = world_with_ghost_windows(2);
    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut wiring, &mut world)
    });
    assert!(!consumed);
    assert_eq!(ghost_count(&mut world), 2, "通知が無ければ窓は残る");
    assert!(logs.is_empty(), "無操作のフレームは記録しない: {logs:?}");

    // ⑵ そもそも受信端を持たない結線（既存の試験構築点と同じ形）。
    let mut unwired = wiring_with_stop(None);
    let mut world = world_with_ghost_windows(2);
    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut unwired, &mut world)
    });
    assert!(!consumed);
    assert_eq!(ghost_count(&mut world), 2, "結線していなければ窓は残る");
    assert!(logs.is_empty(), "無操作のフレームは記録しない: {logs:?}");
}

/// 窓が既に無い状態で通知が届いても失敗にしない——`debug!` で打ち切る（R15.4）。
///
/// 強制退避（Ctrl+Shift）が先に走った後に握手が完了した場合に起きる順序である。
///
/// # 非空虚性
/// 窓 0 を異常として扱えば（`error!`／panic）本檻が落ちる。
#[test]
fn a_notification_with_no_windows_left_is_cut_off_as_a_normal_case() {
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut wiring = wiring_with_stop(Some(rx));
    let mut world = world_with_ghost_windows(0);
    tx.send(KanadeStopped {
        cause: KanadeStopCause::Forced,
    })
    .expect("停止通知を投函できる");

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut wiring, &mut world)
    });

    assert!(consumed, "通知は消化する（窓の有無に依らない）");
    assert_eq!(
        logs.iter()
            .filter(|l| l.contains("ghost_quit_no_windows"))
            .count(),
        1,
        "閉じる窓が無いことは打ち切りとして記録される: {logs:?}"
    );
    assert_eq!(
        logs.iter().filter(|l| l.contains("level=ERROR")).count(),
        0,
        "窓 0 は異常ではない（ERROR を出さない）: {logs:?}"
    );
}
