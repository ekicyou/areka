//! CueSheet の直列化の結合テスト（JSON と、機能 `toml` の下の TOML）。
//!
//! 時刻と長さは 100ns の整数で書かれ、読み戻すと書く前と等しい。表せない値は読み込みの失敗。

use crate::test_support::{barrier, ms, text, wait};
use dola::cue::{ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet};
use dola::time::{TimeDelta, TimePoint, TimeSpan};
use serde_json::json;

/// 起点 0 からのミリ秒の時刻。
fn pt(millis: i64) -> TimePoint {
    TimePoint::from_ticks(millis * 10_000)
}

fn delta_ms(millis: i64) -> TimeDelta {
    TimeDelta::from_ticks(millis * 10_000)
}

fn choice_barrier(start_time: TimeSpan, timeout: Option<TimeDelta>) -> Cue {
    barrier(start_time, BarrierKind::WaitForChoice { timeout })
}

fn assert_same(parsed: &CueSheet, expected: &CueSheet) {
    assert_eq!(parsed.absolute_start_time(), expected.absolute_start_time());
    assert_eq!(parsed.cues().len(), expected.cues().len());
    for (p, e) in parsed.cues().iter().zip(expected.cues()) {
        assert_eq!(p.actor, e.actor);
        assert_eq!(p.start_time, e.start_time);
        assert_eq!(p.payload, e.payload);
        assert_eq!(p.duration, e.duration);
    }
}

fn sample_sheet() -> CueSheet {
    CueSheet::new(vec![
        text(TimeSpan::ZERO, "hello", ms(250)),
        Cue {
            actor: ActorKey::from("kero"),
            start_time: TimeSpan::try_from_ticks(1_234_567).unwrap(),
            payload: CueCommand::Emote { key: "3".into() }.into(),
            duration: TimeSpan::ZERO,
        },
        choice_barrier(ms(1000), Some(delta_ms(3000))),
        barrier(ms(2000), BarrierKind::WaitForInput),
        wait(ms(2000), ms(500)),
    ])
    .with_absolute_start_time(pt(123_456_789))
}

fn json_cue(start: &str, duration: &str) -> String {
    format!(
        r#"{{"actor":"a","start_time":{start},"payload":{{"Command":"Clear"}},"duration":{duration}}}"#
    )
}

// ============================================================================
// JSON
// ============================================================================

/// 起点・開始の隔たり・長さ・時間切れが 100ns の整数で書かれる。
#[test]
fn json_writes_times_as_integer_ticks() {
    let sheet = CueSheet::new(vec![
        text(ms(500), "hi", ms(250)),
        choice_barrier(ms(1000), Some(delta_ms(3000))),
    ])
    .with_absolute_start_time(pt(42_000));

    let value = serde_json::to_value(&sheet).unwrap();

    assert_eq!(value["absolute_start_time"], json!(420_000_000_i64));
    assert_eq!(value["cues"][0]["start_time"], json!(5_000_000_i64));
    assert_eq!(value["cues"][0]["duration"], json!(2_500_000_i64));
    assert_eq!(value["cues"][1]["start_time"], json!(10_000_000_i64));
    assert_eq!(
        value["cues"][1]["payload"]["Barrier"]["WaitForChoice"]["timeout"],
        json!(30_000_000_i64)
    );
}

#[test]
fn json_roundtrip_preserves_every_field() {
    let sheet = sample_sheet();

    let json = serde_json::to_string(&sheet).unwrap();
    let parsed: CueSheet = serde_json::from_str(&json).unwrap();

    assert_same(&parsed, &sheet);
}

/// 起点の押されていない台本は欄ごと書かれず、読み戻しても押されていない。
#[test]
fn json_unstamped_sheet_omits_origin_and_roundtrips() {
    let sheet = CueSheet::new(vec![text(ms(500), "hi", ms(250))]);

    let value = serde_json::to_value(&sheet).unwrap();
    assert!(value.get("absolute_start_time").is_none());

    let parsed: CueSheet = serde_json::from_value(value).unwrap();
    assert_eq!(parsed.absolute_start_time(), None);
    assert_same(&parsed, &sheet);
}

