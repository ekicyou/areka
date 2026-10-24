//! ホイールの溜めと間引きの純関数の檻（mouse-click-wheel-events 要件 6.2〜6.6・6.8・6.9・11.2・design「WheelThrottle」）。

use super::{PendingWheel, WheelAccum, WheelPlan, WheelRotation, plan_wheel};

fn rot(delta: i32, surface: (i64, i64), region: Option<&str>) -> Option<WheelRotation> {
    Some(WheelRotation {
        delta,
        surface,
        region: region.map(str::to_owned),
    })
}

fn quiet() -> WheelAccum {
    WheelAccum {
        scope: 1,
        last_sent_ms: None,
        pending: None,
    }
}

fn sent(sum: i32, surface: (i64, i64), region: Option<&str>) -> WheelPlan {
    WheelPlan::Send(PendingWheel {
        sum,
        surface,
        region: region.map(str::to_owned),
    })
}

#[test]
fn first_rotation_from_quiet_is_sent_at_once() {
    let mut acc = quiet();
    assert_eq!(
        plan_wheel(&mut acc, 1_000, rot(120, (3, 4), Some("Head"))),
        sent(120, (3, 4), Some("Head"))
    );
    assert_eq!(acc.last_sent_ms, Some(1_000));
    assert_eq!(acc.pending, None);
}

#[test]
fn nothing_pending_and_no_rotation_is_idle() {
    let mut acc = quiet();
    assert_eq!(plan_wheel(&mut acc, 1_000, None), WheelPlan::Idle);
    assert_eq!(acc.last_sent_ms, None);
}

#[test]
fn rotations_inside_the_interval_add_up_and_return_the_exact_deadline() {
    let mut acc = quiet();
    plan_wheel(&mut acc, 1_000, rot(120, (0, 0), None));
    assert_eq!(
        plan_wheel(&mut acc, 1_030, rot(120, (1, 1), None)),
        WheelPlan::Wait { until_ms: 1_100 }
    );
    assert_eq!(
        plan_wheel(&mut acc, 1_060, rot(240, (2, 2), None)),
        WheelPlan::Wait { until_ms: 1_100 }
    );
    assert_eq!(acc.pending.as_ref().map(|p| p.sum), Some(360));
}

#[test]
fn flushes_at_the_deadline_exactly_but_not_at_99_ms() {
    let mut acc = quiet();
    plan_wheel(&mut acc, 1_000, rot(120, (0, 0), None));
    plan_wheel(&mut acc, 1_010, rot(-120, (0, 0), None));
    plan_wheel(&mut acc, 1_020, rot(-120, (5, 6), Some("Bust")));
    assert_eq!(
        plan_wheel(&mut acc, 1_099, None),
        WheelPlan::Wait { until_ms: 1_100 }
    );
    assert_eq!(
        plan_wheel(&mut acc, 1_100, None),
        sent(-240, (5, 6), Some("Bust"))
    );
    assert_eq!(acc.last_sent_ms, Some(1_100));
    assert_eq!(plan_wheel(&mut acc, 1_100, None), WheelPlan::Idle);
}

#[test]
fn zero_sum_is_dropped_and_keeps_the_last_sent_time() {
    let mut acc = quiet();
    plan_wheel(&mut acc, 1_000, rot(120, (0, 0), None));
    plan_wheel(&mut acc, 1_010, rot(120, (0, 0), None));
    plan_wheel(&mut acc, 1_020, rot(-120, (0, 0), None));
    assert_eq!(plan_wheel(&mut acc, 1_100, None), WheelPlan::DropZero);
    assert_eq!(acc.pending, None);
    assert_eq!(acc.last_sent_ms, Some(1_000));
}

#[test]
fn the_last_rotation_sets_position_and_region() {
    let mut acc = quiet();
    plan_wheel(&mut acc, 1_000, rot(120, (0, 0), None));
    plan_wheel(&mut acc, 1_010, rot(120, (1, 2), Some("Head")));
    plan_wheel(&mut acc, 1_020, rot(120, (7, 8), None));
    assert_eq!(plan_wheel(&mut acc, 1_100, None), sent(240, (7, 8), None));
}

#[test]
fn a_rotation_after_the_interval_is_sent_at_once() {
    let mut acc = quiet();
    plan_wheel(&mut acc, 1_000, rot(120, (0, 0), None));
    assert_eq!(
        plan_wheel(&mut acc, 1_250, rot(-120, (9, 9), None)),
        sent(-120, (9, 9), None)
    );
    assert_eq!(acc.last_sent_ms, Some(1_250));
}

#[test]
fn the_sum_saturates_instead_of_wrapping() {
    let mut acc = quiet();
    plan_wheel(&mut acc, 1_000, rot(120, (0, 0), None));
    plan_wheel(&mut acc, 1_010, rot(i32::MAX, (0, 0), None));
    plan_wheel(&mut acc, 1_020, rot(i32::MAX, (0, 0), None));
    assert_eq!(acc.pending.as_ref().map(|p| p.sum), Some(i32::MAX));
}
