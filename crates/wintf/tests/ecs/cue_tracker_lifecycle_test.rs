//! 統合テスト: CueSheetTracker ライフサイクルの検証
//!
//! Task 4.9 — Tracker の完了・キャンセル・選択肢応答・タイムアウト・エラーを検証する。

use wintf::ecs::cue::*;

// ── ヘルパー: QueueSnapshot を手動生成 ──

use wintf::ecs::cue::tracker::QueueSnapshot;

/// 全演者 Completed → CueSheetResult::Completed
#[test]
fn tracker_all_completed() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();
    let e2 = world.spawn_empty().id();

    let targets = vec![
        (ActorKey::from("sakura"), CueTarget::Shell, e1),
        (ActorKey::from("sakura"), CueTarget::Balloon, e2),
    ];

    let mut tracker = CueSheetTracker::new(targets);

    // 全スロット Completed のスナップショット
    let snapshots = vec![
        QueueSnapshot {
            entity: e1,
            state: CueQueueState::Completed,
            timed_out: false,
        },
        QueueSnapshot {
            entity: e2,
            state: CueQueueState::Completed,
            timed_out: false,
        },
    ];

    let _action = tracker.update(&snapshots);

    match tracker.result() {
        Some(CueSheetResult::Completed) => {} // OK
        other => panic!("Expected Completed, got {:?}", other),
    }
}

/// cancel() → CueSheetResult::Cancelled
#[test]
fn tracker_cancel() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();

    let targets = vec![(ActorKey::from("sakura"), CueTarget::Shell, e1)];

    let mut tracker = CueSheetTracker::new(targets);
    tracker.cancel();

    let snapshots = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::Playing,
        timed_out: false,
    }];

    let _action = tracker.update(&snapshots);

    match tracker.result() {
        Some(CueSheetResult::Cancelled) => {} // OK
        other => panic!("Expected Cancelled, got {:?}", other),
    }
}

/// WaitForChoice + 選択応答 → CueSheetResult::Choice
#[test]
fn tracker_choice_response() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();

    let targets = vec![(ActorKey::from("sakura"), CueTarget::Balloon, e1)];

    let mut tracker = CueSheetTracker::new(targets);

    // Phase 1: バリア検知（WaitingForChoice 状態）
    let snapshots_barrier = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::WaitingForChoice,
        timed_out: false,
    }];
    let _action = tracker.update(&snapshots_barrier);
    assert!(tracker.result().is_none(), "Should still be running during barrier");

    // Phase 2: 消費者がバリア応答を報告
    tracker.receive_barrier(BarrierResponse::Choice {
        id: "yes".into(),
    });

    // Phase 3: Tracker が応答を集約
    let _action = tracker.update(&snapshots_barrier);

    match tracker.result() {
        Some(CueSheetResult::Choice { id }) => {
            assert_eq!(id, "yes");
        }
        other => panic!("Expected Choice {{ id: 'yes' }}, got {:?}", other),
    }
}

/// タイムアウト → CueSheetResult::Timeout
#[test]
fn tracker_timeout() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();

    let targets = vec![(ActorKey::from("sakura"), CueTarget::Balloon, e1)];

    let mut tracker = CueSheetTracker::new(targets);

    // Phase 1: バリア検知
    let snapshots_barrier = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::WaitingForClick,
        timed_out: false,
    }];
    let _action = tracker.update(&snapshots_barrier);

    // Phase 2: タイムアウト検知
    let snapshots_timeout = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::WaitingForClick,
        timed_out: true,
    }];
    let _action = tracker.update(&snapshots_timeout);

    match tracker.result() {
        Some(CueSheetResult::Timeout) => {} // OK
        other => panic!("Expected Timeout, got {:?}", other),
    }
}

/// プロトコル違反（EmptyChoiceBarrier）→ CueSheetResult::Error
#[test]
fn tracker_error_on_protocol_violation() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();

    let targets = vec![(ActorKey::from("sakura"), CueTarget::Balloon, e1)];

    let mut tracker = CueSheetTracker::new(targets);

    // CueQueue がエラー状態に（先行 Choice なしに WaitForChoice が来た場合）
    let error = CueSystemError::EmptyChoiceBarrier {
        actor: "sakura".into(),
    };
    let snapshots = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::Error(error.clone()),
        timed_out: false,
    }];

    let _action = tracker.update(&snapshots);

    match tracker.result() {
        Some(CueSheetResult::Error(e)) => {
            assert_eq!(e, &error);
        }
        other => panic!("Expected Error, got {:?}", other),
    }
}

/// 一部 Playing + 一部 Completed → まだ完了しない
#[test]
fn tracker_partial_completion_not_done() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();
    let e2 = world.spawn_empty().id();

    let targets = vec![
        (ActorKey::from("sakura"), CueTarget::Shell, e1),
        (ActorKey::from("sakura"), CueTarget::Balloon, e2),
    ];

    let mut tracker = CueSheetTracker::new(targets);

    let snapshots = vec![
        QueueSnapshot {
            entity: e1,
            state: CueQueueState::Completed,
            timed_out: false,
        },
        QueueSnapshot {
            entity: e2,
            state: CueQueueState::Playing,
            timed_out: false,
        },
    ];

    let _action = tracker.update(&snapshots);
    assert!(tracker.result().is_none(), "Should not be complete while one queue is still Playing");
}

/// result() 確定後は update しても変わらない
#[test]
fn tracker_result_is_stable() {
    use bevy_ecs::prelude::*;

    let mut world = World::new();
    let e1 = world.spawn_empty().id();

    let targets = vec![(ActorKey::from("sakura"), CueTarget::Shell, e1)];

    let mut tracker = CueSheetTracker::new(targets);

    // 完了
    let snapshots = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::Completed,
        timed_out: false,
    }];
    let _action = tracker.update(&snapshots);
    assert!(matches!(tracker.result(), Some(CueSheetResult::Completed)));

    // 再度 update しても変わらない
    let snapshots2 = vec![QueueSnapshot {
        entity: e1,
        state: CueQueueState::Playing,
        timed_out: false,
    }];
    let _action = tracker.update(&snapshots2);
    assert!(matches!(tracker.result(), Some(CueSheetResult::Completed)));
}