/// 起点の値は、押した値と同じ物差し（100ns の整数）のまま運ばれる。
#[test]
fn json_origin_is_carried_as_the_same_tick_value() {
    let origin = TimePoint::from_ticks(9_876_543_210);
    let sheet = CueSheet::new(vec![]).with_absolute_start_time(origin);

    let value = serde_json::to_value(&sheet).unwrap();
    assert_eq!(value["absolute_start_time"], json!(9_876_543_210_i64));

    let parsed: CueSheet = serde_json::from_value(value).unwrap();
    assert_eq!(parsed.absolute_start_time(), Some(origin));
}

/// 時間切れの 0 と負は、そのまま書いて、そのまま読み戻す。
#[test]
fn json_zero_and_negative_timeouts_roundtrip() {
    for timeout in [delta_ms(0), delta_ms(-1), TimeDelta::from_ticks(-1)] {
        let sheet = CueSheet::new(vec![choice_barrier(ms(100), Some(timeout))]);

        let json = serde_json::to_string(&sheet).unwrap();
        let parsed: CueSheet = serde_json::from_str(&json).unwrap();

        assert_same(&parsed, &sheet);
        assert_eq!(
            parsed.cues()[0].payload,
            CuePayload::Barrier(BarrierKind::WaitForChoice {
                timeout: Some(timeout)
            })
        );
    }
}

/// 時間切れの無い選択肢待ちは、欄ごと書かず、無いまま読み戻す。
#[test]
fn json_absent_timeout_is_omitted_and_roundtrips() {
    let sheet = CueSheet::new(vec![choice_barrier(ms(100), None)]);

    let value = serde_json::to_value(&sheet).unwrap();
    assert!(
        value["cues"][0]["payload"]["Barrier"]["WaitForChoice"]
            .get("timeout")
            .is_none()
    );

    let parsed: CueSheet = serde_json::from_value(value).unwrap();
    assert_same(&parsed, &sheet);
}

/// 欄の無い長さは 0 として読む。
#[test]
fn json_missing_duration_reads_as_zero() {
    let json = r#"{"cues":[
        {"actor":"sakura","start_time":0,"payload":{"Command":{"Text":"hi"}}}
    ]}"#;
    let sheet: CueSheet = serde_json::from_str(json).unwrap();

    assert_eq!(sheet.absolute_start_time(), None);
    assert_eq!(sheet.cues()[0].duration, TimeSpan::ZERO);
}

