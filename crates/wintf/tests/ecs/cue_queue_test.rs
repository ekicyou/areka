//! ユニットテスト: CueQueue 基本操作の検証（Task 4.5）
//!
//! - insert() によるエントリ追加
//! - pop_ready() / tick()+ready() の時刻到達判定
//! - capacity 超過時の CapacityExceeded エラー
//! - 空キューの is_empty() / len()

use wintf::ecs::cue::*;

// ── insert + pop_ready ──

#[test]
fn insert_and_pop_ready_by_time() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(1.0, CueCommand::Text("first".into())))
        .unwrap();
    queue
        .insert(Entry::Payload(3.0, CueCommand::Text("third".into())))
        .unwrap();
    queue
        .insert(Entry::Payload(2.0, CueCommand::Text("second".into())))
        .unwrap();

    // pop_ready は start_time <= current_time のものを返す
    let pops = queue.pop_ready(1.0);
    assert_eq!(pops.len(), 1);
    if let CueCommand::Text(ref t) = pops[0] {
        assert_eq!(t, "first");
    } else {
        panic!("Expected Text command");
    }
}

#[test]
fn pop_ready_returns_all_ready_commands() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(1.0, CueCommand::Text("a".into())))
        .unwrap();
    queue
        .insert(Entry::Payload(2.0, CueCommand::Text("b".into())))
        .unwrap();
    queue
        .insert(Entry::Payload(5.0, CueCommand::Text("c".into())))
        .unwrap();

    // time=3.0: a(1.0) と b(2.0) が到達済み
    let pops = queue.pop_ready(3.0);
    assert_eq!(pops.len(), 2);

    // まだ c(5.0) が残っている
    assert_eq!(queue.len(), 1);
    assert!(!queue.is_empty());
}

#[test]
fn pop_ready_returns_empty_when_no_ready_commands() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(10.0, CueCommand::Text("future".into())))
        .unwrap();

    let pops = queue.pop_ready(1.0);
    assert!(pops.is_empty());
}

#[test]
fn tick_ready_two_phase_api() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(1.0, CueCommand::Text("hello".into())))
        .unwrap();

    queue.tick(1.0);
    let ready = queue.ready();
    assert_eq!(ready.len(), 1);
    if let CueCommand::Text(ref t) = ready[0] {
        assert_eq!(t, "hello");
    } else {
        panic!("Expected Text command");
    }

    // ready() は何度でも参照可能
    assert_eq!(queue.ready().len(), 1);
}

#[test]
fn capacity_exceeded_error() {
    let mut queue = CueQueue::with_capacity(2);
    queue
        .insert(Entry::Payload(1.0, CueCommand::Clear))
        .unwrap();
    queue
        .insert(Entry::Payload(2.0, CueCommand::Clear))
        .unwrap();

    let result = queue.insert(Entry::Payload(3.0, CueCommand::Clear));
    assert!(result.is_err());
    match result.unwrap_err() {
        CueSystemError::CapacityExceeded { capacity } => assert_eq!(capacity, 2),
        _ => panic!("Expected CapacityExceeded error"),
    }
}

#[test]
fn empty_queue_state() {
    let queue = CueQueue::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
    assert_eq!(*queue.state(), CueQueueState::Playing);
}

#[test]
fn extend_entries_works() {
    let mut queue = CueQueue::new();
    queue
        .extend_entries(vec![
            Entry::Payload(3.0, CueCommand::Text("c".into())),
            Entry::Payload(1.0, CueCommand::Text("a".into())),
            Entry::Payload(2.0, CueCommand::Text("b".into())),
        ])
        .unwrap();

    assert_eq!(queue.len(), 3);

    // pop_ready(1.0) で最小時刻のものだけ取得
    let pops = queue.pop_ready(1.0);
    assert_eq!(pops.len(), 1);
    if let CueCommand::Text(ref t) = pops[0] {
        assert_eq!(t, "a");
    } else {
        panic!("Expected Text command");
    }
}

#[test]
fn pop_ready_transitions_to_completed() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(1.0, CueCommand::Clear))
        .unwrap();

    let _ = queue.pop_ready(2.0);
    assert_eq!(*queue.state(), CueQueueState::Completed);
}

#[test]
fn pause_resume() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(1.0, CueCommand::Clear))
        .unwrap();

    queue.pause();
    assert_eq!(*queue.state(), CueQueueState::Paused);

    // Paused 中は消費しない
    let pops = queue.pop_ready(2.0);
    assert!(pops.is_empty());

    queue.resume();
    assert_eq!(*queue.state(), CueQueueState::Playing);

    let pops = queue.pop_ready(2.0);
    assert_eq!(pops.len(), 1);
}

#[test]
fn clear_resets_queue() {
    let mut queue = CueQueue::new();
    queue
        .insert(Entry::Payload(1.0, CueCommand::Clear))
        .unwrap();
    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(*queue.state(), CueQueueState::Playing);
}
