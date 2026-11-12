// =============================================================
// V13: 変数型とトランジション値型の整合性
// =============================================================

mod v13_tests {
    use super::super::*;

    fn type_mismatch(doc: &DolaDocument, needle: &str) -> bool {
        doc.validate().unwrap_err().iter().any(|e| {
            matches!(
                e,
                DolaError::TypeMismatch { reason, .. } if reason.contains(needle)
            )
        })
    }

    #[test]
    fn float_variable_with_dynamic_to_error() {
        let doc = with_transition(
            doc_with_float_var("x", 0.0, None, None),
            "x",
            TransitionDef {
                to: Some(TransitionValue::Dynamic(DynamicValue::String(
                    "bad".to_string(),
                ))),
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert!(type_mismatch(&doc, "Numeric variable requires Scalar"));
    }

    #[test]
    fn float_variable_with_dynamic_from_error() {
        // D3-T 追加: to 側は既存テストあり。from 側の Dynamic 拒否を固定する
        let doc = with_transition(
            doc_with_float_var("x", 0.0, None, None),
            "x",
            TransitionDef {
                from: Some(TransitionValue::Dynamic(DynamicValue::Null)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert!(type_mismatch(&doc, "Scalar transition value for 'from'"));
    }

    #[test]
    fn object_variable_with_scalar_to_error() {
        let doc = with_transition(
            doc_with_object_var(),
            "bg",
            TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)), // Object + Scalar = error
                ..Default::default()
            },
        );
        assert!(type_mismatch(&doc, "Object variable requires Dynamic"));
    }
}

// =============================================================
// #29: NaN は V12 の値域の比べを素通りするので、数値の規則が InvalidNumber で拒む
// =============================================================

mod v12_nan_tests {
    use super::super::*;

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
            "x",
            TransitionDef {
                from: Some(TransitionValue::Scalar(f64::NAN)),
                to: Some(TransitionValue::Scalar(f64::NAN)),
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert_eq!(invalid_number_fields(&doc), ["from", "to"]);
    }
}
