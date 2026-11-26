//! TimedSchedule<T> の 1 件ずつの取り出しと、起点の置き直しの検証。

use super::tests::take;
use super::*;
use crate::time::testing::{at_ms, delta_ms, ms};
use crate::time::{TimeDelta, TimePoint, TimeSpan};

fn labels(schedule: &mut TimedSchedule<&'static str>, now: TimePoint) -> Vec<&'static str> {
    take(schedule, now)
}

fn three_at_origin(origin: TimePoint) -> TimedSchedule<&'static str> {
    let mut sched = TimedSchedule::new(origin);
    sched.insert(Entry::Payload(ms(100), "a"));
    sched.insert(Entry::Payload(ms(200), "b"));
    sched.insert(Entry::Payload(ms(300), "c"));
    sched
}

// ============================================================================
// 1 件ずつの取り出し
// ============================================================================

#[test]
fn pop_due_returns_one_item_with_fire_time_and_script_time() {
    let mut sched = three_at_origin(at_ms(1000));

    assert_eq!(
        sched.pop_due(at_ms(1300)),
        Some(Due::Payload {
            fire_time: at_ms(1100),
            at: ms(100),
            payload: "a"
        })
    );
    assert_eq!(sched.remaining(), 2, "1 回の取り出しは 1 件だけ");
}

#[test]
fn pop_due_returns_none_when_nothing_is_due() {
    let mut sched = three_at_origin(at_ms(1000));
    assert_eq!(sched.pop_due(at_ms(1099)), None);
    assert_eq!(sched.remaining(), 3);
}

#[test]
fn pop_due_returns_the_barrier_then_none_until_resolved() {
    let mut sched = TimedSchedule::<&str>::new(at_ms(0));
    sched.insert(Entry::Barrier(ms(100), BarrierKind::WaitForInput));
    sched.insert(Entry::Payload(ms(100), "after"));

    assert_eq!(
        sched.pop_due(at_ms(100)),
        Some(Due::Barrier(BarrierKind::WaitForInput))
    );
    assert_eq!(sched.pop_due(at_ms(5000)), None, "区切りの間は何も返らない");

    sched.notify_barrier_resolved();
    assert_eq!(labels(&mut sched, at_ms(100)), ["after"]);
}

/// 遅れた刻みでも、発火時刻は刻みの時刻でなく「起点＋相対」。
#[test]
fn late_tick_keeps_fire_time_at_origin_plus_relative() {
    let mut sched = three_at_origin(at_ms(1000));
    let mut fire_times = Vec::new();
    while let Some(Due::Payload { fire_time, .. }) = sched.pop_due(at_ms(9000)) {
        fire_times.push(fire_time);
    }
    assert_eq!(fire_times, [at_ms(1100), at_ms(1200), at_ms(1300)]);
}

/// 同じ時刻の 2 度目は、置き直しが無ければ何も届かない。
#[test]
fn second_take_at_same_time_without_reorigin_delivers_nothing() {
    let mut sched = three_at_origin(at_ms(0));
    assert_eq!(labels(&mut sched, at_ms(200)), ["a", "b"]);
    assert!(labels(&mut sched, at_ms(200)).is_empty());
    assert_eq!(sched.remaining(), 1);
}

// ============================================================================
// 起点の置き直し
// ============================================================================

/// 起点を後ろへ置くと、まだ取り出していない項目は新しい起点＋相対まで届かない。
#[test]
fn shift_later_delays_pending_items_to_new_origin() {
    let mut sched = three_at_origin(at_ms(0));
    assert_eq!(labels(&mut sched, at_ms(100)), ["a"]);

    sched.shift_origin(at_ms(100), delta_ms(1000));
    assert_eq!(sched.origin(), at_ms(1000));

    assert!(
        labels(&mut sched, at_ms(300)).is_empty(),
        "前の起点では届かない"
    );
    assert!(labels(&mut sched, at_ms(1199)).is_empty());
    assert_eq!(labels(&mut sched, at_ms(1200)), ["b"]);
    assert_eq!(labels(&mut sched, at_ms(1300)), ["c"]);
}

