//! TimedSchedule<T> の届き方（1 件ずつの取り出しを集めた結果）とバリア管理の検証。
//!
//! 手組みの `new`／`insert` はクレートの中だけの口なので、ここに置く。

use super::*;
use crate::cue::CueCommand;
use crate::time::testing::{at_ms, delta_ms, ms};
use crate::time::{TimeDelta, TimePoint};

/// `now` までに届いた中身を、届く物が無くなるか区切りに着くまで取り出して集める。
pub(super) fn take<T>(sched: &mut TimedSchedule<T>, now: TimePoint) -> Vec<T> {
    let mut taken = Vec::new();
    while let Some(Due::Payload { payload, .. }) = sched.pop_due(now) {
        taken.push(payload);
    }
    taken
}

// ============================================================================
// Payload 収集テスト
// ============================================================================

#[test]
fn tick_collects_payloads_at_reached_time() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(500), "a".into()));
    sched.insert(Entry::Payload(ms(1000), "b".into()));
    sched.insert(Entry::Payload(ms(2000), "c".into()));

    let ready = take(&mut sched, at_ms(500));
    assert_eq!(ready, &["a".to_string()]);

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["b".to_string()]);

    let ready = take(&mut sched, at_ms(2000));
    assert_eq!(ready, &["c".to_string()]);
}

#[test]
fn tick_collects_all_past_payloads_on_jump() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(100), "a".into()));
    sched.insert(Entry::Payload(ms(200), "b".into()));
    sched.insert(Entry::Payload(ms(300), "c".into()));

    let ready = take(&mut sched, at_ms(300));
    assert_eq!(ready.len(), 3);
}

#[test]
fn take_twice_at_same_time_delivers_once() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(1000), "x".into()));

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["x".to_string()]);

    let ready = take(&mut sched, at_ms(1000));
    assert!(ready.is_empty());
}

#[test]
fn tick_before_start_time_is_noop() {
    let mut sched = TimedSchedule::<String>::new(at_ms(10000));
    sched.insert(Entry::Payload(ms(0), "x".into()));

    let ready = take(&mut sched, at_ms(5000)); // 起点 10000ms より前
    assert!(ready.is_empty());
}

#[test]
fn tick_absolute_time_to_relative_offset() {
    let mut sched = TimedSchedule::<String>::new(at_ms(100000));
    sched.insert(Entry::Payload(ms(500), "a".into()));
    sched.insert(Entry::Payload(ms(1000), "b".into()));

    let ready = take(&mut sched, at_ms(100500)); // 隔たり 500ms
    assert_eq!(ready, &["a".to_string()]);

    let ready = take(&mut sched, at_ms(101000)); // 隔たり 1000ms
    assert_eq!(ready, &["b".to_string()]);
}

#[test]
fn remaining_and_is_completed() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(1000), "a".into()));
    sched.insert(Entry::Payload(ms(2000), "b".into()));

    assert_eq!(sched.remaining(), 2);
    assert!(!sched.is_completed_at(at_ms(0)));

    take(&mut sched, at_ms(1000));
    assert_eq!(sched.remaining(), 1);

    take(&mut sched, at_ms(2000));
    assert_eq!(sched.remaining(), 0);
    assert!(sched.is_completed_at(at_ms(2000)));
}

// ============================================================================
// 占有 horizon と is_completed（占有終了判定）
// is_completed_at は「entries 枯渇かつ barrier なしかつ now >= 起点 + horizon」。
// ============================================================================

/// horizon > 最終 entry offset のとき、entry を配り終えても開始からの経過が horizon に
/// 達するまで完了しない（占有終了判定）。
#[test]
fn with_horizon_defers_completion_until_elapsed_time_reaches_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(at_ms(0), ms(5000));
    sched.insert(Entry::Payload(ms(1000), "a".into()));

    take(&mut sched, at_ms(1000));
    assert_eq!(sched.remaining(), 0, "entry は配り終えた");
    assert!(
        !sched.is_completed_at(at_ms(1000)),
        "開始からの経過(1000ms) < horizon(5000ms) ゆえ未完了"
    );

    take(&mut sched, at_ms(4999));
    assert!(!sched.is_completed_at(at_ms(4999)), "horizon 直前は未完了");

    take(&mut sched, at_ms(5000));
    assert!(sched.is_completed_at(at_ms(5000)), "horizon 到達で完了");
}

