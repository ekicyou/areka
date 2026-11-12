//! TimedSchedule<T> の tick/ready 2 フェーズ API とバリア管理の検証。
//!
//! 手組みの `new`／`insert` はクレートの中だけの口なので、ここに置く。

use super::*;
use crate::cue::CueCommand;

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

// ============================================================================
// 占有 horizon と is_completed（占有終了判定・D6 / R2.5）
// is_completed は「entries 枯渇かつ barrier なしかつ current_time >= start_time + horizon」。
// ============================================================================

/// horizon > 最終 entry offset のとき、entry を配り終えても開始からの経過が horizon に
/// 達するまで完了しない（占有終了判定）。
#[test]
fn with_horizon_defers_completion_until_elapsed_time_reaches_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(0.0, 5.0);
    sched.insert(Entry::Payload(1.0, "a".into()));

    sched.tick(1.0);
    assert_eq!(sched.remaining(), 0, "entry は配り終えた");
    assert!(
        !sched.is_completed(),
        "開始からの経過(1.0) < horizon(5.0) ゆえ未完了"
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
    sched.insert(Entry::Barrier(0.5, BarrierKind::WaitForInput));

    sched.tick(2.0); // horizon(1.0) 超過だが barrier で停止
    assert!(sched.current_barrier().is_some());
    assert!(!sched.is_completed(), "barrier 中は horizon 超過でも未完了");

    sched.notify_barrier_resolved();
    assert!(
        sched.is_completed(),
        "解除後 entries 空・開始からの経過>=horizon で完了"
    );
}

// ============================================================================
// バリアテスト
// ============================================================================

#[test]
fn barrier_stops_progress() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(0.5, "before".into()));
    sched.insert(Entry::Barrier(1.0, BarrierKind::WaitForInput));
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
    sched.insert(Entry::Barrier(0.5, BarrierKind::WaitForInput));
    sched.insert(Entry::Payload(1.0, "after_barrier".into()));

    sched.tick(0.5);
    assert!(sched.current_barrier().is_some());

    // バリア解除
    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());

    // 次の tick で進行再開
    sched.tick(1.0);
    assert_eq!(sched.ready(), &["after_barrier".to_string()]);
}

#[test]
fn notify_barrier_resolved_noop_when_no_barrier() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "x".into()));

    // バリアなしで届いた後に解決しても、取り出しは変わらない
    sched.tick(1.0);
    sched.notify_barrier_resolved();
    assert_eq!(sched.ready(), &["x".to_string()]);
    assert!(sched.current_barrier().is_none());
}

// ============================================================================
// バリアのテスト
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

    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());

    sched.tick(1.0);
    assert_eq!(sched.ready(), &["after_choice".to_string()]);
}

/// 解決の直後に同じ時刻で tick しても（本番の drive は解決の後すぐ同じ時刻で回す）、
/// 冪等の守りに止められず区切りの後ろだけを集める。
#[test]
fn resolve_then_same_time_tick_collects_after() {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(1.0, "payload".into()));
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForChoice { timeout: None },
    ));
    sched.insert(Entry::Payload(1.0, "after".into()));

    sched.tick(1.0);
    assert_eq!(sched.ready(), &["payload".to_string()]);
    assert!(sched.current_barrier().is_some());

    sched.notify_barrier_resolved();
    sched.tick(1.0);
    assert_eq!(sched.ready(), &["after".to_string()]);
}

// ============================================================================
// 選択待ちの区切りの値は再生層が解かない（areka-P0-choice-timeout-directive 要件 5.1〜5.3）
// ============================================================================

