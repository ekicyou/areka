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
// V10 追加（D3-T）: Object 型の relative_to / easing 禁止
// =============================================================

mod v10_relative_easing_tests {
    use super::*;

    /// ヘルパー: Object 変数 "bg" 付きドキュメントに指定トランジションの sb1 を設定
    fn object_doc_with_transition(trans: TransitionDef) -> DolaDocument {
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
                    transition: Some(TransitionRef::Inline(trans)),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;
        doc
    }

    #[test]
    fn object_with_relative_to_error() {
        let doc = object_doc_with_transition(TransitionDef {
            from: None,
            to: None,
            relative_to: Some(10.0), // Object 型に relative_to は禁止
            easing: None,
            delay: 0.0,
            duration: None,
        });

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ObjectTransitionViolation { field, .. }
            if field == "relative_to"
        )));
    }

    #[test]
    fn object_with_easing_error() {
        let doc = object_doc_with_transition(TransitionDef {
            from: None,
            to: Some(TransitionValue::Dynamic(DynamicValue::String(
                "img.png".to_string(),
            ))),
            relative_to: None,
            easing: Some(EasingFunction::Named(EasingName::Linear)), // Object 型に easing は禁止
            delay: 0.0,
            duration: None,
        });

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ObjectTransitionViolation { field, .. }
            if field == "easing"
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
// V12 追加（D3-T）: from の値域・境界値・Integer 変数のトランジション値域
// =============================================================

mod v12_boundary_tests {
    use super::*;

    /// ヘルパー: 変数 "x" に指定トランジションの sb1 を追加
    fn with_transition(mut doc: DolaDocument, trans: TransitionDef) -> DolaDocument {
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
                    transition: Some(TransitionRef::Inline(trans)),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;
        doc
    }

    #[test]
    fn transition_from_out_of_range_error() {
        // to の値域検証は既存テストあり。from 側の検出を固定する
        let doc = with_transition(
            doc_with_float_var("x", 0.5, Some(0.0), Some(10.0)),
            TransitionDef {
                from: Some(TransitionValue::Scalar(-5.0)), // min 未満
                to: Some(TransitionValue::Scalar(5.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            },
        );

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, field, .. }
            if variable == "x" && field == "from"
        )));
    }

    #[test]
    fn values_exactly_at_min_max_are_valid() {
        // 値域検証は排他的比較（< min / > max）— 境界値ちょうどはエラーにならない
        let doc = with_transition(
            doc_with_float_var("x", 1.0, Some(0.0), Some(1.0)), // initial == max
            TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)), // == min
                to: Some(TransitionValue::Scalar(1.0)),   // == max
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            },
        );

        assert!(doc.validate().is_ok());
    }

    #[test]
    fn i64_variable_initial_below_min_error() {
        // Integer の min 側検証（既存テストは max 側のみ）
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "count".to_string(),
            AnimationVariableDef::Integer {
                initial: -5,
                min: Some(0),
                max: Some(100),
                typewriter: None,
            },
        );
        doc.variable = variable;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, field, .. }
            if variable == "count" && field == "initial"
        )));
    }

    #[test]
    fn i64_variable_transition_to_out_of_range_error() {
        // Integer 変数のトランジション値域検証（min/max の f64 変換経路）
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Integer {
                initial: 0,
                min: Some(0),
                max: Some(10),
                typewriter: None,
            },
        );
        doc.variable = variable;
        let doc = with_transition(
            doc,
            TransitionDef {
                from: None,
                to: Some(TransitionValue::Scalar(11.0)), // max 超過
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            },
        );

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ValueOutOfRange { variable, field, .. }
            if variable == "x" && field == "to"
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
    fn float_variable_with_dynamic_from_error() {
        // D3-T 追加: to 側は既存テストあり。from 側の Dynamic 拒否を固定する
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
                        from: Some(TransitionValue::Dynamic(DynamicValue::Null)),
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
            DolaError::TypeMismatch { reason, .. }
            if reason.contains("Scalar transition value for 'from'")
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

// =============================================================
// D3-V 特性化: NaN の V12 値域検証素通り
// =============================================================

mod v12_nan_tests {
    use super::*;

    /// ヘルパー: "x" 変数へインライントランジションを張る storyboard を設定
    fn with_transition(mut doc: DolaDocument, trans: TransitionDef) -> DolaDocument {
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
                    transition: Some(TransitionRef::Inline(trans)),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;
        doc
    }

    #[test]
    fn nan_initial_passes_range_validation() {
        // 特性化: initial=NaN は min/max との比較（< / >）が常に false となるため、
        // 値域 [0.0, 1.0] を持つ変数でも V12 を素通りして合格する（P14 参照）。
        let doc = doc_with_float_var("x", f64::NAN, Some(0.0), Some(1.0));
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn nan_bounds_pass_range_validation() {
        // 特性化: min=NaN / max=NaN は initial との比較が常に false となるため、
        // 退化した値域定義が検出されないまま合格する（P14 参照）。
        let doc = doc_with_float_var("x", 5.0, Some(f64::NAN), Some(f64::NAN));
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn nan_transition_value_passes_range_validation() {
        // 特性化: from/to の Scalar(NaN) は値域比較（< min / > max）が常に false と
        // なるため、V12 のトランジション値域検証を素通りして合格する（P14 参照）。
        let doc = with_transition(
            doc_with_float_var("x", 0.5, Some(0.0), Some(1.0)),
            TransitionDef {
                from: Some(TransitionValue::Scalar(f64::NAN)),
                to: Some(TransitionValue::Scalar(f64::NAN)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            },
        );
        assert!(doc.validate().is_ok());
    }
}
