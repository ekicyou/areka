//! TimedSchedule<T> ユニットテスト
//!
//! Task 6.2: tick/ready 2 フェーズ API、バリア管理、ルーティング収集の検証。

use dola::cue::{
    ActorKey, BarrierKind, CueCommand, CueTarget, Entry, EntityKey, RoutingCommand,
    TimedSchedule,
};

// ============================================================================
// Payload 収集テスト
// ============================================================================

#[test]
fn tick_collects_payloads_at_reached_time() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(0.5, "a".into()));
    sched.insert(Entry::Payload(1.0, "b".into()));
    sched.insert(Entry::Payload(2.0, "c".into()));

    sched.tick(0.5);
    assert_eq!(sched.ready(), &["a".to_string()]);

    sched.tick(1.0);
    assert_eq!(sched.ready(), &["b".to_string()]);

    sched.tick(2.0);
    assert_eq!(sched.ready(), &["c".to_string()]);
}

#[test]
fn tick_collects_multiple_payloads_at_same_time() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "first".into()));
    sched.insert(Entry::Payload(1.0, "second".into()));

    sched.tick(1.0);
    let ready = sched.ready();
    assert_eq!(ready.len(), 2);
    assert!(ready.contains(&"first".to_string()));
    assert!(ready.contains(&"second".to_string()));
}

#[test]
fn tick_collects_all_past_payloads_on_jump() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(0.1, "a".into()));
    sched.insert(Entry::Payload(0.2, "b".into()));
    sched.insert(Entry::Payload(0.3, "c".into()));

    // 一気に 0.3 まで飛ぶ → 全部収集
    sched.tick(0.3);
    assert_eq!(sched.ready().len(), 3);
}

#[test]
fn tick_idempotent_same_time() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "x".into()));

    sched.tick(1.0);
    assert_eq!(sched.ready(), &["x".to_string()]);

    // 同一時刻で再呼び出し → バッファ変更なし
    sched.tick(1.0);
    assert_eq!(sched.ready(), &["x".to_string()]);
}

#[test]
fn tick_before_start_time_is_noop() {
    let mut sched = TimedSchedule::<String>::new(10.0);
    sched.insert(Entry::Payload(0.0, "x".into()));

    sched.tick(5.0); // start_time=10 なので offset=-5 → noop
    assert!(sched.ready().is_empty());
}

#[test]
fn tick_absolute_time_to_relative_offset() {
    let mut sched = TimedSchedule::<String>::new(100.0);
    sched.insert(Entry::Payload(0.5, "a".into()));
    sched.insert(Entry::Payload(1.0, "b".into()));

    sched.tick(100.5); // offset=0.5
    assert_eq!(sched.ready(), &["a".to_string()]);

    sched.tick(101.0); // offset=1.0
    assert_eq!(sched.ready(), &["b".to_string()]);
}

#[test]
fn remaining_and_is_completed() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "a".into()));
    sched.insert(Entry::Payload(2.0, "b".into()));

    assert_eq!(sched.remaining(), 2);
    assert!(!sched.is_completed());

    sched.tick(1.0);
    assert_eq!(sched.remaining(), 1);

    sched.tick(2.0);
    assert_eq!(sched.remaining(), 0);
    assert!(sched.is_completed());
}

#[test]
fn clear_resets_all_state() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "a".into()));
    sched.tick(1.0);
    assert!(!sched.ready().is_empty());

    sched.clear();
    assert!(sched.ready().is_empty());
    assert_eq!(sched.remaining(), 0);
    assert!(sched.is_completed());
}

// ============================================================================
// バリアテスト
// ============================================================================

#[test]
fn barrier_stops_progress() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(0.5, "before".into()));
    sched.insert(Entry::Barrier(1.0, BarrierKind::WaitForInput { timeout: None }));
    sched.insert(Entry::Payload(1.5, "after".into()));

    // 0.5 到達 → Payload 収集
    sched.tick(0.5);
    assert_eq!(sched.ready(), &["before".to_string()]);
    assert!(sched.current_barrier().is_none());

    // 1.0 到達 → バリアで停止、"after" は未配信
    sched.tick(1.0);
    assert!(sched.current_barrier().is_some());
    assert_eq!(sched.remaining(), 1); // "after" が残っている

    // バリア中に tick しても進行しない
    sched.tick(2.0);
    assert!(sched.current_barrier().is_some());
    assert_eq!(sched.remaining(), 1);
}

#[test]
fn barrier_resolved_resumes_progress() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(0.5, BarrierKind::WaitForInput { timeout: None }));
    sched.insert(Entry::Payload(1.0, "after_barrier".into()));

    sched.tick(0.5);
    assert!(sched.current_barrier().is_some());

    // バリア解除
    sched.notify_barrier_resolved(None);
    assert!(sched.current_barrier().is_none());

    // 次の tick で進行再開
    sched.tick(1.0);
    assert_eq!(sched.ready(), &["after_barrier".to_string()]);
}

