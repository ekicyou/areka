//! ユニットテスト: CueQueue 基本操作の検証（Task 4.5）
//!
//! - push_sorted() の降順ソート維持
//! - pop_ready() の時刻到達判定
//! - peek() の非破壊参照
//! - capacity 超過時の CapacityExceeded エラー
//! - 空キューの is_empty() / len()

use wintf::ecs::cue::*;

// ── push_sorted ──

#[test]
fn push_sorted_maintains_descending_order() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Text("first".into()),
        })
        .unwrap();
    queue
        .push_sorted(TimedCue {
            start_time: 3.0,
            command: CueCommand::Text("third".into()),
        })
        .unwrap();
    queue
        .push_sorted(TimedCue {
            start_time: 2.0,
            command: CueCommand::Text("second".into()),
        })
        .unwrap();

    // peek() は最小 start_time（末尾）を返す
    let first = queue.peek().unwrap();
    assert_eq!(first.start_time, 1.0);

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
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Text("a".into()),
        })
        .unwrap();
    queue
        .push_sorted(TimedCue {
            start_time: 2.0,
            command: CueCommand::Text("b".into()),
        })
        .unwrap();
    queue
        .push_sorted(TimedCue {
            start_time: 5.0,
            command: CueCommand::Text("c".into()),
        })
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
        .push_sorted(TimedCue {
            start_time: 10.0,
            command: CueCommand::Text("future".into()),
        })
        .unwrap();

    let pops = queue.pop_ready(1.0);
    assert!(pops.is_empty());
}

#[test]
fn peek_is_non_destructive() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Text("hello".into()),
        })
        .unwrap();

    assert!(queue.peek().is_some());
    assert!(queue.peek().is_some());
    assert_eq!(queue.len(), 1);
}

#[test]
fn capacity_exceeded_error() {
    let mut queue = CueQueue::with_capacity(2);
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Clear,
        })
        .unwrap();
    queue
        .push_sorted(TimedCue {
            start_time: 2.0,
            command: CueCommand::Clear,
        })
        .unwrap();

    let result = queue.push_sorted(TimedCue {
        start_time: 3.0,
        command: CueCommand::Clear,
    });
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
    assert!(queue.peek().is_none());
    assert_eq!(*queue.state(), CueQueueState::Playing);
}

#[test]
fn extend_sorted_works() {
    let mut queue = CueQueue::new();
    queue
        .extend_sorted(vec![
            TimedCue {
                start_time: 3.0,
                command: CueCommand::Text("c".into()),
            },
            TimedCue {
                start_time: 1.0,
                command: CueCommand::Text("a".into()),
            },
            TimedCue {
                start_time: 2.0,
                command: CueCommand::Text("b".into()),
            },
        ])
        .unwrap();

    assert_eq!(queue.len(), 3);
    // peek は最小 start_time
    assert_eq!(queue.peek().unwrap().start_time, 1.0);
}

#[test]
fn pop_ready_transitions_to_completed() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Clear,
        })
        .unwrap();

    let _ = queue.pop_ready(2.0);
    assert_eq!(*queue.state(), CueQueueState::Completed);
}

#[test]
fn pause_resume() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Clear,
        })
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
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::Clear,
        })
        .unwrap();
    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(*queue.state(), CueQueueState::Playing);
}
