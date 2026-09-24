//! 終了相の檻（areka-P0-emo2-conformance-e2e タスク 6.9・R15.4・design D15 の 4）。
//!
//! kanade の終了系列が完了したことを知らせる停止通知を受けたら、UI は全ゴースト窓を
//! **ちょうど 1 度**閉じる。ここで固定するのはその判断だけで、GPU も実窓も要らない
//! （素の `World` に `GhostWindowMarker` と受け口 `KanadeStopRx` を並べて相を直に叩く）。
//!
//! 5 本の構成:
//!
//! 1. 通知 1 件 → 窓 0・`info!(event="ghost_quit")` 1 行。
//! 2. 2 件目は `debug!` で打ち切る（窓を 2 度閉じない）。
//! 3. 通知なし・受け口なしのフレームは無操作（定常フレームで窓が消えない）。
//! 4. 窓が既に無ければ `debug!` で打ち切る（失敗にしない）。
//! 5. 原因が Fault でも同じ 1 本道で終わり、記録と最初の出所に種類と理由が載る
//!    （areka-P0-shiori-fault-notice 要件 7.4・2.5）。

use std::sync::mpsc;

use areka_kanade::{KanadeStopCause, KanadeStopped, ShioriFault, ShioriFaultKind};
use bevy_ecs::prelude::With;
use log_capture_kit::{LineFormat, capture_lines};

use super::*;
use crate::app_exit::FirstExit;
use crate::placement::spawn::GhostWindowMarker;

/// `GhostWindowMarker` 窓を `count` 枚並べた素の World を組む（無関係 entity を 1 つ混ぜる）。
///
/// 終了の受け口（`wintf::AppExit`）も挿す——本番では `WinApp` が必ず挿すので、無ければ
/// 統合操作は配線の誤りとして `error!` を残す。停止通知の受け口は `rx` が `Some` のときだけ挿す。
fn world_with_ghost_windows(count: usize, rx: Option<Receiver<KanadeStopped>>) -> World {
    let mut world = World::new();
    world.insert_non_send(wintf::AppExit::new());
    if let Some(rx) = rx {
        world.insert_non_send(KanadeStopRx(rx));
    }
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

/// 終了の受け口が「指示済み」になっているか。
fn exit_requested(world: &World) -> bool {
    world
        .get_non_send::<wintf::AppExit>()
        .expect("world_with_ghost_windows が受け口を挿している")
        .is_requested()
}

/// `event="<name>"` の記録行だけを拾う（`ghost_quit_extra` 等の接頭一致は拾わない）。
fn event_lines<'a>(logs: &'a [String], name: &str) -> Vec<&'a String> {
    let quoted = format!("event=\"{name}\"");
    logs.iter().filter(|l| l.contains(&quoted)).collect()
}

/// 停止通知 1 件で全ゴースト窓が閉じ、`info!(event="ghost_quit")` が 1 行残る（R15.4）。
///
/// # 非空虚性
/// 相が通知を読まなければ窓が 3 枚のまま残り、記録も出ない。
#[test]
fn one_notification_closes_every_ghost_window_and_logs_once() {
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut world = world_with_ghost_windows(3, Some(rx));
    tx.send(KanadeStopped {
        cause: KanadeStopCause::Quit,
    })
    .expect("停止通知を投函できる");

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
    });

    assert!(consumed, "通知を消化したフレームは以後の相を飛ばす");
    assert_eq!(ghost_count(&mut world), 0, "全ゴースト窓が閉じる（R15.4）");
    assert!(
        exit_requested(&world),
        "窓を閉じた上で終了を指示する（要件 3.1）"
    );
    let quit_lines = event_lines(&logs, "ghost_quit");
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
    let mut world = world_with_ghost_windows(2, Some(rx));
    for cause in [KanadeStopCause::Quit, KanadeStopCause::Forced] {
        tx.send(KanadeStopped { cause })
            .expect("停止通知を投函できる");
    }

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
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
        event_lines(&logs, "ghost_quit").len(),
        1,
        "窓を閉じる記録は 1 行だけ（2 度閉じない）: {logs:?}"
    );

    // 次のフレームは通知が尽きているので無操作（窓が無くても記録が増えない）。
    let (consumed_again, logs_again) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
    });
    assert!(!consumed_again, "通知が尽きたフレームは無操作");
    assert!(
        logs_again.is_empty(),
        "無操作のフレームは 1 行も記録しない: {logs_again:?}"
    );
}

/// 通知が無いフレーム・受け口を持たない World は、いずれも完全な無操作である（定常フレーム）。
///
/// # 非空虚性
/// 相が窓に触れば件数が減り、記録を書けば行が残る。
#[test]
fn no_notification_and_no_receiver_are_both_complete_no_ops() {
    // ⑴ 受け口は在るが通知が来ていない。
    let (_tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut world = world_with_ghost_windows(2, Some(rx));
    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
    });
    assert!(!consumed);
    assert_eq!(ghost_count(&mut world), 2, "通知が無ければ窓は残る");
    assert!(logs.is_empty(), "無操作のフレームは記録しない: {logs:?}");

    // ⑵ そもそも受け口を持たない World（結線していない構成）。
    let mut world = world_with_ghost_windows(2, None);
    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
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
    let mut world = world_with_ghost_windows(0, Some(rx));
    tx.send(KanadeStopped {
        cause: KanadeStopCause::Forced,
    })
    .expect("停止通知を投函できる");

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
    });

    assert!(consumed, "通知は消化する（窓の有無に依らない）");
    assert!(
        exit_requested(&world),
        "閉じる窓が 0 でも終了を指示する（要件 3.2）"
    );
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

/// 原因が Fault の停止通知も同じ 1 本道で終わる: 窓が閉じ、終了が指示され、`ghost_quit`／
/// `app_exit` の記録に種類と理由が載り、最初の出所が Fault として World に残る
/// （areka-P0-shiori-fault-notice 要件 7.4・2.5・1.2）。
///
/// # 非空虚性
/// 記録から種類か理由を落とせば含有の主張が落ち、最初の出所を Fault 以外にすれば
/// `FirstExit` の比較が落ちる。
#[test]
fn a_fault_notification_quits_and_records_kind_and_reason() {
    const REASON: &str = "応答待ちが 5 秒を超えた";
    let fault = ShioriFault {
        kind: ShioriFaultKind::Timeout,
        reason: REASON.to_owned(),
    };
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    let mut world = world_with_ghost_windows(2, Some(rx));
    tx.send(KanadeStopped {
        cause: KanadeStopCause::Fault(fault.clone()),
    })
    .expect("停止通知を投函できる");

    let (consumed, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        run_ghost_quit_phase(&mut world)
    });

    assert!(consumed, "Fault の通知も消化する（原因による分岐なし）");
    assert_eq!(ghost_count(&mut world), 0, "全ゴースト窓が閉じる");
    assert!(exit_requested(&world), "終了を指示する");
    for name in ["ghost_quit", "app_exit"] {
        let lines = event_lines(&logs, name);
        assert_eq!(lines.len(), 1, "{name} は 1 行: {logs:?}");
        assert!(
            lines[0].contains("Timeout") && lines[0].contains(REASON),
            "{name} の記録に種類と理由が載る: {lines:?}"
        );
    }
    assert_eq!(
        world.resource::<FirstExit>().0,
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault)),
        "最初の出所は Fault の停止"
    );
}
