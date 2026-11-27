use serde::{Deserialize, Serialize};

use super::super::{TimeDelta, TimeSpan};
use super::within_document_range;

const LIMIT: i64 = 1 << 51;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Span {
    #[serde(with = "crate::time::serde_secs")]
    v: TimeSpan,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Delta {
    #[serde(with = "crate::time::serde_secs", default)]
    v: Option<TimeDelta>,
}

fn span(ticks: i64) -> Span {
    Span {
        v: TimeSpan::try_from_ticks(ticks).unwrap(),
    }
}

fn delta(ticks: i64) -> Delta {
    Delta {
        v: Some(TimeDelta::from_ticks(ticks)),
    }
}

fn secs_json(ticks: i64) -> String {
    format!(r#"{{"v":{}}}"#, ticks as f64 / 10_000_000.0)
}

#[test]
fn writes_seconds() {
    assert_eq!(
        serde_json::to_string(&span(15_000_000)).unwrap(),
        r#"{"v":1.5}"#
    );
    assert_eq!(
        serde_json::to_string(&delta(-500_000)).unwrap(),
        r#"{"v":-0.05}"#
    );
}

#[test]
fn read_write_read_is_stable() {
    for ticks in [0, 1, 333_333, 15_000_000, 123_456_789_012, LIMIT - 1] {
        let first: Span = serde_json::from_str(&secs_json(ticks)).unwrap();
        let again: Span = serde_json::from_str(&serde_json::to_string(&first).unwrap()).unwrap();
        assert_eq!(first, again);
        assert_eq!(first, span(ticks));
    }
}

#[test]
fn just_below_the_limit_round_trips_on_both_sides() {
    for ticks in [LIMIT - 1, -(LIMIT - 1)] {
        let value = delta(ticks);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<Delta>(&json).unwrap(), value);
    }
    let value = span(LIMIT - 1);
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(serde_json::from_str::<Span>(&json).unwrap(), value);
}

#[test]
fn the_limit_fails_on_write() {
    assert!(serde_json::to_string(&span(LIMIT)).is_err());
    assert!(serde_json::to_string(&span(i64::MAX)).is_err());
    assert!(serde_json::to_string(&delta(LIMIT)).is_err());
    assert!(serde_json::to_string(&delta(-LIMIT)).is_err());
    assert!(serde_json::to_string(&delta(i64::MIN)).is_err());
}

#[test]
fn the_limit_fails_on_read() {
    assert!(serde_json::from_str::<Span>(&secs_json(LIMIT)).is_err());
    assert!(serde_json::from_str::<Delta>(&secs_json(LIMIT)).is_err());
    assert!(serde_json::from_str::<Delta>(&secs_json(-LIMIT)).is_err());
    assert!(serde_json::from_str::<Span>(r#"{"v":1e300}"#).is_err());
}

#[test]
fn read_rejects_negative_span() {
    assert!(serde_json::from_str::<Span>(r#"{"v":-0.1}"#).is_err());
}

#[test]
fn missing_or_null_option_reads_as_none() {
    for json in ["{}", r#"{"v":null}"#] {
        assert_eq!(
            serde_json::from_str::<Delta>(json).unwrap(),
            Delta { v: None }
        );
    }
}

#[test]
fn range_check_edges() {
    assert!(within_document_range(LIMIT - 1));
    assert!(within_document_range(-(LIMIT - 1)));
    assert!(!within_document_range(LIMIT));
    assert!(!within_document_range(-LIMIT));
    assert!(!within_document_range(i64::MIN));
}
