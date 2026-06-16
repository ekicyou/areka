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
// D3-V 特性化: NaN の V12 値域検証素通り
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
