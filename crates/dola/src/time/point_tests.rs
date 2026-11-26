use super::*;

const fn p(ticks: i64) -> TimePoint {
    TimePoint::from_ticks(ticks)
}
const fn d(ticks: i64) -> TimeDelta {
    TimeDelta::from_ticks(ticks)
}
fn s(ticks: i64) -> TimeSpan {
    TimeSpan::try_from_ticks(ticks).unwrap()
}

#[test]
fn arithmetic_result_types() {
    let a: TimePoint = p(10) + s(5);
    assert_eq!(a, p(15));
    let b: TimePoint = p(10) - s(15);
    assert_eq!(b, p(-5));
    let c: TimePoint = p(10) + d(-3);
    assert_eq!(c, p(7));
    let e: TimePoint = p(10) - d(-3);
    assert_eq!(e, p(13));
    let f: TimeDelta = p(3) - p(10);
    assert_eq!(f, d(-7));
}

#[test]
fn operators_saturate_at_the_ends() {
    assert_eq!(TimePoint::MAX + s(1), TimePoint::MAX);
    assert_eq!(TimePoint::MIN - s(1), TimePoint::MIN);
    assert_eq!(TimePoint::MAX + d(1), TimePoint::MAX);
    assert_eq!(TimePoint::MIN + d(-1), TimePoint::MIN);
    assert_eq!(TimePoint::MIN - d(1), TimePoint::MIN);
    assert_eq!(TimePoint::MAX - d(-1), TimePoint::MAX);
    assert_eq!(TimePoint::MIN - TimePoint::MAX, TimeDelta::MIN);
    assert_eq!(TimePoint::MAX - TimePoint::MIN, TimeDelta::MAX);
}

#[test]
fn checked_add_is_none_past_the_ends() {
    assert_eq!(p(1).checked_add(s(2)), Some(p(3)));
    assert_eq!(TimePoint::MAX.checked_add(s(1)), None);
    assert_eq!(p(1).checked_add_delta(d(-2)), Some(p(-1)));
    assert_eq!(TimePoint::MAX.checked_add_delta(d(1)), None);
    assert_eq!(TimePoint::MIN.checked_add_delta(d(-1)), None);
}

#[test]
fn span_since_is_none_for_an_earlier_self() {
    assert_eq!(p(10).span_since(p(3)), Some(s(7)));
    assert_eq!(p(3).span_since(p(3)), Some(TimeSpan::ZERO));
    assert_eq!(p(3).span_since(p(4)), None);
    assert_eq!(
        TimePoint::MAX.span_since(TimePoint::MIN),
        Some(TimeSpan::MAX)
    );
}

#[test]
fn same_values_in_another_order_give_the_same_point() {
    assert_eq!(p(100) + s(7) - d(3), p(100) - d(3) + s(7));
}

#[test]
fn default_is_zero() {
    assert_eq!(TimePoint::default(), TimePoint::ZERO);
    assert_eq!(TimePoint::ZERO.as_ticks(), 0);
    assert_eq!(TimeSpan::default(), TimeSpan::ZERO);
    assert_eq!(TimeDelta::default(), TimeDelta::ZERO);
}
