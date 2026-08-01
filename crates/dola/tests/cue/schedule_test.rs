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
// 占有 horizon と is_completed（占有終了判定・D6 / R2.5）
//
// is_completed は「entries 枯渇かつ barrier なしかつ current_offset >= horizon」。
// new(start) は horizon=0.0 を既定とし、current_offset は常に >=0 ゆえ horizon=0 は
// 素通り＝「entries 枯渇かつ barrier なし」の旧挙動に一致する（手組みスケジュールの後方互換）。
// ============================================================================

/// `new(start)` の既定 horizon は 0.0 ゆえ、is_completed は entry 枯渇で即真（旧挙動保存）。
#[test]
fn new_defaults_horizon_to_zero_preserving_legacy_is_completed() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "a".into()));
    assert!(!sched.is_completed());
    sched.tick(1.0);
    assert!(
        sched.is_completed(),
        "horizon=0.0（既定）では entry 枯渇で即完了（旧挙動）"
    );
}

/// horizon > 最終 entry offset のとき、entry を配り終えても current_offset が horizon に
/// 達するまで完了しない（占有終了判定）。
#[test]
fn with_horizon_defers_completion_until_current_offset_reaches_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(0.0, 5.0);
    sched.insert(Entry::Payload(1.0, "a".into()));

    sched.tick(1.0);
    assert_eq!(sched.remaining(), 0, "entry は配り終えた");
    assert!(
        !sched.is_completed(),
        "current_offset(1.0) < horizon(5.0) ゆえ未完了"
    );

    sched.tick(4.999);
    assert!(!sched.is_completed(), "horizon 直前は未完了");

    sched.tick(5.0);
    assert!(sched.is_completed(), "horizon 到達で完了");
}

/// horizon を超過していても barrier 中は完了しない（barrier 優先）。解除後に完了する。
#[test]
fn with_horizon_not_completed_while_barrier_active_even_past_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(0.0, 1.0);
    sched.insert(Entry::Barrier(
        0.5,
        BarrierKind::WaitForInput { timeout: None },
    ));

    sched.tick(2.0); // horizon(1.0) 超過だが barrier で停止
    assert!(sched.current_barrier().is_some());
    assert!(!sched.is_completed(), "barrier 中は horizon 超過でも未完了");

    sched.notify_barrier_resolved(None);
    assert!(
        sched.is_completed(),
        "解除後 entries 空・current_offset>=horizon で完了"
    );
}

/// `clear()` は horizon も 0.0 へリセットするため、clear 済みスケジュールは完了状態になる。
#[test]
fn clear_resets_horizon_so_cleared_schedule_is_completed() {
    let mut sched = TimedSchedule::<String>::with_horizon(0.0, 5.0);
    sched.insert(Entry::Payload(1.0, "a".into()));
    sched.tick(1.0);
    assert!(!sched.is_completed(), "horizon 5.0 未到達ゆえ未完了");

    sched.clear();
    assert!(
        sched.is_completed(),
        "clear は horizon をリセットし完了状態にする"
    );
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
// バリアテスト追加（D3-T）
// ============================================================================

#[test]
fn wait_for_choice_barrier_resolved_with_choice_id() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        0.5,
        BarrierKind::WaitForChoice { timeout: None },
    ));
    sched.insert(Entry::Payload(1.0, "after_choice".into()));

    sched.tick(0.5);
    assert!(matches!(
        sched.current_barrier(),
        Some(BarrierKind::WaitForChoice { .. })
    ));

    // 選択 ID 付きで解除（現行実装は choice_id を検査せず解除する）
    sched.notify_barrier_resolved(Some("yes".to_string()));
    assert!(sched.current_barrier().is_none());

    sched.tick(1.0);
    assert_eq!(sched.ready(), &["after_choice".to_string()]);
}

#[test]
fn input_barrier_with_timeout_skipped_when_jumped_past() {
    // バリア到達前にタイムアウト時刻を一気に飛び越えた場合、
    // WaitForInput { timeout } バリアは停止せずスキップされる
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForInput {
            timeout: Some(0.5),
        },
    ));
    sched.insert(Entry::Payload(2.0, "after".into()));

    sched.tick(2.0); // 1.0 + 0.5 = 1.5 を既に超過
    assert!(sched.current_barrier().is_none());
    assert_eq!(sched.ready(), &["after".to_string()]);
}

