use super::*;

const fn d(ticks: i64) -> TimeDelta {
    TimeDelta::from_ticks(ticks)
}
fn s(ticks: i64) -> TimeSpan {
    TimeSpan::try_from_ticks(ticks).unwrap()
}
fn k(value: f64) -> TimeScale {
    TimeScale::new(value).unwrap()
}
fn g(scale: f64, script: i64) -> i64 {
    k(scale).first_reaching(d(script)).unwrap().as_ticks()
}

const SCALES: [f64; 9] = [1.0, 0.5, 0.25, 0.1, 0.3, 1.0 / 3.0, 1e-6, 2.0, 3.0];
const MORE_SCALES: [f64; 1] = [1e6];

fn all_scales() -> impl Iterator<Item = TimeScale> {
    SCALES.iter().chain(MORE_SCALES.iter()).map(|&v| k(v))
}

#[test]
fn new_accepts_only_finite_positive() {
    assert_eq!(TimeScale::new(1.0), Some(TimeScale::ONE));
    assert_eq!(TimeScale::new(0.5).map(TimeScale::get), Some(0.5));
    for bad in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(TimeScale::new(bad), None, "{bad}");
    }
}

#[test]
fn scale_one_is_identity_at_every_value() {
    for e in [i64::MIN, -1, 0, 1, (1 << 53) + 1, i64::MAX - 1, i64::MAX] {
        assert_eq!(TimeScale::ONE.forward(d(e)), d(e));
        assert_eq!(TimeScale::ONE.first_reaching(d(e)), Some(d(e)));
    }
}

#[test]
fn forward_floors_toward_negative_infinity_and_saturates() {
    assert_eq!(k(3.0).forward(d(333_333)), d(999_999));
    assert_eq!(k(0.5).forward(d(3)), d(1));
    assert_eq!(k(0.5).forward(d(-3)), d(-2));
    assert_eq!(k(2.0).forward(d(i64::MAX)), d(i64::MAX));
    assert_eq!(k(2.0).forward(d(i64::MIN)), d(i64::MIN));
    assert_eq!(k(f64::MAX).forward(d(1)), d(i64::MAX));
}

#[test]
fn decision_and_schedule_never_disagree() {
    let scripts = [
        i64::MIN,
        -1_000_001,
        -1,
        0,
        1,
        3,
        999_999,
        1_000_000,
        1_000_001,
        (1 << 53) + 1,
        i64::MAX / 3,
        i64::MAX - 1,
        i64::MAX,
    ];
    for scale in all_scales() {
        for &script in &scripts {
            let script = d(script);
            let Some(first) = scale.first_reaching(script) else {
                assert!(scale.forward(TimeDelta::MAX) < script);
                continue;
            };
            for e in [
                first.checked_add(d(-1)),
                Some(first),
                first.checked_add(d(1)),
            ]
            .into_iter()
            .flatten()
            {
                assert_eq!(
                    e >= first,
                    scale.forward(e) >= script,
                    "scale {} script {script:?} e {e:?}",
                    scale.get()
                );
            }
        }
    }
}

#[test]
fn forward_never_decreases() {
    let points = [
        i64::MIN,
        i64::MIN + 1,
        -1_000_000,
        -2,
        -1,
        0,
        1,
        2,
        999_999,
        1_000_000,
        1 << 53,
        (1 << 53) + 1,
        i64::MAX - 1,
        i64::MAX,
    ];
    for scale in all_scales() {
        for pair in points.windows(2) {
            assert!(scale.forward(d(pair[0])) <= scale.forward(d(pair[1])));
        }
    }
}

#[test]
fn unreachable_script_time_is_none() {
    assert_eq!(k(0.5).first_reaching(TimeDelta::MAX), None);
    assert_eq!(k(1e-6).first_reaching(d(i64::MAX / 2)), None);
    assert_eq!(k(f64::MIN_POSITIVE).first_reaching(d(1)), None);
}