/// horizon を超過していても barrier 中は完了しない（barrier 優先）。解除後に完了する。
#[test]
fn with_horizon_not_completed_while_barrier_active_even_past_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(at_ms(0), ms(1000));
    sched.insert(Entry::Barrier(ms(500), BarrierKind::WaitForInput));

    take(&mut sched, at_ms(2000)); // horizon(1000ms) 超過だが barrier で停止
    assert!(sched.current_barrier().is_some());
    assert!(
        !sched.is_completed_at(at_ms(2000)),
        "barrier 中は horizon 超過でも未完了"
    );

    sched.notify_barrier_resolved();
    assert!(
        sched.is_completed_at(at_ms(2000)),
        "解除後 entries 空・開始からの経過>=horizon で完了"
    );
}

// ============================================================================
// バリアテスト
// ============================================================================

#[test]
fn barrier_stops_progress() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(500), "before".into()));
    sched.insert(Entry::Barrier(ms(1000), BarrierKind::WaitForInput));
    sched.insert(Entry::Payload(ms(1500), "after".into()));

    let ready = take(&mut sched, at_ms(500));
    assert_eq!(ready, &["before".to_string()]);
    assert!(sched.current_barrier().is_none());

    take(&mut sched, at_ms(1000));
    assert!(sched.current_barrier().is_some());
    assert_eq!(sched.remaining(), 1); // "after" が残っている

    take(&mut sched, at_ms(2000));
    assert!(sched.current_barrier().is_some());
    assert_eq!(sched.remaining(), 1);
}

#[test]
fn barrier_resolved_resumes_progress() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Barrier(ms(500), BarrierKind::WaitForInput));
    sched.insert(Entry::Payload(ms(1000), "after_barrier".into()));

    take(&mut sched, at_ms(500));
    assert!(sched.current_barrier().is_some());

    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["after_barrier".to_string()]);
}

#[test]
fn notify_barrier_resolved_noop_when_no_barrier() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(1000), "x".into()));

    // バリアなしで解決しても、届き方は変わらない
    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());
    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["x".to_string()]);
    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());
    assert_eq!(sched.remaining(), 0);
}

// ============================================================================
// バリアのテスト
// ============================================================================

#[test]
fn wait_for_choice_barrier_resolved_with_choice_id() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Barrier(
        ms(500),
        BarrierKind::WaitForChoice { timeout: None },
    ));
    sched.insert(Entry::Payload(ms(1000), "after_choice".into()));

    take(&mut sched, at_ms(500));
    assert!(matches!(
        sched.current_barrier(),
        Some(BarrierKind::WaitForChoice { .. })
    ));

    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["after_choice".to_string()]);
}

/// 解決の直後に同じ時刻で取り出しても（本番の drive は解決の後すぐ同じ時刻で回す）、
/// 区切りの後ろだけが届く。
#[test]
fn resolve_then_same_time_tick_collects_after() {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(1000), "payload".into()));
    sched.insert(Entry::Barrier(
        ms(1000),
        BarrierKind::WaitForChoice { timeout: None },
    ));
    sched.insert(Entry::Payload(ms(1000), "after".into()));

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["payload".to_string()]);
    assert!(sched.current_barrier().is_some());

    sched.notify_barrier_resolved();
    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["after".to_string()]);
}

// ============================================================================
// 選択待ちの区切りの値は再生層が解かない
// ============================================================================

/// 値 `timeout` の選択待ちの区切りを 1000ms に置き、1000ms の tick で区切りに着いたとき
/// 止まって後続を配らないことを確かめる（値は上位層へ運ぶ指令で、区切りを飛ばす理由にしない）。
fn assert_choice_barrier_stops_on_arrival(timeout: Option<TimeDelta>) {
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Barrier(
        ms(1000),
        BarrierKind::WaitForChoice { timeout },
    ));
    sched.insert(Entry::Payload(ms(1000), "after".into()));

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(
        sched.current_barrier(),
        Some(&BarrierKind::WaitForChoice { timeout })
    );
    assert!(ready.is_empty());
}

#[test]
fn choice_barrier_with_zero_timeout_stops_on_arrival() {
    // 値 0（時間切れなしの指令）でも区切りを飛ばさない
    assert_choice_barrier_stops_on_arrival(Some(delta_ms(0)));
}

#[test]
fn choice_barrier_with_negative_timeout_stops_on_arrival() {
    // 負の値（時間切れなしの指令）でも区切りを飛ばさない
    assert_choice_barrier_stops_on_arrival(Some(delta_ms(-1)));
}

