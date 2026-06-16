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
// D3-T 追加: 複数ルールのエラー蓄積（fail-fast しないことの固定）
// =============================================================

mod error_accumulation_tests {
    use super::*;

    #[test]
    fn validate_collects_errors_from_multiple_rules_in_one_pass() {
        // V1（スキーマ不一致）と V4（未定義変数）が 1 回の validate() で両方報告される
        let mut doc = DolaDocument {
            schema_version: "9.9".to_string(),
            ..minimal_valid_doc()
        };
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
        assert!(errors.len() >= 2, "expected >=2 errors, got {errors:?}");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::SchemaVersionMismatch { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::UndefinedVariable { name, .. } if name == "undefined_var"))
        );
    }
}

// =============================================================
// D3-V 特性化: loop_count は文書バリデーションの対象外
// =============================================================

mod loop_count_gap_tests {
    use super::*;

    fn doc_with_loop_count(loop_count: i32) -> DolaDocument {
        let mut doc = minimal_valid_doc();
        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![],
            },
        );
        doc.storyboard = storyboard;
        doc
    }

    #[test]
    fn invalid_loop_count_passes_document_validation() {
        // 特性化: スキーマ仕様（storyboard.rs: 「0以下 = エラー、-1 = 無限」）に
        // 反する loop_count は validate()/compile では検査されず素通りし、
        // ランタイムの start 時（facade の InvalidLoopCount）で初めて拒否される
        // （tests/runtime/facade_test.rs::start_with_zero_loop_count_fails 等で
        // 後置検出を固定済み）。文書レベル検証の追加は P26 を参照。
        assert!(doc_with_loop_count(0).validate().is_ok());
        assert!(doc_with_loop_count(-2).validate().is_ok());
        // -1（無限ループ）と正値は当然合格
        assert!(doc_with_loop_count(-1).validate().is_ok());
        assert!(doc_with_loop_count(3).validate().is_ok());
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
