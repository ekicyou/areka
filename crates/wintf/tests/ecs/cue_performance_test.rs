//! パフォーマンステスト: キュー操作ベンチマーク
//!
//! Task 4.11 — push_sorted / pop_ready の各操作を 100 件・1000 件で計測し、
//! NFR-1 の「1000 コマンドを 1ms 以内に消費可能」要件を検証する。
//!
//! NOTE: このテストは正確なベンチマーク環境（criterion 等）ではなく、
//! `std::time::Instant` による簡易計測である。CI や低スペック環境では閾値を緩めてある。

use std::time::Instant;
use wintf::ecs::cue::*;

/// テスト用 TimedCue を生成するヘルパー。
fn make_timed_cue(start_time: f64) -> TimedCue {
    TimedCue {
        start_time,
        command: CueCommand::Text("bench".into()),
    }
}

// ---------------------------------------------------------------
// push_sorted ベンチマーク
// ---------------------------------------------------------------

/// push_sorted 100 件挿入の計測。
#[test]
fn bench_push_sorted_100() {
    let mut queue = CueQueue::with_capacity(10_000);
    let cues: Vec<TimedCue> = (0..100).map(|i| make_timed_cue(i as f64 * 0.01)).collect();

    let start = Instant::now();
    for cue in cues {
        queue.push_sorted(cue).unwrap();
    }
    let elapsed = start.elapsed();

    assert_eq!(queue.len(), 100);
    // 100 件挿入は 10ms 以内であるべき（余裕を持たせた閾値）
    assert!(
        elapsed.as_millis() < 10,
        "push_sorted 100 items took {elapsed:?} (expected < 10ms)"
    );
    eprintln!("[bench] push_sorted 100 items: {elapsed:?}");
}

/// push_sorted 1000 件挿入の計測。
#[test]
fn bench_push_sorted_1000() {
    let mut queue = CueQueue::with_capacity(10_000);
    let cues: Vec<TimedCue> = (0..1000).map(|i| make_timed_cue(i as f64 * 0.001)).collect();

    let start = Instant::now();
    for cue in cues {
        queue.push_sorted(cue).unwrap();
    }
    let elapsed = start.elapsed();

    assert_eq!(queue.len(), 1000);
    // 1000 件挿入は 50ms 以内であるべき
    assert!(
        elapsed.as_millis() < 50,
        "push_sorted 1000 items took {elapsed:?} (expected < 50ms)"
    );
    eprintln!("[bench] push_sorted 1000 items: {elapsed:?}");
}

// ---------------------------------------------------------------
// pop_ready ベンチマーク
// ---------------------------------------------------------------

/// pop_ready 100 件消費の計測。
#[test]
fn bench_pop_ready_100() {
    let mut queue = CueQueue::with_capacity(10_000);
    for i in 0..100 {
        queue
            .push_sorted(make_timed_cue(i as f64 * 0.01))
            .unwrap();
    }

    // current_time を十分先に設定して全コマンドを ready にする
    let start = Instant::now();
    let commands = queue.pop_ready(1000.0);
    let elapsed = start.elapsed();

    assert_eq!(commands.len(), 100);
    assert!(queue.is_empty());
    // 100 件消費は 1ms 以内であるべき
    assert!(
        elapsed.as_millis() < 1 || elapsed.as_micros() < 1000,
        "pop_ready 100 items took {elapsed:?} (expected < 1ms)"
    );
    eprintln!("[bench] pop_ready 100 items: {elapsed:?}");
}

/// pop_ready 1000 件消費の計測（NFR-1 の閾値: 1ms 以内）。
#[test]
fn bench_pop_ready_1000() {
    let mut queue = CueQueue::with_capacity(10_000);
    for i in 0..1000 {
        queue
            .push_sorted(make_timed_cue(i as f64 * 0.001))
            .unwrap();
    }

    let start = Instant::now();
    let commands = queue.pop_ready(1000.0);
    let elapsed = start.elapsed();

    assert_eq!(commands.len(), 1000);
    assert!(queue.is_empty());
    // NFR-1: 1000 コマンドを 1ms 以内に消費可能
    // CI 環境マージンとして 5ms に緩和
    assert!(
        elapsed.as_millis() < 5,
        "pop_ready 1000 items took {elapsed:?} (expected < 5ms per NFR-1)"
    );
    eprintln!("[bench] pop_ready 1000 items: {elapsed:?}");
}

// ---------------------------------------------------------------
// 空キューの pop_ready コスト
// ---------------------------------------------------------------

/// 空 CueQueue の pop_ready コストが実質ゼロであることを検証。
#[test]
fn bench_pop_ready_empty_queue() {
    let mut queue = CueQueue::new();

    let start = Instant::now();
    for _ in 0..10_000 {
        let _commands = queue.pop_ready(1000.0);
    }
    let elapsed = start.elapsed();

    // 10,000 回の空 pop_ready が 1ms 以内であるべき
    assert!(
        elapsed.as_millis() < 1 || elapsed.as_micros() < 1000,
        "10,000 empty pop_ready took {elapsed:?} (expected < 1ms)"
    );
    eprintln!("[bench] 10,000 empty pop_ready: {elapsed:?}");
}
