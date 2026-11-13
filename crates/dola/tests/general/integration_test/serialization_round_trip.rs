//! Serialization round-trip tests (JSON: dev-only `serde_json`; TOML: feature `toml`)
//! Tasks 11.2, 11.4

// =============================================================
// Task 11.2: JSON round-trip
// =============================================================

mod json_integration_tests {
    use super::super::*;

    #[test]
    fn complete_document_json_roundtrip() {
        let doc = build_complete_document();
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let deserialized: DolaDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn easing_name_snake_case_in_json() {
        let doc = build_complete_document();
        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("quadratic_in_out"));
    }

    #[test]
    fn interruption_policy_snake_case_in_json() {
        let sb = StoryboardBuilder::new()
            .interruption_policy(InterruptionPolicy::Conclude)
            .entry(StoryboardEntry {
                keyframe: Some("kf".to_string()),
                ..Default::default()
            })
            .build();
        let json = serde_json::to_string(&sb).unwrap();
        assert!(json.contains("conclude"));
    }
}

// =============================================================
// TOML round-trip and read/write failures (feature `toml`)
// =============================================================

#[cfg(feature = "toml")]
mod toml_integration_tests {
    use super::super::*;

    #[test]
    fn complete_document_toml_roundtrip() {
        let doc = build_complete_document();
        let toml = doc.to_toml_string().unwrap();
        let deserialized = DolaDocument::from_toml_str(&toml).unwrap();
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn toml_output_is_identical_twice() {
        let doc = build_complete_document();
        assert_eq!(doc.to_toml_string().unwrap(), doc.to_toml_string().unwrap());
    }

    #[test]
    fn toml_read_errors_carry_a_reason() {
        let malformed = DolaDocument::from_toml_str("schema_version = ").unwrap_err();
        assert!(!malformed.to_string().is_empty());

        let missing = DolaDocument::from_toml_str("").unwrap_err();
        assert!(missing.to_string().contains("schema_version"));
    }

    #[test]
    fn toml_write_of_null_is_an_error() {
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::from([(
                "obj".to_string(),
                AnimationVariableDef::Object {
                    initial: DynamicValue::Null,
                },
            )]),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };
        assert!(doc.to_toml_string().is_err());
    }
}