#[test]
fn barrier_resolved_externally_before_timeout() {
    // タイムアウト付きバリアでも外部解除が先行すれば即時に進行再開できる
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForInput {
            timeout: Some(10.0),
        },
    ));
    sched.insert(Entry::Payload(2.0, "after".into()));

    sched.tick(1.0);
    assert!(sched.current_barrier().is_some());

    sched.notify_barrier_resolved(None);
    assert!(sched.current_barrier().is_none());

    sched.tick(2.0); // タイムアウト時刻 11.0 を待たずに進行
    assert_eq!(sched.ready(), &["after".to_string()]);
}

#[test]
fn clear_resets_active_barrier() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        0.0,
        BarrierKind::WaitForInput { timeout: None },
    ));
    sched.tick(0.0);
    assert!(sched.current_barrier().is_some());

    sched.clear();
    assert!(sched.current_barrier().is_none());
    assert!(sched.is_completed());
    assert!(sched.ready().is_empty());
}

#[test]
fn routing_before_barrier_collected_then_stops() {
    // Routing はバリア手前で収集され、バリアで進行が停止し、後続 Payload は残る
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Routing(
        0.5,
        RoutingCommand::RouteRemove {
            target: CueTarget::Shell,
        },
    ));
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForInput { timeout: None },
    ));
    sched.insert(Entry::Payload(1.5, "after".into()));

    sched.tick(2.0);
    assert!(sched.next_routing().is_some());
    assert!(sched.current_barrier().is_some());
    assert!(sched.ready().is_empty());
    assert_eq!(sched.remaining(), 1); // "after" は未配信
}

// ============================================================================
// 同時刻エントリの順序（D3-T 特性化）
// ============================================================================

#[test]
fn insert_same_offset_payloads_delivered_fifo() {
    // insert() は同一オフセットで挿入順（FIFO）配信となる
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "a".into()));
    sched.insert(Entry::Payload(1.0, "b".into()));
    sched.insert(Entry::Payload(1.0, "c".into()));

    sched.tick(1.0);
    assert_eq!(
        sched.ready(),
        &["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn extend_same_offset_payloads_delivered_lifo() {
    // 特性化: extend() は安定降順ソート + 末尾 pop のため、同一オフセットでは
    // 挿入順と逆（LIFO）配信となる。insert() の FIFO と不整合がある点は
    // 既知の挙動として固定する（改善は挙動変更を伴うため提案記録 P22 を参照）。
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.extend(vec![
        Entry::Payload(1.0, "a".into()),
        Entry::Payload(1.0, "b".into()),
        Entry::Payload(1.0, "c".into()),
    ]);

    sched.tick(1.0);
    assert_eq!(
        sched.ready(),
        &["c".to_string(), "b".to_string(), "a".to_string()]
    );
}

// ============================================================================
// 非有限時刻の境界（D3-V 特性化）
// ============================================================================

#[test]
fn tick_with_nan_time_delivers_all_pending_payloads() {
    // 特性化: tick(NaN) は offset=NaN となり、負値ガード・冪等性ガード・
    // `entry_offset > offset` 比較がすべて false になるため、最初のバリアまでの
    // 全ペイロードが即時配信される（時刻入力の有限性検証の欠如は P25 を参照）。
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "a".into()));
    sched.insert(Entry::Payload(100.0, "b".into()));
    sched.insert(Entry::Payload(10000.0, "c".into()));

    sched.tick(f64::NAN);
    assert_eq!(
        sched.ready(),
        &["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(sched.remaining(), 0);
}

#[test]
fn tick_with_nan_time_stops_at_barrier_then_normal_tick_recovers() {
    // 特性化: tick(NaN) でもバリアでは停止する（バリア設定経路は比較に依存しない）。
    // その後の正常時刻 tick は冪等性ガード（x <= NaN = false）を素通りして
    // 通常進行へ復帰する。
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "before".into()));
    sched.insert(Entry::Barrier(2.0, BarrierKind::WaitForInput { timeout: None }));
    sched.insert(Entry::Payload(3.0, "after".into()));

    sched.tick(f64::NAN);
    assert_eq!(sched.ready(), &["before".to_string()]);
    assert!(sched.current_barrier().is_some());
    assert_eq!(sched.remaining(), 1);

    // バリア解除後、正常時刻で進行再開できる（current_offset=NaN から復帰）
    sched.notify_barrier_resolved(None);
    sched.tick(3.0);
    assert_eq!(sched.ready(), &["after".to_string()]);
    assert!(sched.is_completed());
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Entry offset must be non-negative")]
fn insert_nan_offset_fires_debug_assert_in_debug_builds() {
    // 特性化: NaN オフセットは `offset >= 0.0` が false となるため、debug ビルド
    // では insert() の debug_assert が発火する（release では素通りし partition_point
    // の前提を破る — src/cue/schedule.rs の NOTE および P25 を参照）。
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(f64::NAN, "x".into()));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Entry offset must be non-negative")]
fn extend_nan_offset_fires_debug_assert_in_debug_builds() {
    // 特性化: extend() も insert() と同一の debug_assert で NaN を検出する。
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.extend(vec![Entry::Payload(f64::NAN, "x".into())]);
}

