use super::*;

const EDGES: [i64; 9] = [
    i64::MIN,
    i64::MIN + 1,
    -10_000_001,
    -1,
    0,
    1,
    500_000,
    123_456_789,
    i64::MAX,
];

#[test]
fn display_shows_seven_digits() {
    assert_eq!(TimeDelta::from_ticks(123_456_789).to_string(), "12.3456789");
    assert_eq!(TimeDelta::from_ticks(-500_000).to_string(), "-0.0500000");
    assert_eq!(TimePoint::ZERO.to_string(), "0.0000000");
    assert_eq!(TimeDelta::MIN.to_string(), "-922337203685.4775808");
    assert_eq!(TimeSpan::MAX.to_string(), "922337203685.4775807");
}

#[test]
fn point_and_delta_round_trip_edges() {
    for t in EDGES {
        let p = TimePoint::from_ticks(t);
        assert_eq!(p.to_string().parse::<TimePoint>(), Ok(p));
        let d = TimeDelta::from_ticks(t);
        assert_eq!(d.to_string().parse::<TimeDelta>(), Ok(d));
    }
}

#[test]
fn span_round_trips_and_rejects_negative() {
    for t in EDGES.into_iter().filter(|t| *t >= 0) {
        let s = TimeSpan::try_from_ticks(t).unwrap();
        assert_eq!(s.to_string().parse::<TimeSpan>(), Ok(s));
    }
    assert_eq!(
        "-0.0000001".parse::<TimeSpan>(),
        Err(TimeParseError::OutOfRange)
    );
}

#[test]
fn parse_accepts_short_fractions_and_rejects_bad_text() {
    assert_eq!(
        "1.5".parse::<TimeDelta>(),
        Ok(TimeDelta::from_ticks(15_000_000))
    );
    assert_eq!(
        "2".parse::<TimeDelta>(),
        Ok(TimeDelta::from_ticks(20_000_000))
    );
    for bad in [
        "",
        "-",
        ".5",
        "1.",
        "1.12345678",
        "+1",
        "1e3",
        "1.2.3",
        " 1",
        "a",
    ] {
        assert_eq!(
            bad.parse::<TimeDelta>(),
            Err(TimeParseError::Malformed),
            "{bad:?}"
        );
    }
    for over in [
        "922337203685.4775808",
        "-922337203685.4775809",
        "99999999999999999999999999999999999999999",
    ] {
        assert_eq!(
            over.parse::<TimeDelta>(),
            Err(TimeParseError::OutOfRange),
            "{over:?}"
        );
    }
}
