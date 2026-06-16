//! Serialization round-trip tests (JSON / TOML / YAML)
//! Tasks 11.2, 11.3, 11.4

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
// Task 11.3: TOML round-trip (feature "toml")
// =============================================================

#[cfg(feature = "toml")]
mod toml_integration_tests {
    use super::super::*;

    #[test]
    fn complete_document_toml_roundtrip() {
        let doc = build_complete_document();
        let toml_str = toml::to_string_pretty(&doc).unwrap();
        let deserialized: DolaDocument = toml::from_str(&toml_str).unwrap();
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn btreemap_key_order_deterministic_toml() {
        let doc = build_complete_document();
        let toml1 = toml::to_string_pretty(&doc).unwrap();
        let toml2 = toml::to_string_pretty(&doc).unwrap();
        assert_eq!(toml1, toml2, "BTreeMap key order must be deterministic");
    }
}

// =============================================================
// Task 11.4: YAML round-trip (feature "yaml")
// =============================================================

#[cfg(feature = "yaml")]
mod yaml_integration_tests {
    use super::super::*;

    #[test]
    fn complete_document_yaml_roundtrip() {
        let doc = build_complete_document();
        let yaml_str = serde_yaml::to_string(&doc).unwrap();
        let deserialized: DolaDocument = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(doc, deserialized);
    }
}
