use super::*;

const fn d(ticks: i64) -> TimeDelta {
    TimeDelta::from_ticks(ticks)
}

#[test]
fn arithmetic_result_types_and_saturation() {
    let a: TimeDelta = d(2) + d(-5);
    assert_eq!(a, d(-3));
    let b: TimeDelta = d(2) - d(5);
    assert_eq!(b, d(-3));
    let c: TimeDelta = -d(4);
    assert_eq!(c, d(-4));
    assert_eq!(TimeDelta::MAX + d(1), TimeDelta::MAX);
    assert_eq!(TimeDelta::MIN - d(1), TimeDelta::MIN);
    assert_eq!(-TimeDelta::MIN, TimeDelta::MAX);
}

#[test]
fn checked_add_is_none_past_the_ends() {
    assert_eq!(d(1).checked_add(d(-3)), Some(d(-2)));
    assert_eq!(TimeDelta::MAX.checked_add(d(1)), None);
    assert_eq!(TimeDelta::MIN.checked_add(d(-1)), None);
}

#[test]
fn to_span_is_none_for_negative() {
    assert!(d(-1).is_negative());
    assert!(!d(0).is_negative());
    assert_eq!(d(0).to_span(), Some(TimeSpan::ZERO));
    assert_eq!(d(7).to_span(), TimeSpan::try_from_ticks(7).ok());
    assert_eq!(d(-1).to_span(), None);
}
