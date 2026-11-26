//! 文書の値の上限（100ns の数の絶対値が 2^51 未満）と秒の往復のテスト

use dola::*;

const LIMIT: i64 = 1 << 51;

fn span(ticks: i64) -> TimeSpan {
    TimeSpan::try_from_ticks(ticks).unwrap()
}

fn doc_with_inline_transition(def: TransitionDef) -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        .variable(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .storyboard(
            "sb",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(def)),
                    ..Default::default()
                })
                .build(),
        )
        .build()
        .unwrap()
}

fn transition(delay: TimeSpan, duration: TimeSpan) -> TransitionDef {
    TransitionDef {
        to: Some(TransitionValue::Scalar(1.0)),
        delay,
        duration: Some(duration),
        ..Default::default()
    }
}

#[test]
fn document_json_seconds_round_trip() {
    let doc = doc_with_inline_transition(transition(
        TimeSpan::from_millis(250),
        TimeSpan::from_millis(1500),
    ));
    let first = serde_json::to_string(&doc).unwrap();
    let read: DolaDocument = serde_json::from_str(&first).unwrap();
    assert_eq!(read, doc);
    assert_eq!(serde_json::to_string(&read).unwrap(), first);
}

#[test]
fn limit_edge_round_trips_through_a_document() {
    let doc = doc_with_inline_transition(transition(span(LIMIT - 1), span(LIMIT - 1)));
    let json = serde_json::to_string(&doc).unwrap();
    assert_eq!(serde_json::from_str::<DolaDocument>(&json).unwrap(), doc);
}

#[test]
fn time_beyond_the_limit_is_rejected_by_build_and_compile_storyboard() {
    let over = span(LIMIT);
    let make = |def: TransitionDef| DolaDocument {
        schema_version: "1.0".to_string(),
        variable: std::collections::BTreeMap::from([(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )]),
        transition: std::collections::BTreeMap::new(),
        storyboard: std::collections::BTreeMap::from([(
            "sb".to_string(),
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(def)),
                    ..Default::default()
                })
                .build(),
        )]),
    };
    let is_range_error = |errors: &[DolaError], field: &str| {
        errors
            .iter()
            .any(|e| matches!(e, DolaError::TimeOutOfDocumentRange { field: f, .. } if f == field))
    };

    for (def, field) in [
        (transition(over, TimeSpan::ZERO), "delay"),
        (transition(TimeSpan::ZERO, over), "duration"),
    ] {
        let doc = make(def.clone());
        let errors = compile_storyboard(&doc, "sb").unwrap_err();
        assert!(is_range_error(&errors, field), "{errors:?}");

        let errors = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard("sb", doc.storyboard["sb"].clone())
            .build()
            .unwrap_err();
        assert!(is_range_error(&errors, field), "{errors:?}");
    }
}

#[test]
fn time_beyond_the_limit_fails_to_write_and_to_read() {
    let over = TransitionDef {
        delay: span(LIMIT),
        ..Default::default()
    };
    assert!(serde_json::to_string(&over).is_err());
    assert!(serde_json::from_str::<TransitionDef>(r#"{"delay":225179981.3685248}"#).is_err());
}

#[cfg(feature = "toml")]
mod toml_tests {
    use super::*;

    #[test]
    fn limit_edge_round_trips_and_the_limit_fails_in_toml() {
        let doc = doc_with_inline_transition(transition(span(LIMIT - 1), span(LIMIT - 1)));
        let toml = doc.to_toml_string().unwrap();
        assert_eq!(DolaDocument::from_toml_str(&toml).unwrap(), doc);

        let over = DolaDocument {
            transition: std::collections::BTreeMap::from([(
                "t".to_string(),
                transition(span(LIMIT), TimeSpan::ZERO),
            )]),
            ..doc
        };
        assert!(over.to_toml_string().is_err());
        let text =
            "schema_version = \"1.0\"\n[transition.t]\nto = 1.0\ndelay = 225179981.3685248\n";
        assert!(DolaDocument::from_toml_str(text).is_err());
    }

    #[test]
    fn negative_and_non_finite_durations_fail_to_load_in_toml() {
        for field in [
            "duration = -1.0",
            "duration = nan",
            "duration = inf",
            "delay = -inf",
        ] {
            let text = format!("schema_version = \"1.0\"\n[transition.t]\nto = 1.0\n{field}\n");
            assert!(DolaDocument::from_toml_str(&text).is_err(), "{field}");
        }
    }
}