#[test]
fn not_reached_before_its_time() {
    assert_eq!(g(0.1, 1), 10);
    assert_eq!(g(0.1, 1_000_000), 10_000_000);
    assert_eq!(g(0.25, 1_000_000), 4_000_000);
    assert_eq!(g(0.25, 1_000_001), 4_000_004);
    assert_eq!(g(0.3, 3), 10);
    assert_eq!(g(0.3, 300_000), 1_000_000);
    assert_eq!(g(0.3, 1_000_000), 3_333_334);
    assert_eq!(g(0.5, 500_000), 1_000_000);
    assert_eq!(g(0.5, 1_000_001), 2_000_002);
    assert_eq!(g(2.0, 1_000_000), 500_000);
    assert_eq!(g(2.0, 1_000_001), 500_001);
    assert_eq!(g(3.0, 3), 1);
    assert_eq!(g(3.0, 1_000_000), 333_334);
}

#[test]
fn first_reaching_zero_is_zero() {
    for scale in [0.5, 0.25, 0.1, 0.3, 1e-6, 2.0, 3.0] {
        assert_eq!(g(scale, 0), 0, "{scale}");
    }
}

#[test]
fn ticks_before_start_do_not_reach_zero() {
    for scale in [0.5, 0.25, 0.1, 0.3, 1e-6] {
        assert_eq!(k(scale).forward(d(-1)), d(-1), "{scale}");
    }
    assert_eq!(k(2.0).forward(d(-1)), d(-2));
    for scale in all_scales() {
        assert!(scale.forward(d(-1)) < TimeDelta::ZERO);
        assert_eq!(scale.forward(d(0)), TimeDelta::ZERO);
    }
}

#[test]
fn lower_bound_lap_lengths() {
    assert_eq!(g(0.5, 500_000), 1_000_000);
    assert_eq!(g(0.5, 499_999), 999_998);
}

#[test]
fn lap_length_is_never_negative() {
    for scale in all_scales() {
        for total in [0, 1, 499_999, 500_000, 1_000_000, 10_000_000_000] {
            let lap = scale.first_reaching(d(total));
            assert!(
                lap.is_some_and(|lap| !lap.is_negative()),
                "scale {} total {total}",
                scale.get()
            );
        }
        if let Some(lap) = scale.first_reaching(TimeDelta::MAX) {
            assert!(!lap.is_negative());
        }
    }
}

#[test]
fn mul_div_rounds_to_nearest() {
    assert_eq!(s(1_000_000).mul_div(1, 3), s(333_333));
    assert_eq!(s(1_000_000).mul_div(2, 3), s(666_667));
    assert_eq!(s(10_000).mul_div(31, 32), s(9_688));
    assert_eq!(s(10_000).mul_div(1, 32), s(313));
    assert_eq!(s(10_000).mul_div(1, 64), s(156));
    assert_eq!(s(0).mul_div(5, 7), s(0));
    assert_eq!(s(7).mul_div(0, 3), s(0));
}

#[test]
fn mul_div_uses_wide_intermediate_and_saturates() {
    assert_eq!(TimeSpan::MAX.mul_div(u64::MAX, u64::MAX), TimeSpan::MAX);
    assert_eq!(TimeSpan::MAX.mul_div(3, 6), s(i64::MAX / 2 + 1));
    assert_eq!(TimeSpan::MAX.mul_div(2, 1), TimeSpan::MAX);
}

#[test]
fn mul_unit_clamps_ratio_and_rounds_to_nearest() {
    assert_eq!(s(1_000_000).mul_unit(0.5), s(500_000));
    assert_eq!(s(3).mul_unit(0.5), s(2));
    assert_eq!(s(1_000_000).mul_unit(2.0), s(1_000_000));
    assert_eq!(s(1_000_000).mul_unit(-1.0), s(0));
    assert_eq!(s(1_000_000).mul_unit(f64::NAN), s(0));
    let big = s((1 << 60) + 1);
    assert_eq!(big.mul_unit(1.0), big);
    assert!(big.mul_unit(0.999_999_999) <= big);
    assert_eq!(TimeSpan::MAX.mul_unit(1.0), TimeSpan::MAX);
}

#[test]
fn ratio_of_is_integer_ratio() {
    assert_eq!(d(1).ratio_of(d(4)), Some(0.25));
    assert_eq!(d(-1).ratio_of(d(4)), Some(-0.25));
    assert_eq!(d(5).ratio_of(d(5)), Some(1.0));
    assert_eq!(d(5).ratio_of(TimeDelta::ZERO), None);
}
