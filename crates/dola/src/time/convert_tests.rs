use super::*;
use std::time::Duration;

fn ticks(span: Result<TimeSpan, TimeRangeError>) -> i64 {
    span.unwrap().as_ticks()
}

#[test]
fn from_millis_is_exact() {
    assert_eq!(TimeSpan::from_millis(0).as_ticks(), 0);
    assert_eq!(TimeSpan::from_millis(1).as_ticks(), 10_000);
    assert_eq!(
        TimeSpan::from_millis(u32::MAX).as_ticks(),
        u32::MAX as i64 * 10_000
    );
    assert_eq!(ticks(TimeSpan::try_from_millis(1_500)), 15_000_000);
    let edge = i64::MAX as u64 / 10_000;
    assert_eq!(ticks(TimeSpan::try_from_millis(edge)), edge as i64 * 10_000);
    assert_eq!(
        TimeSpan::try_from_millis(edge + 1),
        Err(TimeRangeError::OutOfRange)
    );
}

#[test]
fn secs_round_to_nearest_and_half_away_from_zero() {
    // 1.5e-7 秒は、10^7 倍した f64 の積がちょうど 1.5 になる。
    let half = 1.5e-7_f64;
    assert_eq!(ticks(TimeSpan::try_from_secs_f64(half.next_down())), 1);
    assert_eq!(ticks(TimeSpan::try_from_secs_f64(half)), 2);
    assert_eq!(ticks(TimeSpan::try_from_secs_f64(half.next_up())), 2);
    assert_eq!(ticks(TimeSpan::try_from_secs_f64(2.5e-7)), 3);
    assert_eq!(ticks(TimeSpan::try_from_secs_f64(0.05)), 500_000);
    assert_eq!(
        TimeDelta::try_from_secs_f64(-half.next_down()),
        Ok(TimeDelta::from_ticks(-1))
    );
    assert_eq!(
        TimeDelta::try_from_secs_f64(-half),
        Ok(TimeDelta::from_ticks(-2))
    );
    assert_eq!(
        TimeDelta::try_from_secs_f64(-half.next_up()),
        Ok(TimeDelta::from_ticks(-2))
    );
    assert_eq!(
        TimeDelta::try_from_secs_f64(-2.5e-7),
        Ok(TimeDelta::from_ticks(-3))
    );
}

#[test]
fn secs_reject_non_finite_negative_and_out_of_range() {
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            TimeSpan::try_from_secs_f64(v),
            Err(TimeRangeError::NotFinite)
        );
        assert_eq!(
            TimeDelta::try_from_secs_f64(v),
            Err(TimeRangeError::NotFinite)
        );
    }
    assert_eq!(
        TimeSpan::try_from_secs_f64(-1e-3),
        Err(TimeRangeError::Negative)
    );
    assert_eq!(ticks(TimeSpan::try_from_secs_f64(-0.0)), 0);
    assert_eq!(
        TimeSpan::try_from_secs_f64(1e12),
        Err(TimeRangeError::OutOfRange)
    );
    // 2^63 / 10^7 秒の積は 2^63 で、i64 に収まらない。負の側の -2^63 は収まる。
    let two_pow_63_ticks = 922_337_203_685.477_6;
    assert_eq!(
        TimeDelta::try_from_secs_f64(two_pow_63_ticks),
        Err(TimeRangeError::OutOfRange)
    );
    assert_eq!(
        TimeDelta::try_from_secs_f64(-two_pow_63_ticks),
        Ok(TimeDelta::MIN)
    );
    assert_eq!(
        TimeDelta::try_from_secs_f64(-1e12),
        Err(TimeRangeError::OutOfRange)
    );
}

#[test]
fn signed_millis_saturate_at_the_ends() {
    assert_eq!(TimeDelta::from_millis_saturating(-3).as_ticks(), -30_000);
    let edge = i64::MAX / 10_000;
    assert_eq!(
        TimeDelta::from_millis_saturating(edge).as_ticks(),
        edge * 10_000
    );
    assert_eq!(
        TimeDelta::from_millis_saturating(-edge).as_ticks(),
        -edge * 10_000
    );
    assert_eq!(TimeDelta::from_millis_saturating(edge + 1), TimeDelta::MAX);
    assert_eq!(TimeDelta::from_millis_saturating(-edge - 1), TimeDelta::MIN);
    assert_eq!(TimeDelta::from_millis_saturating(i64::MAX), TimeDelta::MAX);
    assert_eq!(TimeDelta::from_millis_saturating(i64::MIN), TimeDelta::MIN);
}

#[test]
fn duration_rounds_sub_100ns_to_nearest() {
    let span = |nanos: u64| {
        TimeSpan::try_from(Duration::from_nanos(nanos))
            .unwrap()
            .as_ticks()
    };
    assert_eq!(span(1_049), 10);
    assert_eq!(span(1_050), 11);
    assert_eq!(span(1_051), 11);
    assert_eq!(span(49), 0);
    assert_eq!(span(50), 1);
    assert_eq!(
        TimeSpan::try_from(Duration::from_millis(1_500)),
        Ok(TimeSpan::from_millis(1_500))
    );
}

#[test]
fn duration_fails_only_out_of_range() {
    assert_eq!(
        TimeSpan::try_from(TimeSpan::MAX.to_std()),
        Ok(TimeSpan::MAX)
    );
    assert_eq!(
        TimeSpan::try_from(TimeSpan::MAX.to_std() + Duration::from_nanos(49)),
        Ok(TimeSpan::MAX)
    );
    assert_eq!(
        TimeSpan::try_from(TimeSpan::MAX.to_std() + Duration::from_nanos(50)),
        Err(TimeRangeError::OutOfRange)
    );
    assert_eq!(
        TimeSpan::try_from(Duration::MAX),
        Err(TimeRangeError::OutOfRange)
    );
}
