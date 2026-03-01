//! CueQueue + TimedSchedule 統合テスト (Task 8.4)
//!
//! 投入 → tick → ready → バリア停止 → 解除通知 の一連フロー、
//! 新 CueSheet 投入による全破棄・再構築、Entity 参照ラウンドトリップを検証する。

use bevy_ecs::world::World;
use wintf::ecs::cue::*;

// ---------------------------------------------------------------
// 投入 → tick → ready → バリア停止 → 解除通知の一連フロー
// ---------------------------------------------------------------

#[test]
fn integration_full_flow_with_barrier() {
    let mut queue = CueQueue::new();

    // Text → Choice → WaitForChoice → Text の順で投入
    queue
        .insert(Entry::Payload(0.0, CueCommand::Text("hello".into())))
        .unwrap();
    queue
        .insert(Entry::Payload(
            1.0,
            CueCommand::Choice {
                id: "a".into(),
                text: "Option A".into(),
            },
        ))
        .unwrap();
    queue
        .insert(Entry::Barrier(
            1.0,
            BarrierKind::WaitForChoice { timeout: None },
        ))
        .unwrap();
    queue
        .insert(Entry::Payload(2.0, CueCommand::Text("after choice".into())))
        .unwrap();

    // Phase 1: tick(0.5) → Text("hello") が ready に
    queue.tick(0.5);
    let cmds = queue.ready();
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], CueCommand::Text(t) if t == "hello"));
    assert_eq!(queue.state(), &CueQueueState::Playing);

    // Phase 2: tick(1.5) → Choice が pending_choices に蓄積、WaitForChoice でブロック
    queue.tick(1.5);
    assert_eq!(queue.state(), &CueQueueState::WaitingForChoice);
    assert_eq!(queue.pending_choices().len(), 1);
    assert_eq!(queue.pending_choices()[0].id, "a");
    // ready() は Choice 除外済みなので空
    assert!(queue.ready().is_empty());

    // Phase 3: resolve_choice で解除
    let result = queue.resolve_choice("a");
    assert_eq!(result, Some("a".to_string()));
    assert_eq!(queue.state(), &CueQueueState::Playing);

    // Phase 4: tick(2.5) → Text("after choice") が ready に
    queue.tick(2.5);
    let cmds = queue.ready();
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], CueCommand::Text(t) if t == "after choice"));

    // Phase 5: 全消費完了
    queue.tick(3.0);
    assert_eq!(queue.state(), &CueQueueState::Completed);
}

// ---------------------------------------------------------------
// WaitForInput バリアの tick → resolve_click フロー
// ---------------------------------------------------------------

#[test]
fn integration_wait_for_input_flow() {
    let mut queue = CueQueue::new();

    queue
        .insert(Entry::Payload(0.0, CueCommand::Text("line 1".into())))
        .unwrap();
    queue
        .insert(Entry::Barrier(
            1.0,
            BarrierKind::WaitForInput { timeout: Some(5.0) },
        ))
        .unwrap();
    queue
        .insert(Entry::Payload(2.0, CueCommand::Text("line 2".into())))
        .unwrap();

    // tick → バリアに到達
    queue.tick(1.5);
    assert_eq!(queue.state(), &CueQueueState::WaitingForClick);

    // タイムアウト未到達
    assert!(!queue.check_timeout(3.0)); // 3.0 - 1.5 = 1.5 < 5.0

    // タイムアウト到達
    assert!(queue.check_timeout(6.5)); // 6.5 - 1.5 = 5.0 >= 5.0

    // クリックで解除
    queue.resolve_click();
    assert_eq!(queue.state(), &CueQueueState::Playing);

    // 続行
    queue.tick(2.5);
    let cmds = queue.ready();
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], CueCommand::Text(t) if t == "line 2"));
}

// ---------------------------------------------------------------
// 新 CueSheet 投入による全破棄・再構築
// ---------------------------------------------------------------

#[test]
fn integration_reset_schedule_clears_and_rebuilds() {
    let mut queue = CueQueue::new();

    // 最初のスケジュール
    queue
        .insert(Entry::Payload(0.0, CueCommand::Text("old".into())))
        .unwrap();
    queue
        .insert(Entry::Barrier(
            1.0,
            BarrierKind::WaitForInput { timeout: None },
        ))
        .unwrap();

    // バリアに到達
    queue.tick(1.5);
    assert_eq!(queue.state(), &CueQueueState::WaitingForClick);

    // 新 CueSheet 投入 → 全破棄再構築
    queue.reset_schedule(10.0);
    assert_eq!(queue.state(), &CueQueueState::Playing);
    assert!(queue.is_empty());
    assert!(queue.pending_choices().is_empty());

    // 新しいコマンドを投入
    queue
        .insert(Entry::Payload(10.0, CueCommand::Text("new sheet".into())))
        .unwrap();
    queue
        .insert(Entry::Payload(11.0, CueCommand::Clear))
        .unwrap();

    // 新スケジュールが正常に動作する
    queue.tick(10.5);
    let cmds = queue.ready();
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], CueCommand::Text(t) if t == "new sheet"));

    queue.tick(11.5);
    let cmds = queue.ready();
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], CueCommand::Clear));

    // 全消費で Completed
    queue.tick(12.0);
    assert_eq!(queue.state(), &CueQueueState::Completed);
}

// ---------------------------------------------------------------
// Entity 参照のラウンドトリップ変換
// ---------------------------------------------------------------

#[test]
fn integration_entity_ref_roundtrip() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();

    let mut queue = CueQueue::new();

    // push 境界: Entity → u64
    queue.push_entity_command(0.0, entity).unwrap();

    // pop 境界: u64 → Entity
    let cmds = queue.pop_ready(1.0);
    assert_eq!(cmds.len(), 1);

    let restored = CueQueue::resolve_entity_ref(&cmds[0]);
    assert!(restored.is_some());
    assert_eq!(restored.unwrap(), entity);
}

#[test]
fn integration_entity_ref_non_entity_returns_none() {
    let cmd = CueCommand::Text("hello".into());
    assert!(CueQueue::resolve_entity_ref(&cmd).is_none());
}

// ---------------------------------------------------------------
// バリア中の新 CueSheet 強制切替
// ---------------------------------------------------------------

#[test]
fn integration_barrier_override_by_new_sheet() {
    let mut queue = CueQueue::new();

    // Choice 先積み → WaitForChoice
    queue
        .insert(Entry::Payload(
            0.0,
            CueCommand::Choice {
                id: "x".into(),
                text: "X".into(),
            },
        ))
        .unwrap();
    queue
        .insert(Entry::Barrier(
            0.0,
            BarrierKind::WaitForChoice { timeout: None },
        ))
        .unwrap();

    queue.tick(0.5);
    assert_eq!(queue.state(), &CueQueueState::WaitingForChoice);
    assert!(!queue.pending_choices().is_empty());

    // 新 CueSheet で強制上書き
    queue.reset_schedule(5.0);
    assert_eq!(queue.state(), &CueQueueState::Playing);
    assert!(queue.pending_choices().is_empty());
    assert!(queue.is_empty());

    // 新スケジュールを再投入して正常動作
    queue
        .insert(Entry::Payload(5.0, CueCommand::Text("override".into())))
        .unwrap();
    queue.tick(5.5);
    assert_eq!(queue.ready().len(), 1);
}