#[test]
fn choice_barrier_with_timeout_not_skipped_when_jumped_past() {
    // 区切り＋値（1000ms + 500ms = 1500ms）を一度に越えてから着いても止まる
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Barrier(
        ms(1000),
        BarrierKind::WaitForChoice {
            timeout: Some(delta_ms(500)),
        },
    ));
    sched.insert(Entry::Payload(ms(2000), "after".into()));

    let ready = take(&mut sched, at_ms(2000));
    assert_eq!(
        sched.current_barrier(),
        Some(&BarrierKind::WaitForChoice {
            timeout: Some(delta_ms(500))
        })
    );
    assert!(ready.is_empty());
}

#[test]
fn choice_barrier_with_timeout_never_self_releases_only_external_resolve() {
    // 止まった後に時計を大きく進めても自分では解かない。
    // 解けるのは外からの解決（notify_barrier_resolved）だけ
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Barrier(
        ms(1000),
        BarrierKind::WaitForChoice {
            timeout: Some(delta_ms(500)),
        },
    ));
    sched.insert(Entry::Payload(ms(2000), "after".into()));

    take(&mut sched, at_ms(1000));
    assert!(sched.current_barrier().is_some());

    let ready = take(&mut sched, at_ms(10000));
    assert_eq!(
        sched.current_barrier(),
        Some(&BarrierKind::WaitForChoice {
            timeout: Some(delta_ms(500))
        })
    );
    assert!(ready.is_empty());

    sched.notify_barrier_resolved();
    assert!(sched.current_barrier().is_none());

    let ready = take(&mut sched, at_ms(10500));
    assert_eq!(ready, &["after".to_string()]);
}

// ============================================================================
// 同時刻エントリの順序
// ============================================================================

#[test]
fn insert_same_offset_payloads_delivered_fifo() {
    // insert() は同一オフセットで挿入順（FIFO）配信となる
    let mut sched = TimedSchedule::<String>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(1000), "a".into()));
    sched.insert(Entry::Payload(ms(1000), "b".into()));
    sched.insert(Entry::Payload(ms(1000), "c".into()));

    let ready = take(&mut sched, at_ms(1000));
    assert_eq!(ready, &["a".to_string(), "b".to_string(), "c".to_string()]);
}

// ============================================================================
// Entry アクセサ
// ============================================================================

#[test]
fn entry_offset_accessor_for_all_variants() {
    assert_eq!(Entry::Payload(ms(1500), "x".to_string()).offset(), ms(1500));
    assert_eq!(
        Entry::<String>::Barrier(ms(2500), BarrierKind::WaitForInput).offset(),
        ms(2500)
    );
}

// ============================================================================
// CueCommand ペイロードでの統合検証
// ============================================================================

#[test]
fn timed_schedule_with_cue_command() {
    let mut sched = TimedSchedule::<CueCommand>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(0), CueCommand::Text("hello".into())));
    sched.insert(Entry::Payload(
        ms(500),
        CueCommand::Emote {
            key: "smile".into(),
        },
    ));
    sched.insert(Entry::Payload(ms(1000), CueCommand::Clear));

    let ready = take(&mut sched, at_ms(0));
    assert_eq!(ready.len(), 1);
    assert!(matches!(ready[0], CueCommand::Text(_)));

    let ready = take(&mut sched, at_ms(500));
    assert_eq!(ready.len(), 1);
    assert!(matches!(ready[0], CueCommand::Emote { .. }));

    let ready = take(&mut sched, at_ms(1000));
    assert!(matches!(ready[0], CueCommand::Clear));
}

// ============================================================================
// 占有 horizon の絶対時刻照会
// ============================================================================

/// `occupancy_horizon()` はアンカー（`start_time`）＋相対 horizon の**和**を返す。
/// タイムアウト計測の起点はこの duration 権威から取得され、独自の時間基準を作らない。
#[test]
fn occupancy_horizon_returns_anchor_plus_relative_horizon() {
    let sched = TimedSchedule::<String>::with_horizon(at_ms(10000), ms(2500));
    assert_eq!(
        sched.occupancy_horizon(),
        at_ms(12_500),
        "アンカー 10000ms ＋ 相対 horizon 2500ms = 12500ms"
    );
}

/// horizon 既定（`new` = 0ms）ではアンカーそのものを返す（手組みスケジュールの後方互換）。
#[test]
fn occupancy_horizon_of_default_schedule_is_the_anchor() {
    let sched = TimedSchedule::<String>::new(at_ms(7250));
    assert_eq!(
        sched.occupancy_horizon(),
        at_ms(7250),
        "horizon=0ms 既定ゆえアンカーそのもの"
    );
}

