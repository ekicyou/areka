// =============================================================
// V11: to/relative_to 排他
// =============================================================

mod v11_tests {
    use super::super::*;

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
    use super::super::*;

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
    use super::super::*;

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