/// 同じ時刻の 2 度目でも、起点が前へ動いていれば、新しく届いた分だけが届く。
#[test]
fn second_take_at_same_time_after_shift_earlier_delivers_only_new_items() {
    let mut sched = three_at_origin(at_ms(1000));
    assert_eq!(labels(&mut sched, at_ms(1100)), ["a"]);

    sched.shift_origin(at_ms(1100), delta_ms(-100));
    assert_eq!(labels(&mut sched, at_ms(1100)), ["b"]);
    assert!(labels(&mut sched, at_ms(1100)).is_empty());
}

/// 起点は、最初の刻みより前へも置ける。
#[test]
fn origin_can_move_before_the_first_tick() {
    let mut sched = three_at_origin(at_ms(1000));
    assert!(labels(&mut sched, at_ms(1000)).is_empty());

    sched.shift_origin(at_ms(1000), delta_ms(-5000));
    assert_eq!(sched.origin(), at_ms(-4000));
    assert_eq!(labels(&mut sched, at_ms(1000)), ["a", "b", "c"]);
}

/// 「時刻 at が台本の時刻 script_time である」ように起点を置く。
#[test]
fn reanchor_places_origin_at_at_minus_script_time() {
    let mut sched = three_at_origin(at_ms(0));

    let notice = sched.reanchor(at_ms(50), at_ms(5000), ms(200));
    assert_eq!(sched.origin(), at_ms(4800));
    assert_eq!(
        notice,
        Some(OriginNotice {
            effective_at: at_ms(50),
            origin: at_ms(4800),
            previous: Some(at_ms(0)),
        })
    );

    assert_eq!(labels(&mut sched, at_ms(4999)), ["a"]);
    assert_eq!(labels(&mut sched, at_ms(5000)), ["b"]);
}

/// 取り出しの間で置き直すと、前の項目は前の起点で、後ろの項目は新しい起点で発火時刻が決まる。
#[test]
fn reorigin_between_pops_decides_fire_time_per_pop() {
    let mut sched = three_at_origin(at_ms(1000));
    let now = at_ms(1300);

    let first = sched.pop_due(now);
    sched.shift_origin(now, delta_ms(-50));
    let second = sched.pop_due(now);
    sched.shift_origin(now, delta_ms(20));
    let third = sched.pop_due(now);

    assert_eq!(
        [first, second, third],
        [
            Some(Due::Payload {
                fire_time: at_ms(1100),
                at: ms(100),
                payload: "a"
            }),
            Some(Due::Payload {
                fire_time: at_ms(1150),
                at: ms(200),
                payload: "b"
            }),
            Some(Due::Payload {
                fire_time: at_ms(1270),
                at: ms(300),
                payload: "c"
            }),
        ]
    );
}

/// 置き直した後は、同じ刻みの中で、新しい起点で届く後ろの項目が続けて取り出せる。
#[test]
fn reorigin_within_a_tick_lets_later_items_follow() {
    let mut sched = three_at_origin(at_ms(0));
    let now = at_ms(100);

    assert!(matches!(
        sched.pop_due(now),
        Some(Due::Payload { payload: "a", .. })
    ));
    assert_eq!(sched.pop_due(now), None);

    sched.reanchor(now, now, ms(300));
    assert_eq!(labels(&mut sched, now), ["b", "c"]);
}

/// 区切りで止まっている間も起点は動き、解けた後の項目は新しい起点で届く。
#[test]
fn origin_moves_during_a_barrier() {
    let mut sched = TimedSchedule::<&str>::new(at_ms(0));
    sched.insert(Entry::Barrier(ms(100), BarrierKind::WaitForInput));
    sched.insert(Entry::Payload(ms(200), "after"));

    assert!(matches!(sched.pop_due(at_ms(100)), Some(Due::Barrier(_))));

    let notice = sched.shift_origin(at_ms(3000), delta_ms(2900));
    assert_eq!(
        notice,
        Some(OriginNotice {
            effective_at: at_ms(3000),
            origin: at_ms(2900),
            previous: Some(at_ms(0)),
        })
    );
    assert!(
        sched.current_barrier().is_some(),
        "置き直しは区切りを解かない"
    );

    sched.notify_barrier_resolved();
    assert!(labels(&mut sched, at_ms(3099)).is_empty());
    let due = sched.pop_due(at_ms(3100));
    assert_eq!(
        due,
        Some(Due::Payload {
            fire_time: at_ms(3100),
            at: ms(200),
            payload: "after"
        })
    );
}