/// 整数でない値（実数の秒の形）は、起点・開始・長さ・時間切れのどれでも読み込みの失敗。
#[test]
fn json_non_integer_values_fail_to_load() {
    let timeout_float = r#"{"cues":[{"actor":"a","start_time":0,"payload":{"Barrier":{"WaitForChoice":{"timeout":3.0}}}}]}"#;
    let cases = [
        format!(
            r#"{{"absolute_start_time":1.5,"cues":[{}]}}"#,
            json_cue("0", "0")
        ),
        format!(r#"{{"cues":[{}]}}"#, json_cue("0.5", "0")),
        format!(r#"{{"cues":[{}]}}"#, json_cue("1.0", "0")),
        format!(r#"{{"cues":[{}]}}"#, json_cue("0", "0.25")),
        timeout_float.to_owned(),
    ];

    for json in cases {
        assert!(
            serde_json::from_str::<CueSheet>(&json).is_err(),
            "読み込めてはならない: {json}"
        );
    }
}

/// i64 に収まらない整数は、起点・開始・長さ・時間切れのどれでも読み込みの失敗。
#[test]
fn json_integer_beyond_i64_fails_to_load() {
    let big = "9223372036854775808";
    let timeout_big = format!(
        r#"{{"cues":[{{"actor":"a","start_time":0,"payload":{{"Barrier":{{"WaitForChoice":{{"timeout":{big}}}}}}}}}]}}"#
    );
    for json in [
        format!(
            r#"{{"absolute_start_time":{big},"cues":[{}]}}"#,
            json_cue("0", "0")
        ),
        format!(r#"{{"cues":[{}]}}"#, json_cue(big, "0")),
        format!(r#"{{"cues":[{}]}}"#, json_cue("0", big)),
        timeout_big,
    ] {
        assert!(
            serde_json::from_str::<CueSheet>(&json).is_err(),
            "読み込めてはならない: {json}"
        );
    }
}

/// 起点と時間切れの `null` は、欄が無い時と同じ「無い」として読む。
#[test]
fn json_null_origin_and_timeout_read_as_none() {
    let json = r#"{"absolute_start_time":null,"cues":[
        {"actor":"a","start_time":0,"payload":{"Barrier":{"WaitForChoice":{"timeout":null}}}}
    ]}"#;
    let sheet: CueSheet = serde_json::from_str(json).unwrap();

    assert_eq!(sheet.absolute_start_time(), None);
    assert_eq!(
        sheet.cues()[0].payload,
        CuePayload::Barrier(BarrierKind::WaitForChoice { timeout: None })
    );
}

/// 負の開始の隔たり・負の長さは読み込みの失敗。起点の負は読める。
#[test]
fn json_negative_lengths_fail_to_load() {
    for json in [
        format!(r#"{{"cues":[{}]}}"#, json_cue("-1", "0")),
        format!(r#"{{"cues":[{}]}}"#, json_cue("0", "-1")),
        format!(r#"{{"cues":[{}]}}"#, json_cue(&i64::MIN.to_string(), "0")),
    ] {
        assert!(
            serde_json::from_str::<CueSheet>(&json).is_err(),
            "読み込めてはならない: {json}"
        );
    }

    let negative_origin: CueSheet = serde_json::from_str(&format!(
        r#"{{"absolute_start_time":-5,"cues":[{}]}}"#,
        json_cue("0", "0")
    ))
    .unwrap();
    assert_eq!(
        negative_origin.absolute_start_time(),
        Some(TimePoint::from_ticks(-5))
    );
}

/// 付け替えの前の形（実数の秒）の台本は読めない。
#[test]
fn json_legacy_float_seconds_sheet_is_rejected() {
    let json = r#"{"absolute_start_time":100.0,"cues":[
        {"actor":"sakura","start_time":0.5,"payload":{"Command":{"Text":"hi"}},"duration":0.25}
    ]}"#;

    assert!(serde_json::from_str::<CueSheet>(json).is_err());
}

// ============================================================================
// TOML（機能 `toml`）
// ============================================================================

#[cfg(feature = "toml")]
mod toml_format {
    use super::*;

    #[test]
    fn toml_roundtrip_preserves_every_field() {
        let sheet = sample_sheet();

        let doc = toml::to_string(&sheet).unwrap();
        let parsed: CueSheet = toml::from_str(&doc).unwrap();

        assert_same(&parsed, &sheet);
    }

    #[test]
    fn toml_writes_times_as_integer_ticks() {
        let sheet =
            CueSheet::new(vec![text(ms(500), "hi", ms(250))]).with_absolute_start_time(pt(42_000));

        let value: toml::Value = toml::from_str(&toml::to_string(&sheet).unwrap()).unwrap();

        assert_eq!(value["absolute_start_time"].as_integer(), Some(420_000_000));
        assert_eq!(value["cues"][0]["start_time"].as_integer(), Some(5_000_000));
        assert_eq!(value["cues"][0]["duration"].as_integer(), Some(2_500_000));
    }

    #[test]
    fn toml_unstamped_sheet_omits_origin_and_roundtrips() {
        let sheet = CueSheet::new(vec![text(ms(500), "hi", ms(250))]);

        let doc = toml::to_string(&sheet).unwrap();
        assert!(!doc.contains("absolute_start_time"));

        let parsed: CueSheet = toml::from_str(&doc).unwrap();
        assert_eq!(parsed.absolute_start_time(), None);
        assert_same(&parsed, &sheet);
    }

    #[test]
    fn toml_zero_and_negative_timeouts_roundtrip() {
        for timeout in [delta_ms(0), delta_ms(-1), TimeDelta::from_ticks(-1)] {
            let sheet = CueSheet::new(vec![choice_barrier(ms(100), Some(timeout))]);

            let doc = toml::to_string(&sheet).unwrap();
            let parsed: CueSheet = toml::from_str(&doc).unwrap();

            assert_same(&parsed, &sheet);
        }
    }

    #[test]
    fn toml_non_integer_and_negative_values_fail_to_load() {
        let cue = |start: &str, duration: &str| {
            format!(
                "[[cues]]\nactor = \"a\"\nstart_time = {start}\nduration = {duration}\npayload = {{ Command = \"Clear\" }}\n"
            )
        };

        for doc in [
            cue("0.5", "0"),
            cue("0", "0.25"),
            cue("-1", "0"),
            cue("0", "-1"),
            format!("absolute_start_time = 1.5\n{}", cue("0", "0")),
        ] {
            assert!(
                toml::from_str::<CueSheet>(&doc).is_err(),
                "読み込めてはならない: {doc}"
            );
        }
    }
}
