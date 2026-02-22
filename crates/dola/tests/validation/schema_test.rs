//! Validation tests for V1-V5 (schema version, keyframe names, variable/transition references)
//! Tasks 7.6, 8.6

use dola::*;
use std::collections::BTreeMap;

use super::common::minimal_valid_doc;

// =============================================================
// V1: スキーマバージョン検証
// =============================================================

mod v1_tests {
    use super::*;

    #[test]
    fn schema_version_1_0_ok() {
        let doc = minimal_valid_doc();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn schema_version_mismatch() {
        let doc = DolaDocument {
            schema_version: "2.0".to_string(),
            ..minimal_valid_doc()
        };
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::SchemaVersionMismatch { expected, found }
            if expected == "1.0" && found == "2.0"
        )));
    }
}

// =============================================================
// V2: キーフレーム名重複検出
// =============================================================

mod v2_tests {
    use super::*;

    #[test]
    fn duplicate_keyframe_detected() {
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
                entry: vec![
                    StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(1.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        keyframe: Some("visible".to_string()),
                        ..Default::default()
                    },
                    StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(2.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        keyframe: Some("visible".to_string()), // duplicate!
                        ..Default::default()
                    },
                ],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::DuplicateKeyframe { storyboard, name }
            if storyboard == "sb1" && name == "visible"
        )));
    }
}

// =============================================================
// V3: 予約キーフレーム名 "start" 使用禁止
// =============================================================

mod v3_tests {
    use super::*;

    #[test]
    fn reserved_keyframe_start_rejected() {
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
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("start".to_string()), // reserved!
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::ReservedKeyframeName { name } if name == "start"))
        );
    }
}

// =============================================================
// V4: 未定義変数参照
// =============================================================

mod v4_tests {
    use super::*;

    #[test]
    fn undefined_variable_detected() {
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
                    variable: Some("undefined_var".to_string()),
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
            DolaError::UndefinedVariable { name, .. }
            if name == "undefined_var"
        )));
    }
}

// =============================================================
// V5: 未定義トランジション参照
// =============================================================

mod v5_tests {
    use super::*;

    #[test]
    fn undefined_transition_detected() {
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
                    transition: Some(TransitionRef::Named("undefined_trans".to_string())),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedTransition { name, .. }
            if name == "undefined_trans"
        )));
    }
}