/// 値 `timeout` の選択待ちの区切りを 1.0 に置き、`tick(1.0)` で区切りに着いたとき
/// 止まって後続を配らないことを確かめる（値は上位層へ運ぶ指令で、区切りを飛ばす理由にしない）。
fn assert_choice_barrier_stops_on_arrival(timeout: Option<f64>) {
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(1.0, BarrierKind::WaitForChoice { timeout }));
    sched.insert(Entry::Payload(1.0, "after".into()));

    sched.tick(1.0);
    assert_eq!(
        sched.current_barrier(),
        Some(&BarrierKind::WaitForChoice { timeout })
    );
    assert!(sched.ready().is_empty());
}

#[test]
fn choice_barrier_with_zero_timeout_stops_on_arrival() {
    // 値 0（時間切れなしの指令）でも区切りを飛ばさない（要件 5.1）
    assert_choice_barrier_stops_on_arrival(Some(0.0));
}

#[test]
fn choice_barrier_with_negative_timeout_stops_on_arrival() {
    // 負の値（時間切れなしの指令）でも区切りを飛ばさない（要件 5.1）
    assert_choice_barrier_stops_on_arrival(Some(-0.001));
}

#[test]
fn choice_barrier_with_timeout_not_skipped_when_jumped_past() {
    // 区切り＋値（1.0 + 0.5 = 1.5）を一度に越えてから着いても止まる（要件 5.2）
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForChoice { timeout: Some(0.5) },
    ));
    sched.insert(Entry::Payload(2.0, "after".into()));

    sched.tick(2.0);
    assert_eq!(
        sched.current_barrier(),
        Some(&BarrierKind::WaitForChoice { timeout: Some(0.5) })
    );
    assert!(sched.ready().is_empty());
}

#[test]
fn choice_barrier_with_timeout_never_self_releases_only_external_resolve() {
    // 止まった後に時計を大きく進めても自分では解かない（要件 5.3）。
    // 解けるのは外からの解決（notify_barrier_resolved）だけ
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Barrier(
        1.0,
        BarrierKind::WaitForChoice { timeout: Some(0.5) },
    ));
    sched.insert(Entry::Payload(2.0, "after".into()));

    sched.tick(1.0);
    assert!(sched.current_barrier().is_some());

    sched.tick(10.0);
    assert_eq!(
        sched.current_barrier(),
        Some(&BarrierKind::WaitForChoice { timeout: Some(0.5) })
    );
    assert!(sched.ready().is_empty());

    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());

    sched.tick(10.5);
    assert_eq!(sched.ready(), &["after".to_string()]);
}

// ============================================================================
// 同時刻エントリの順序
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

// ============================================================================
// 非有限時刻の境界
// ============================================================================

#[test]
fn tick_with_nan_time_delivers_all_pending_payloads() {
    // 特性化: tick(NaN) は offset=NaN となり、負値ガード・冪等性ガード・
    // `entry_offset > offset` 比較がすべて false になるため、最初のバリアまでの
    // 全ペイロードが即時配信される（時刻入力の有限性は検証しない）。
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
    sched.insert(Entry::Barrier(2.0, BarrierKind::WaitForInput));
    sched.insert(Entry::Payload(3.0, "after".into()));

    sched.tick(f64::NAN);
    assert_eq!(sched.ready(), &["before".to_string()]);
    assert!(sched.current_barrier().is_some());
    assert_eq!(sched.remaining(), 1);

    // バリア解除後、正常時刻で進行再開できる（current_time=NaN から復帰）
    sched.notify_barrier_resolved();
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
    // の前提を破る — src/cue/schedule.rs の NOTE を参照）。
    let mut sched = TimedSchedule::<String>::new(0.0);
    sched.insert(Entry::Payload(f64::NAN, "x".into()));
}

// ============================================================================
// Entry アクセサ
// ============================================================================

#[test]
fn entry_offset_accessor_for_all_variants() {
    assert_eq!(Entry::Payload(1.5, "x".to_string()).offset(), 1.5);
    assert_eq!(
        Entry::<String>::Barrier(2.5, BarrierKind::WaitForInput).offset(),
        2.5
    );
}

// ============================================================================
// CueCommand ペイロードでの統合検証
// ============================================================================