/// 何度置き直しても、落とさず、2 度届けず、順を保つ。
#[test]
fn repeated_reorigin_never_drops_duplicates_or_reorders() {
    let mut sched = TimedSchedule::<u32>::new(at_ms(0));
    sched.insert_each((0..40).map(|i| Entry::Payload(ms(i * 10), i)));

    let shifts = [-70, 500, -1, 0, -900, 333, -333, 10_000, -20_000, 7];
    let mut delivered = Vec::new();
    for step in 0..200i64 {
        let now = at_ms(step * 5);
        sched.shift_origin(now, delta_ms(shifts[step as usize % shifts.len()]));
        delivered.extend(take(&mut sched, now));
        sched.reanchor(now, now, ms((step as u32 * 3) % 400));
        delivered.extend(take(&mut sched, now));
    }
    sched.reanchor(at_ms(1000), at_ms(1000), ms(390));
    delivered.extend(take(&mut sched, at_ms(1000)));

    assert_eq!(delivered, (0..40).collect::<Vec<u32>>());
    assert_eq!(sched.remaining(), 0);
}

/// 起点が型の下の端より前になっても、端が無い時と同じ項目が、同じ刻みで届く。
#[test]
fn origin_below_the_type_minimum_delivers_as_if_there_were_no_bound() {
    let floor = TimePoint::MIN.as_ticks();
    let span = |ticks| TimeSpan::try_from_ticks(ticks).unwrap();
    let mut sched = TimedSchedule::<&str>::new(TimePoint::from_ticks(floor + 10));
    sched.insert(Entry::Payload(span(2000), "a"));
    sched.insert(Entry::Payload(span(3000), "b"));

    let now = TimePoint::from_ticks(floor + 10);
    let notice = sched.shift_origin(now, TimeDelta::from_ticks(-1000));
    assert_eq!(
        notice,
        Some(OriginNotice {
            effective_at: now,
            origin: TimePoint::MIN,
            previous: Some(TimePoint::from_ticks(floor + 10)),
        }),
        "返す起点だけが端へ張り付く"
    );
    assert_eq!(sched.origin(), TimePoint::MIN);

    assert!(labels(&mut sched, TimePoint::from_ticks(floor + 1009)).is_empty());
    assert_eq!(
        sched.pop_due(TimePoint::from_ticks(floor + 1010)),
        Some(Due::Payload {
            fire_time: TimePoint::from_ticks(floor + 1010),
            at: span(2000),
            payload: "a"
        })
    );

    sched.shift_origin(
        TimePoint::from_ticks(floor + 1010),
        TimeDelta::from_ticks(1000),
    );
    assert_eq!(
        sched.origin(),
        TimePoint::from_ticks(floor + 10),
        "戻すと、元の起点に戻る（端で張り付いた値から数え直さない）"
    );
    assert!(labels(&mut sched, TimePoint::from_ticks(floor + 3009)).is_empty());
    assert_eq!(
        labels(&mut sched, TimePoint::from_ticks(floor + 3010)),
        ["b"]
    );
}

/// 型の下の端そのものの刻みでも、端より前の起点の項目が届く（比べは 128 ビット）。
#[test]
fn reanchor_below_the_type_minimum_keeps_comparison_exact() {
    let mut sched = TimedSchedule::<&str>::new(at_ms(0));
    sched.insert(Entry::Payload(ms(0), "a"));
    sched.insert(Entry::Payload(TimeSpan::MAX, "far"));

    sched.reanchor(TimePoint::MIN, TimePoint::MIN, TimeSpan::MAX);
    assert_eq!(sched.origin(), TimePoint::MIN);
    assert_eq!(
        labels(&mut sched, TimePoint::MIN),
        ["a", "far"],
        "起点は MIN − MAX。「far」の発火時刻が MIN ちょうど"
    );
}

