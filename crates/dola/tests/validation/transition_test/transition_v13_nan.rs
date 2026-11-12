// =============================================================
// V13: 変数型とトランジション値型の整合性
// =============================================================

mod v13_tests {
    use super::super::*;

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
// #29: NaN は V12 の値域の比べを素通りするので、数値の規則が InvalidNumber で拒む
// =============================================================

mod v12_nan_tests {
    use super::super::*;

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

    fn invalid_number_fields(doc: &DolaDocument) -> Vec<String> {
        doc.validate()
            .unwrap_err()
            .into_iter()
            .map(|e| match e {
                DolaError::InvalidNumber { field, .. } => field,
                other => panic!("unexpected error: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn nan_initial_is_rejected() {
        let doc = doc_with_float_var("x", f64::NAN, Some(0.0), Some(1.0));
        assert_eq!(invalid_number_fields(&doc), ["initial"]);
    }

    #[test]
    fn nan_bounds_are_rejected() {
        let doc = doc_with_float_var("x", 5.0, Some(f64::NAN), Some(f64::NAN));
        assert_eq!(invalid_number_fields(&doc), ["min", "max"]);
    }

    #[test]
    fn nan_transition_value_is_rejected() {
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
        assert_eq!(invalid_number_fields(&doc), ["from", "to"]);
    }
}
