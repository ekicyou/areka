use serde::{Deserialize, Serialize};

use super::super::{TimeDelta, TimePoint, TimeSpan};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Fields {
    #[serde(with = "crate::time::serde_ticks")]
    span: TimeSpan,
    #[serde(with = "crate::time::serde_ticks")]
    delta: TimeDelta,
    #[serde(
        with = "crate::time::serde_ticks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    origin: Option<TimePoint>,
}

fn fields(span: i64, delta: i64, origin: Option<i64>) -> Fields {
    Fields {
        span: TimeSpan::try_from_ticks(span).unwrap(),
        delta: TimeDelta::from_ticks(delta),
        origin: origin.map(TimePoint::from_ticks),
    }
}

#[test]
fn writes_and_reads_integers() {
    let value = fields(5_000_000, -10_000, Some(420_000_000));
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(
        json,
        r#"{"span":5000000,"delta":-10000,"origin":420000000}"#
    );
    assert_eq!(serde_json::from_str::<Fields>(&json).unwrap(), value);
}

#[test]
fn edge_values_round_trip() {
    let value = fields(i64::MAX, i64::MIN, Some(i64::MIN));
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(serde_json::from_str::<Fields>(&json).unwrap(), value);
}

#[test]
fn absent_option_is_omitted_and_missing_or_null_reads_as_none() {
    let value = fields(0, 0, None);
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        r#"{"span":0,"delta":0}"#
    );
    for json in [
        r#"{"span":0,"delta":0}"#,
        r#"{"span":0,"delta":0,"origin":null}"#,
    ] {
        assert_eq!(serde_json::from_str::<Fields>(json).unwrap(), value);
    }
}

#[test]
fn rejects_negative_span_non_integer_and_out_of_range() {
    for json in [
        r#"{"span":-1,"delta":0}"#,
        r#"{"span":1.5,"delta":0}"#,
        r#"{"span":0,"delta":9223372036854775808}"#,
    ] {
        assert!(serde_json::from_str::<Fields>(json).is_err(), "{json}");
    }
}
