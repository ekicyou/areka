// =============================================================
// V7: variable と transition は両方あるか両方無いか
// =============================================================

mod v7_tests {
    use super::super::*;

    #[test]
    fn transition_without_variable_error() {
        let mut doc = minimal_valid_doc();
        doc.storyboard = sb(vec![StoryboardEntry {
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            ..Default::default()
        }]);

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::InvalidEntry { reason, .. }
            if reason.contains("transition requires variable")
        )));
    }

    #[test]
    fn variable_without_transition_error() {
        // 片方だけの variable は compile で「transition が無い」と落ちる。検証で先に拒む
        let mut doc = doc_with_float_var("x", 0.0, None, None);
        doc.storyboard = sb(vec![StoryboardEntry {
            variable: Some("x".to_string()),
            ..Default::default()
        }]);

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::InvalidEntry { entry_index: 0, reason, .. }
            if reason.contains("variable requires transition")
        )));
    }
}

// =============================================================
// V10: Object型トランジション制限（Object に Scalar の to は V13 の型の整合で見る）
// =============================================================

mod v10_tests {
    use super::super::*;

    fn object_violation_fields(trans: TransitionDef) -> Vec<String> {
        with_transition(doc_with_object_var(), "bg", trans)
            .validate()
            .unwrap_err()
            .into_iter()
            .filter_map(|e| match e {
                DolaError::ObjectTransitionViolation { field, .. } => Some(field),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn object_with_from_error() {
        let fields = object_violation_fields(TransitionDef {
            from: Some(TransitionValue::Dynamic(DynamicValue::Null)), // not allowed!
            to: Some(TransitionValue::Dynamic(DynamicValue::String(
                "img.png".to_string(),
            ))),
            ..Default::default()
        });
        assert!(fields.iter().any(|f| f == "from"));
    }

    #[test]
    fn object_with_relative_to_error() {
        let fields = object_violation_fields(TransitionDef {
            relative_to: Some(10.0), // Object 型に relative_to は禁止
            ..Default::default()
        });
        assert!(fields.iter().any(|f| f == "relative_to"));
    }

    #[test]
    fn object_with_easing_error() {
        let fields = object_violation_fields(TransitionDef {
            to: Some(TransitionValue::Dynamic(DynamicValue::String(
                "img.png".to_string(),
            ))),
            easing: Some(EasingFunction::Named(EasingName::Linear)), // Object 型に easing は禁止
            ..Default::default()
        });
        assert!(fields.iter().any(|f| f == "easing"));
    }
}