/// アンカー未刻印（0ms）では相対 horizon がそのまま絶対値になる。
#[test]
fn occupancy_horizon_without_anchor_is_the_relative_horizon() {
    let sched = TimedSchedule::<String>::with_horizon(at_ms(0), ms(3000));
    assert_eq!(sched.occupancy_horizon(), at_ms(3000));
}

/// `occupancy_horizon()` は tick で進行しても不変（配送位置でなく占有区間の終端を指す）。
#[test]
fn occupancy_horizon_is_invariant_across_ticks() {
    let mut sched = TimedSchedule::<String>::with_horizon(at_ms(100000), ms(2000));
    sched.insert(Entry::Payload(ms(500), "a".into()));
    assert_eq!(sched.occupancy_horizon(), at_ms(102_000));

    let ready = take(&mut sched, at_ms(100500));
    assert_eq!(ready, &["a".to_string()]);
    assert_eq!(
        sched.occupancy_horizon(),
        at_ms(102_000),
        "配送が進んでも占有 horizon の絶対時刻は動かない"
    );
}

/// `occupancy_horizon()` は `is_completed_at()` の閾値と一致する（同一の horizon 権威）。
#[test]
fn occupancy_horizon_matches_is_completed_threshold() {
    let mut sched = TimedSchedule::<String>::with_horizon(at_ms(100000), ms(2000));
    sched.insert(Entry::Payload(ms(500), "a".into()));

    take(&mut sched, at_ms(100500));
    assert!(
        !sched.is_completed_at(at_ms(100500)),
        "horizon 未到達では完了しない"
    );

    let horizon = sched.occupancy_horizon();
    take(&mut sched, horizon);
    assert!(
        sched.is_completed_at(horizon),
        "occupancy_horizon() が返す絶対時刻の到達で完了する"
    );
}

// ============================================================================
// 絶対時刻ちょうどの到達（起点＋隔たりの整数の和で判定する）
// ============================================================================

/// 中身の合図は「アンカー＋相対」の絶対時刻ちょうどの tick で配られる。
#[test]
fn payload_is_delivered_at_exact_absolute_fire_time_and_not_one_tick_before() {
    let mut sched = TimedSchedule::<String>::new(at_ms(100000));
    sched.insert(Entry::Payload(ms(2600), "x".into()));

    let fire = at_ms(102_600);
    let ready = take(&mut sched, TimePoint::from_ticks(fire.as_ticks() - 1));
    assert!(ready.is_empty());

    let ready = take(&mut sched, fire);
    assert_eq!(ready, &["x".to_string()]);
}

/// 占有の終わりは `occupancy_horizon()` の絶対時刻ちょうどの tick で完了になる。
#[test]
fn completion_is_reached_at_exact_occupancy_horizon() {
    let mut sched = TimedSchedule::<String>::with_horizon(at_ms(100_000), ms(2600));
    let horizon = sched.occupancy_horizon();
    take(&mut sched, TimePoint::from_ticks(horizon.as_ticks() - 1));
    assert!(!sched.is_completed_at(TimePoint::from_ticks(horizon.as_ticks() - 1)));

    take(&mut sched, horizon);
    assert!(sched.is_completed_at(horizon));
}

// ============================================================================
// まとめて入れる（insert_each）
// ============================================================================

/// 同じ時刻の群・区切り・時刻の逆行を混ぜた見本（記述順）。
fn mixed_entries() -> Vec<Entry<String>> {
    vec![
        Entry::Payload(ms(0), "a0".into()),
        Entry::Payload(ms(0), "a1".into()),
        Entry::Payload(ms(500), "b0".into()),
        Entry::Barrier(ms(500), BarrierKind::WaitForInput),
        Entry::Payload(ms(500), "b1".into()),
        Entry::Payload(ms(250), "late".into()),
        Entry::Barrier(
            ms(1000),
            BarrierKind::WaitForChoice {
                timeout: Some(delta_ms(500)),
            },
        ),
        Entry::Payload(ms(1000), "c0".into()),
        Entry::Payload(ms(2000), "d0".into()),
        Entry::Payload(ms(2000), "d1".into()),
    ]
}

/// まとめて入れた並びは 1 件ずつ `insert` した並びと同じ（同じ時刻は記述順に配る）。
/// 空のスケジュールへも、既に入っているスケジュールへも同じ（長い台本）。
#[test]
fn insert_each_matches_per_entry_insert_including_equal_time_fifo() {
    for prefilled in [vec![], mixed_entries()] {
        let mut one_by_one = TimedSchedule::<String>::new(at_ms(0));
        let mut bulk = TimedSchedule::<String>::new(at_ms(0));
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