#[test]
fn barrier_timeout_auto_releases() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForInput {
            timeout: Some(2.0),
        },
    ));
    sched.insert(Entry::Payload(4.0, "after_timeout".into()));

    sched.tick(1.0);
    assert!(sched.current_barrier().is_some());

    // タイムアウト時刻 (1.0 + 2.0 = 3.0) に達する
    sched.tick(3.0);
    assert!(sched.current_barrier().is_none());

    // 後続のペイロードが取得可能
    sched.tick(4.0);
    assert_eq!(sched.ready(), &["after_timeout".to_string()]);
}

#[test]
fn timeout_barrier_auto_releases() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(1.0, BarrierKind::Timeout { duration: 0.5 }));
    sched.insert(Entry::Payload(2.0, "after".into()));

    sched.tick(1.0);
    assert!(sched.current_barrier().is_some());

    // duration 経過 (1.0 + 0.5 = 1.5) で自動解除
    sched.tick(1.5);
    assert!(sched.current_barrier().is_none());

    sched.tick(2.0);
    assert_eq!(sched.ready(), &["after".to_string()]);
}

#[test]
fn timeout_barrier_skipped_when_already_past() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(1.0, BarrierKind::Timeout { duration: 0.5 }));
    sched.insert(Entry::Payload(2.0, "target".into()));

    // 一気に 2.0 まで飛ぶ → Timeout はスキップ
    sched.tick(2.0);
    assert!(sched.current_barrier().is_none());
    assert_eq!(sched.ready(), &["target".to_string()]);
}

#[test]
fn barrier_not_completed_while_active() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(0.0, BarrierKind::WaitForInput { timeout: None }));

    sched.tick(0.0);
    assert!(sched.current_barrier().is_some());
    // entries は空だが barrier 中なので completed ではない
    assert!(!sched.is_completed());

    sched.notify_barrier_resolved(None);
    assert!(sched.is_completed());
}

#[test]
fn notify_barrier_resolved_noop_when_no_barrier() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "x".into()));

    // バリアなし状態で notify → パニックしない
    sched.notify_barrier_resolved(None);
    assert!(sched.current_barrier().is_none());
}

// ============================================================================
// ルーティングテスト
// ============================================================================

#[test]
fn routing_collected_without_stopping() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    let routing = RoutingCommand::RouteAdd {
        target: CueTarget::Balloon,
        to: EntityKey::Actor(ActorKey::from("sakura"), CueTarget::Balloon),
    };
    sched.insert(Entry::Routing(0.5, routing.clone()));
    sched.insert(Entry::Payload(1.0, "data".into()));

    sched.tick(1.0);
    // Payload は ready に
    assert_eq!(sched.ready(), &["data".to_string()]);
    // Routing は next_routing に
    let r = sched.next_routing().expect("routing should be available");
    assert_eq!(r, routing);
}

#[test]
fn routing_not_in_ready() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Routing(
        0.5,
        RoutingCommand::RouteRemove {
            target: CueTarget::Shell,
        },
    ));

    sched.tick(1.0);
    // ready にはルーティングが混入しない
    assert!(sched.ready().is_empty());
    // next_routing で取れる
    assert!(sched.next_routing().is_some());
}

#[test]
fn multiple_routings_fifo_order() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    let r1 = RoutingCommand::RouteAdd {
        target: CueTarget::Shell,
        to: EntityKey::Spot("spot1".into()),
    };
    let r2 = RoutingCommand::RouteSwitch {
        target: CueTarget::Balloon,
        to: EntityKey::Balloon("balloon1".into()),
    };
    let r3 = RoutingCommand::RouteRemove {
        target: CueTarget::Shell,
    };
    sched.insert(Entry::Routing(0.1, r1.clone()));
    sched.insert(Entry::Routing(0.2, r2.clone()));
    sched.insert(Entry::Routing(0.3, r3.clone()));

    sched.tick(1.0);

    assert_eq!(sched.next_routing(), Some(r1));
    assert_eq!(sched.next_routing(), Some(r2));
    assert_eq!(sched.next_routing(), Some(r3));
    assert_eq!(sched.next_routing(), None);
}

// ============================================================================
// CueCommand ペイロードでの統合検証
// ============================================================================

#[test]
fn timed_schedule_with_cue_command() {
    let mut sched = TimedSchedule::<CueCommand>::new(0.0);
    sched.insert(Entry::Payload(0.0, CueCommand::Text("hello".into())));
    sched.insert(Entry::Payload(0.5, CueCommand::Emote { key: "smile".into() }));
    sched.insert(Entry::Payload(1.0, CueCommand::Clear));

    sched.tick(0.0);
    assert_eq!(sched.ready().len(), 1);
    assert!(matches!(sched.ready()[0], CueCommand::Text(_)));

    sched.tick(0.5);
    assert_eq!(sched.ready().len(), 1);
    assert!(matches!(sched.ready()[0], CueCommand::Emote { .. }));

    sched.tick(1.0);
    assert!(matches!(sched.ready()[0], CueCommand::Clear));
}

// ============================================================================
// extend テスト
// ============================================================================

#[test]
fn extend_inserts_multiple_entries() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.extend(vec![
        Entry::Payload(2.0, "second".into()),
        Entry::Payload(1.0, "first".into()),
    ]);

    sched.tick(1.0);
    assert_eq!(sched.ready(), &["first".to_string()]);

    sched.tick(2.0);
    assert_eq!(sched.ready(), &["second".to_string()]);
}