// ============================================================================
// Entry アクセサ（D3-T）
// ============================================================================

#[test]
fn entry_offset_accessor_for_all_variants() {
    assert_eq!(Entry::Payload(1.5, "x".to_string()).offset(), 1.5);
    assert_eq!(
        Entry::<String>::Barrier(2.5, BarrierKind::WaitForInput { timeout: None }).offset(),
        2.5
    );
    assert_eq!(
        Entry::<String>::Routing(
            3.5,
            RoutingCommand::RouteRemove {
                target: CueTarget::Shell,
            }
        )
        .offset(),
        3.5
    );
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

// ============================================================================
// 占有 horizon の絶対時刻照会（Task 3.1・R7.2）
// ============================================================================

/// `occupancy_horizon()` はアンカー（`start_time`）＋相対 horizon の**和**を返す。
/// タイムアウト計測の起点はこの duration 権威から取得され、独自の時間基準を作らない（R7.2）。
#[test]
fn occupancy_horizon_returns_anchor_plus_relative_horizon() {
    let sched = TimedSchedule::<String>::with_horizon(10.0, 2.5);
    assert_eq!(
        sched.occupancy_horizon(),
        12.5,
        "アンカー 10.0 ＋ 相対 horizon 2.5 = 12.5"
    );
}

/// horizon 既定（`new` = 0.0）ではアンカーそのものを返す（手組みスケジュールの後方互換）。
#[test]
fn occupancy_horizon_of_default_schedule_is_the_anchor() {
    let sched = TimedSchedule::<String>::new(7.25);
    assert_eq!(
        sched.occupancy_horizon(),
        7.25,
        "horizon=0.0 既定ゆえアンカーそのもの"
    );
}

/// アンカー未刻印（0.0）では相対 horizon がそのまま絶対値になる。
#[test]
fn occupancy_horizon_without_anchor_is_the_relative_horizon() {
    let sched = TimedSchedule::<String>::with_horizon(0.0, 3.0);
    assert_eq!(sched.occupancy_horizon(), 3.0);
}

/// `occupancy_horizon()` は tick で進行しても不変（配送位置でなく占有区間の終端を指す）。
#[test]
fn occupancy_horizon_is_invariant_across_ticks() {
    let mut sched = TimedSchedule::<String>::with_horizon(100.0, 2.0);
    sched.insert(Entry::Payload(0.5, "a".into()));
    assert_eq!(sched.occupancy_horizon(), 102.0);

    sched.tick(100.5);
    assert_eq!(sched.ready(), &["a".to_string()]);
    assert_eq!(
        sched.occupancy_horizon(),
        102.0,
        "配送が進んでも占有 horizon の絶対時刻は動かない"
    );
}

/// `occupancy_horizon()` は `is_completed()` の閾値と一致する（同一の horizon 権威）。
#[test]
fn occupancy_horizon_matches_is_completed_threshold() {
    let mut sched = TimedSchedule::<String>::with_horizon(100.0, 2.0);
    sched.insert(Entry::Payload(0.5, "a".into()));

    sched.tick(100.5);
    assert!(!sched.is_completed(), "horizon 未到達では完了しない");

    sched.tick(sched.occupancy_horizon());
    assert!(
        sched.is_completed(),
        "occupancy_horizon() が返す絶対時刻の到達で完了する"
    );
}
