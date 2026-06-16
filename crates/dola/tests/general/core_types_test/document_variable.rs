//! core_types: DolaDocument / AnimationVariableDef serde round-trip
//! Tasks 2.5, 3.4

use super::*;

// =============================================================
// Task 2.5: DolaDocument / AnimationVariableDef / DynamicValue serde round-trip
// =============================================================

mod document_tests {
    use super::*;

    #[test]
    fn minimal_document_json_roundtrip() {
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: DolaDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn document_with_variables_json_roundtrip() {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        );
        variable.insert(
            "count".to_string(),
            AnimationVariableDef::Integer {
                initial: 0,
                min: Some(0),
                max: Some(100),
                typewriter: None,
            },
        );
        variable.insert(
            "bg".to_string(),
            AnimationVariableDef::Object {
                initial: DynamicValue::Map({
                    let mut m = BTreeMap::new();
                    m.insert(
                        "path".to_string(),
                        DynamicValue::String("default.png".to_string()),
                    );
                    m
                }),
            },
        );

        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let deserialized: DolaDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, deserialized);
    }
}

mod variable_tests {
    use super::*;

    #[test]
    fn float_variable_json_roundtrip() {
        let var = AnimationVariableDef::Float {
            initial: 0.5,
            min: Some(0.0),
            max: Some(1.0),
        };
        let json = serde_json::to_string(&var).unwrap();
        assert!(json.contains(r#""type":"f64""#));
        let deserialized: AnimationVariableDef = serde_json::from_str(&json).unwrap();
        assert_eq!(var, deserialized);
    }

    #[test]
    fn integer_variable_json_roundtrip() {
        let var = AnimationVariableDef::Integer {
            initial: 42,
            min: Some(0),
            max: Some(100),
            typewriter: None,
        };
        let json = serde_json::to_string(&var).unwrap();
        assert!(json.contains(r#""type":"i64""#));
        let deserialized: AnimationVariableDef = serde_json::from_str(&json).unwrap();
        assert_eq!(var, deserialized);
    }

    #[test]
    fn integer_variable_with_typewriter_json_roundtrip() {
        let var = AnimationVariableDef::Integer {
            initial: 0,
            min: Some(0),
            max: None,
            typewriter: Some("こんにちは世界".to_string()),
        };
        let json = serde_json::to_string(&var).unwrap();
        let deserialized: AnimationVariableDef = serde_json::from_str(&json).unwrap();
        assert_eq!(var, deserialized);
    }

    #[test]
    fn object_variable_json_roundtrip() {
        let var = AnimationVariableDef::Object {
            initial: DynamicValue::Map({
                let mut m = BTreeMap::new();
                m.insert(
                    "path".to_string(),
                    DynamicValue::String("image.png".to_string()),
                );
                m
            }),
        };
        let json = serde_json::to_string(&var).unwrap();
        assert!(json.contains(r#""type":"object""#));
        let deserialized: AnimationVariableDef = serde_json::from_str(&json).unwrap();
        assert_eq!(var, deserialized);
    }
}
