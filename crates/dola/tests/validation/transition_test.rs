//! Validation tests for V7, V10-V13 (transition constraints, object type, value range, type mismatch)
//! Tasks 7.6, 8.6

use dola::*;
use std::collections::BTreeMap;

use super::common::minimal_valid_doc;

/// ヘルパー: f64変数付きドキュメント
fn doc_with_float_var(
    name: &str,
    initial: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        name.to_string(),
        AnimationVariableDef::Float { initial, min, max },
    );
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard: BTreeMap::new(),
    }
}

// =============================================================
// V7: transition あり → variable 必須
// =============================================================

mod v7_tests {
    use super::*;

    #[test]
    fn transition_without_variable_error() {
        let mut doc = minimal_valid_doc();
        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::InvalidEntry { reason, .. }
            if reason.contains("transition requires variable")
        )));
    }
}

// =============================================================
// V10: Object型トランジション制限
// =============================================================

mod v10_tests {
    use super::*;

    #[test]
    fn object_with_from_error() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "bg".to_string(),
            AnimationVariableDef::Object {
                initial: DynamicValue::Null,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("bg".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Dynamic(DynamicValue::Null)), // not allowed!
                        to: Some(TransitionValue::Dynamic(DynamicValue::String(
                            "img.png".to_string(),
                        ))),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None,
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ObjectTransitionViolation { field, .. }
            if field == "from"
        )));
    }

    #[test]
    fn object_with_scalar_to_error() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "bg".to_string(),
            AnimationVariableDef::Object {
                initial: DynamicValue::Null,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("bg".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)), // Object variable + Scalar = error
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None,
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::TypeMismatch { reason, .. }
            if reason.contains("Object variable requires Dynamic")
        )));
    }
}

// =============================================================
// V11: to/relative_to 排他
// =============================================================

mod v11_tests {
    use super::*;

    #[test]
    fn to_and_relative_to_mutually_exclusive() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: Some(50.0), // both specified!
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::MutuallyExclusive { .. }))
        );
    }
}

// =============================================================
// V12: 値域検証
// =============================================================

mod v12_tests {
    use super::*;

    #[test]
    fn initial_above_max_error() {
        let doc = doc_with_float_var("x", 1.5, Some(0.0), Some(1.0));
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, field, .. }
            if variable == "x" && field == "initial"
        )));
    }

    #[test]
    fn initial_below_min_error() {
        let doc = doc_with_float_var("x", -1.0, Some(0.0), Some(1.0));
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, field, .. }
            if variable == "x" && field == "initial"
        )));
    }

    #[test]
    fn transition_to_out_of_range_error() {
        let mut doc = doc_with_float_var("x", 0.5, Some(0.0), Some(100.0));
        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(200.0)), // out of range!
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, field, .. }
            if variable == "x" && field == "to"
        )));
    }

    #[test]
    fn i64_variable_initial_out_of_range() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "count".to_string(),
            AnimationVariableDef::Integer {
                initial: 200,
                min: Some(0),
                max: Some(100),
                typewriter: None,
            },
        );
        doc.variable = variable;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, .. }
            if variable == "count"
        )));
    }
}

// =============================================================
// V13: 変数型とトランジション値型の整合性
// =============================================================

mod v13_tests {
    use super::*;

    #[test]
    fn float_variable_with_dynamic_to_error() {
        let mut doc = doc_with_float_var("x", 0.0, None, None);
        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Dynamic(DynamicValue::String(
                            "bad".to_string(),
                        ))),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::TypeMismatch { reason, .. }
            if reason.contains("Numeric variable requires Scalar")
        )));
    }

    #[test]
    fn object_variable_with_scalar_to_error() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "bg".to_string(),
            AnimationVariableDef::Object {
                initial: DynamicValue::Null,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("bg".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)), // Object + Scalar = error
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None,
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::TypeMismatch { reason, .. }
            if reason.contains("Object variable requires Dynamic")
        )));
    }
}