// ============================================================================
// 置き直しの知らせ
// ============================================================================

#[test]
fn shift_origin_returns_notice_with_effective_time_new_and_previous_origin() {
    let mut sched = three_at_origin(at_ms(1000));

    let notice = sched.shift_origin(at_ms(1234), delta_ms(-300)).unwrap();
    assert_eq!(
        notice.effective_at,
        at_ms(1234),
        "効く時刻は渡した現在の時刻"
    );
    assert_eq!(notice.origin, at_ms(700));
    assert_eq!(notice.previous, Some(at_ms(1000)));
    assert_eq!(notice.delta(), Some(delta_ms(-300)));
}

#[test]
fn unchanged_reorigin_returns_none_and_changes_nothing() {
    let mut sched = three_at_origin(at_ms(1000));

    assert_eq!(sched.shift_origin(at_ms(1050), TimeDelta::ZERO), None);
    assert_eq!(sched.reanchor(at_ms(1050), at_ms(1200), ms(200)), None);
    assert_eq!(sched.origin(), at_ms(1000));
    assert_eq!(labels(&mut sched, at_ms(1100)), ["a"]);
}

#[test]
fn first_notice_carries_origin_as_effective_time_and_no_previous() {
    let notice = OriginNotice::first(at_ms(420));
    assert_eq!(
        notice,
        OriginNotice {
            effective_at: at_ms(420),
            origin: at_ms(420),
            previous: None,
        }
    );
    assert_eq!(notice.delta(), None);
}

// ============================================================================
// 完了と占有の終わりは、その時の起点で答える
// ============================================================================

#[test]
fn occupancy_horizon_moves_with_origin() {
    let mut sched = TimedSchedule::<&str>::with_horizon(at_ms(1000), ms(500));
    assert_eq!(sched.occupancy_horizon(), at_ms(1500));

    sched.shift_origin(at_ms(1000), delta_ms(250));
    assert_eq!(sched.occupancy_horizon(), at_ms(1750));

    sched.reanchor(at_ms(1000), at_ms(100), ms(100));
    assert_eq!(sched.occupancy_horizon(), at_ms(500));
}

#[test]
fn completion_is_answered_from_the_current_origin() {
    let mut sched = TimedSchedule::<&str>::with_horizon(at_ms(0), ms(500));
    sched.insert(Entry::Payload(ms(0), "a"));
    assert_eq!(labels(&mut sched, at_ms(500)), ["a"]);
    assert!(sched.is_completed_at(at_ms(500)));

    sched.shift_origin(at_ms(500), delta_ms(100));
    assert!(
        !sched.is_completed_at(at_ms(500)),
        "起点が後ろへ動けば、同じ時刻でも占有の終わりに届いていない"
    );
    assert!(sched.is_completed_at(at_ms(600)));

    sched.shift_origin(at_ms(500), delta_ms(-600));
    assert!(sched.is_completed_at(at_ms(0)));
}

// ============================================================================
// まだ取り出していない項目の照会
// ============================================================================

#[test]
fn pending_lists_undelivered_payloads_in_order_with_planned_fire_time() {
    let mut sched = three_at_origin(at_ms(1000));
    sched.insert(Entry::Barrier(ms(150), BarrierKind::WaitForInput));
    assert_eq!(labels(&mut sched, at_ms(1100)), ["a"]);

    sched.shift_origin(at_ms(1100), delta_ms(40));
    let pending: Vec<_> = sched
        .pending_payloads()
        .map(|(fire_time, at, payload)| (fire_time, at, *payload))
        .collect();
    assert_eq!(
        pending,
        [(at_ms(1240), ms(200), "b"), (at_ms(1340), ms(300), "c")]
    );
}
