use super::*;

fn s(ticks: i64) -> TimeSpan {
    TimeSpan::try_from_ticks(ticks).unwrap()
}

#[test]
fn arithmetic_result_types() {
    let a: TimeSpan = s(2) + s(3);
    assert_eq!(a, s(5));
    let b: TimeDelta = s(2) - s(3);
    assert_eq!(b, TimeDelta::from_ticks(-1));
    assert_eq!(
        TimeSpan::ZERO - TimeSpan::MAX,
        TimeDelta::from_ticks(-i64::MAX)
    );
}

#[test]
fn add_saturates_and_checked_is_none() {
    assert_eq!(TimeSpan::MAX + s(1), TimeSpan::MAX);
    assert_eq!(s(1).checked_add(s(2)), Some(s(3)));
    assert_eq!(TimeSpan::MAX.checked_add(s(1)), None);
    assert_eq!(s(3).checked_sub(s(1)), Some(s(2)));
    assert_eq!(s(1).checked_sub(s(3)), None);
}

#[test]
fn try_from_ticks_rejects_negative() {
    assert_eq!(TimeSpan::try_from_ticks(0), Ok(TimeSpan::ZERO));
    assert_eq!(
        TimeSpan::try_from_ticks(333_333).unwrap().as_ticks(),
        333_333
    );
    assert_eq!(TimeSpan::try_from_ticks(i64::MAX), Ok(TimeSpan::MAX));
    assert_eq!(TimeSpan::try_from_ticks(-1), Err(TimeRangeError::Negative));
    assert_eq!(
        TimeSpan::try_from_ticks(i64::MIN),
        Err(TimeRangeError::Negative)
    );
}

#[test]
fn to_std_is_exact() {
    use std::time::Duration;
    assert_eq!(s(0).to_std(), Duration::ZERO);
    assert_eq!(s(12_345_678).to_std(), Duration::new(1, 234_567_800));
    assert_eq!(
        TimeSpan::MAX.to_std(),
        Duration::new(922_337_203_685, 477_580_700)
    );
}

#[test]
fn widens_into_delta_unchanged() {
    assert_eq!(TimeDelta::from(s(42)), TimeDelta::from_ticks(42));
    assert_eq!(TimeDelta::from(TimeSpan::MAX), TimeDelta::MAX);
}
