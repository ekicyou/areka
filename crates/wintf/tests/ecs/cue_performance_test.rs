//! バルク操作の正当性テスト: キュー操作のスケール検証
//!
//! Task 4.11 — insert / pop_ready を 100 件・1000 件規模で実行し、
//! 大量コマンドが欠損・重複なく挿入・消費されることを検証する。
//!
//! NOTE(W8-T1 フレーキー安定化): 旧版は `std::time::Instant` による実時間ベンチ
//! （`elapsed < Nms` アサーション）で、負荷依存のタイミングフレーキーだった
//! （特に `bench_pop_ready_empty_queue` は CI / 高負荷環境で偽陽性）。
//! 実時間しきい値は OS スケジューラのプリエンプションに左右され、コードの正当性とは
//! 無関係に揺れるため、検証内容（同じ正当性 = 全件挿入・全件消費・空キュー無害）を
//! 維持したまま実時間アサーションを除去し、決定論的な件数・内容検証へ置換した。
//! 真のパフォーマンス回帰検知は criterion 等の専用ベンチが担うべきもので、
//! 単体テストのタイミングアサーションは回帰検知器として機能しない（NFR-1 の
//! 性能要件はベンチ層の責務）。

use wintf::ecs::cue::*;

/// テスト用 Entry を生成するヘルパー。
fn make_entry(start_time: f64) -> Entry<CueCommand> {
    Entry::Payload(start_time, CueCommand::Text("bench".into()))
}

// ---------------------------------------------------------------
// insert バルク正当性
// ---------------------------------------------------------------

/// insert 100 件: 全件がキューに残る。
#[test]
fn bulk_insert_100_all_retained() {
    let mut queue = CueQueue::with_capacity(10_000);
    let entries: Vec<Entry<CueCommand>> = (0..100).map(|i| make_entry(i as f64 * 0.01)).collect();

    for entry in entries {
        queue.insert(entry).unwrap();
    }

    assert_eq!(queue.len(), 100);
    assert!(!queue.is_empty());
}

/// insert 1000 件: 全件がキューに残る。
#[test]
fn bulk_insert_1000_all_retained() {
    let mut queue = CueQueue::with_capacity(10_000);
    let entries: Vec<Entry<CueCommand>> =
        (0..1000).map(|i| make_entry(i as f64 * 0.001)).collect();

    for entry in entries {
        queue.insert(entry).unwrap();
    }

    assert_eq!(queue.len(), 1000);
}

// ---------------------------------------------------------------
// pop_ready バルク正当性
// ---------------------------------------------------------------

/// pop_ready 100 件: 到達済み全件を欠損なく消費し、キューが空になる。
#[test]
fn bulk_pop_ready_100_drains_all() {
    let mut queue = CueQueue::with_capacity(10_000);
    for i in 0..100 {
        queue.insert(make_entry(i as f64 * 0.01)).unwrap();
    }

    // current_time を十分先に設定して全コマンドを ready にする
    let commands = queue.pop_ready(1000.0);

    assert_eq!(commands.len(), 100);
    assert!(queue.is_empty());
    // 消費後の状態は Completed
    assert_eq!(*queue.state(), CueQueueState::Completed);
}

/// pop_ready 1000 件: 到達済み全件を欠損なく消費し、キューが空になる。
#[test]
fn bulk_pop_ready_1000_drains_all() {
    let mut queue = CueQueue::with_capacity(10_000);
    for i in 0..1000 {
        queue.insert(make_entry(i as f64 * 0.001)).unwrap();
    }

    let commands = queue.pop_ready(1000.0);

    assert_eq!(commands.len(), 1000);
    assert!(queue.is_empty());
    // 全件が Text("bench") として復元される（型・内容の欠損なし）
    assert!(
        commands
            .iter()
            .all(|c| matches!(c, CueCommand::Text(t) if t == "bench")),
        "all popped commands must be the inserted Text payload"
    );
}

/// 部分到達: current_time 未満のエントリは残り、到達済みのみ消費される。
#[test]
fn bulk_pop_ready_partial_time_leaves_future_entries() {
    let mut queue = CueQueue::with_capacity(10_000);
    // offset = 0.0, 1.0, 2.0, ..., 99.0 の 100 件
    for i in 0..100 {
        queue.insert(make_entry(i as f64)).unwrap();
    }

    // t=9.5 → offset 0.0..=9.0 の 10 件が到達済み、残り 90 件
    let commands = queue.pop_ready(9.5);
    assert_eq!(commands.len(), 10);
    assert_eq!(queue.len(), 90);
    assert!(!queue.is_empty());
}

// ---------------------------------------------------------------
// 空キューの pop_ready は無害（決定論版）
// ---------------------------------------------------------------

/// 空 CueQueue の pop_ready を多数回呼んでも空を返し、状態が壊れない。
///
/// 旧版はこの繰り返しの実時間（`elapsed < 1ms`）を検査していたが、負荷依存で
/// フレーキーだった。回数ループの実時間ではなく「常に空を返す・パニックしない・
/// 状態が Playing のまま」という決定論的な正当性へ置換する。
#[test]
fn empty_queue_pop_ready_is_noop_and_stable() {
    let mut queue = CueQueue::new();

    for _ in 0..10_000 {
        let commands = queue.pop_ready(1000.0);
        assert!(commands.is_empty(), "empty queue must always pop empty");
    }

    // 多数回 pop 後も状態は健全（空のまま・パニックなし）。
    // 空キューは最初の tick で全消費扱いとなり Completed へ遷移する
    // （schedule.is_completed() == true）。以降の pop も空を返し続ける。
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
    assert_eq!(*queue.state(), CueQueueState::Completed);
}