#[test]
fn timed_schedule_with_cue_command() {
    let mut sched = TimedSchedule::<CueCommand>::new(0.0);
    sched.insert(Entry::Payload(0.0, CueCommand::Text("hello".into())));
    sched.insert(Entry::Payload(
        0.5,
        CueCommand::Emote {
            key: "smile".into(),
        },
    ));
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

// ============================================================================
// 絶対時刻ちょうどの到達（刻む時刻と判定の時刻を同じ足し算で作る）
//
// 絶対の発火時刻の定義は `CueSheet::absolute_fire_time` の「アンカー＋相対」の足し算だけ。
// 到達の判定が「現在時刻−アンカー」の引き算で相対へ戻すと、丸めでその時刻ちょうどに
// 届かないことがある（例: 100.0 + 2.5999999999999996 から 100.0 を引くと 2.5999999999999943）。
// ============================================================================

/// アンカーを刻んだときに、引き算で相対へ戻すと丸めで手前に落ちる相対時刻。
const ROUNDS_DOWN: f64 = 2.5999999999999996;

/// 前提: この値は引き算で相対へ戻すと元の値より手前に落ちる（検査が丸めを踏んでいる）。
#[test]
fn rounds_down_value_really_rounds_down_when_subtracted() {
    const { assert!((100.0 + ROUNDS_DOWN) - 100.0 < ROUNDS_DOWN) };
}

/// 中身の合図は「アンカー＋相対」の絶対時刻ちょうどの tick で配られる。
#[test]
fn payload_is_delivered_at_exact_absolute_fire_time() {
    let mut sched = TimedSchedule::<String>::new(100.0);
    sched.insert(Entry::Payload(ROUNDS_DOWN, "x".into()));

    sched.tick(100.0 + ROUNDS_DOWN);
    assert_eq!(sched.ready(), &["x".to_string()]);
}

/// 占有の終わりは `occupancy_horizon()` の絶対時刻ちょうどの tick で完了になる。
#[test]
fn completion_is_reached_at_exact_occupancy_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(100.0, ROUNDS_DOWN);
    sched.tick(sched.occupancy_horizon());
    assert!(sched.is_completed());
}

// ============================================================================
// まとめて入れる（insert_each）
// ============================================================================

/// 同じ時刻の群・区切り・時刻の逆行を混ぜた見本（記述順）。
fn mixed_entries() -> Vec<Entry<String>> {
    vec![
        Entry::Payload(0.0, "a0".into()),
        Entry::Payload(0.0, "a1".into()),
        Entry::Payload(0.5, "b0".into()),
        Entry::Barrier(0.5, BarrierKind::WaitForInput),
        Entry::Payload(0.5, "b1".into()),
        Entry::Payload(0.25, "late".into()),
        Entry::Barrier(1.0, BarrierKind::WaitForChoice { timeout: Some(0.5) }),
        Entry::Payload(1.0, "c0".into()),
        Entry::Payload(2.0, "d0".into()),
        Entry::Payload(2.0, "d1".into()),
    ]
}

/// まとめて入れた並びは 1 件ずつ `insert` した並びと同じ（同じ時刻は記述順に配る）。
/// 空のスケジュールへも、既に入っているスケジュールへも同じ（mcp-strict-errors 9.4・長い台本）。
#[test]
fn insert_each_matches_per_entry_insert_including_equal_time_fifo() {
    for prefilled in [vec![], mixed_entries()] {
        let mut one_by_one = TimedSchedule::<String>::new(0.0);
        let mut bulk = TimedSchedule::<String>::new(0.0);
        for entry in prefilled {
            one_by_one.insert(entry.clone());
            bulk.insert(entry);
        }
        for entry in mixed_entries() {
            one_by_one.insert(entry);
        }
        bulk.insert_each(mixed_entries());
        assert_eq!(format!("{bulk:?}"), format!("{one_by_one:?}"));
    }
}
