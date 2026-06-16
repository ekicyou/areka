// =============================================================
// V7: transition あり → variable 必須
// =============================================================

mod v7_tests {
    use super::super::*;

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
    use super::super::*;

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
    use super::super::*;

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
