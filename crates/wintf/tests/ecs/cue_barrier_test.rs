//! ユニットテスト: バリアプロトコルの検証（Task 4.6）
//!
//! - Choice 先積み + WaitForChoice ブロック遷移
//! - WaitForChoice 空打ちで EmptyChoiceBarrier エラー
//! - WaitForClick ブロック + resolve_click() 解除
//! - check_timeout() のタイムアウト判定
//! - skip_barrier() の強制スキップ

use wintf::ecs::cue::*;

// ── Choice + WaitForChoice ──

#[test]
fn choice_accumulation_and_wait_for_choice_barrier() {
    let mut queue = CueQueue::new();
    queue
        .extend_sorted(vec![
            TimedCue {
                start_time: 1.0,
                command: CueCommand::Choice {
                    id: "a".into(),
                    text: "Option A".into(),
                },
            },
            TimedCue {
                start_time: 1.0,
                command: CueCommand::Choice {
                    id: "b".into(),
                    text: "Option B".into(),
                },
            },
            TimedCue {
                start_time: 2.0,
                command: CueCommand::WaitForChoice { timeout: None },
            },
        ])
        .unwrap();

    // t=1.5: Choice は pending_choices に蓄積（返却されない）
    let pops = queue.pop_ready(1.5);
    assert!(pops.is_empty());
    assert_eq!(queue.pending_choices().len(), 2);
    assert_eq!(*queue.state(), CueQueueState::Playing);

    // t=2.5: WaitForChoice でブロック
    let pops = queue.pop_ready(2.5);
    assert!(pops.is_empty());
    assert_eq!(*queue.state(), CueQueueState::WaitingForChoice);

    // resolve_choice で解除
    let result = queue.resolve_choice("a");
    assert_eq!(result, Some("a".to_string()));
    assert_eq!(*queue.state(), CueQueueState::Completed); // 以降コマンドなし
}

#[test]
fn wait_for_choice_without_preceding_choices_is_error() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::WaitForChoice { timeout: None },
        })
        .unwrap();

    let pops = queue.pop_ready(2.0);
    assert!(pops.is_empty());
    match queue.state() {
        CueQueueState::Error(CueSystemError::EmptyChoiceBarrier { .. }) => {}
        other => panic!("Expected EmptyChoiceBarrier error, got {:?}", other),
    }
}

#[test]
fn resolve_choice_with_wrong_id_returns_none() {
    let mut queue = CueQueue::new();
    queue
        .extend_sorted(vec![
            TimedCue {
                start_time: 1.0,
                command: CueCommand::Choice {
                    id: "a".into(),
                    text: "A".into(),
                },
            },
            TimedCue {
                start_time: 2.0,
                command: CueCommand::WaitForChoice { timeout: None },
            },
        ])
        .unwrap();

    queue.pop_ready(1.5);
    queue.pop_ready(2.5);
    assert_eq!(*queue.state(), CueQueueState::WaitingForChoice);

    let result = queue.resolve_choice("nonexistent");
    assert_eq!(result, None);
    // 状態は変わらない
    assert_eq!(*queue.state(), CueQueueState::WaitingForChoice);
}

// ── WaitForClick ──

#[test]
fn wait_for_click_barrier_and_resolve() {
    let mut queue = CueQueue::new();
    queue
        .extend_sorted(vec![
            TimedCue {
                start_time: 1.0,
                command: CueCommand::Text("hello".into()),
            },
            TimedCue {
                start_time: 2.0,
                command: CueCommand::WaitForClick { timeout: None },
            },
            TimedCue {
                start_time: 3.0,
                command: CueCommand::Text("world".into()),
            },
        ])
        .unwrap();

    // t=1.5: Text消費
    let pops = queue.pop_ready(1.5);
    assert_eq!(pops.len(), 1);

    // t=2.5: WaitForClick でブロック
    let pops = queue.pop_ready(2.5);
    assert!(pops.is_empty());
    assert_eq!(*queue.state(), CueQueueState::WaitingForClick);

    // ブロック中は消費しない
    let pops = queue.pop_ready(3.5);
    assert!(pops.is_empty());

    // resolve_click() で解除
    queue.resolve_click();
    assert_eq!(*queue.state(), CueQueueState::Playing);

    // 残りのコマンドが消費可能に
    let pops = queue.pop_ready(3.5);
    assert_eq!(pops.len(), 1);
}

#[test]
fn pending_barrier_kind() {
    let mut queue = CueQueue::new();
    assert!(queue.pending_barrier_kind().is_none());

    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::WaitForClick { timeout: None },
        })
        .unwrap();

    queue.pop_ready(2.0);
    assert_eq!(
        queue.pending_barrier_kind(),
        Some(&BarrierKind::Click)
    );
}

// ── タイムアウト ──

#[test]
fn check_timeout_returns_true_when_expired() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::WaitForClick {
                timeout: Some(5.0),
            },
        })
        .unwrap();

    queue.pop_ready(1.0);
    assert_eq!(*queue.state(), CueQueueState::WaitingForClick);

    // まだタイムアウトしていない
    assert!(!queue.check_timeout(3.0));
    // タイムアウト
    assert!(queue.check_timeout(6.0));
    // ちょうど5秒 = タイムアウト
    assert!(queue.check_timeout(6.0));
}

#[test]
fn no_timeout_when_not_set() {
    let mut queue = CueQueue::new();
    queue
        .push_sorted(TimedCue {
            start_time: 1.0,
            command: CueCommand::WaitForClick { timeout: None },
        })
        .unwrap();

    queue.pop_ready(1.0);
    assert!(!queue.check_timeout(1000.0));
}

// ── skip_barrier ──

#[test]
fn skip_barrier_clears_all_barrier_state() {
    let mut queue = CueQueue::new();
    queue
        .extend_sorted(vec![
            TimedCue {
                start_time: 1.0,
                command: CueCommand::Choice {
                    id: "x".into(),
                    text: "X".into(),
                },
            },
            TimedCue {
                start_time: 2.0,
                command: CueCommand::WaitForChoice { timeout: None },
            },
            TimedCue {
                start_time: 3.0,
                command: CueCommand::Text("after".into()),
            },
        ])
        .unwrap();

    queue.pop_ready(1.5); // Choice 蓄積
    queue.pop_ready(2.5); // WaitForChoice でブロック
    assert_eq!(*queue.state(), CueQueueState::WaitingForChoice);
    assert_eq!(queue.pending_choices().len(), 1);

    queue.skip_barrier();
    assert_eq!(*queue.state(), CueQueueState::Playing);
    assert!(queue.pending_barrier_kind().is_none());
    // pending_choices はクリアされる
    assert!(queue.pending_choices().is_empty());

    // 後続コマンドが消費可能
    let pops = queue.pop_ready(3.5);
    assert_eq!(pops.len(), 1);
}
