// =============================================================
// V11: to/relative_to 排他
// =============================================================

mod v11_tests {
    use super::super::*;

    #[test]
    fn to_and_relative_to_mutually_exclusive() {
        let doc = with_transition(
            doc_with_float_var("x", 0.0, None, None),
            "x",
            TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                relative_to: Some(50.0), // both specified!
                duration: Some(1.0),
                ..Default::default()
            },
        );

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

    fn doc_with_int_var(name: &str, initial: i64, min: i64, max: i64) -> DolaDocument {
        let mut doc = minimal_valid_doc();
        doc.variable.insert(
            name.to_string(),
            AnimationVariableDef::Integer {
                initial,
                min: Some(min),
                max: Some(max),
            },
        );
        doc
    }

    /// 初期値の違反は変数だけ、遷移の違反は台本 "sb1" の entry 0 の場所つき（#70）
    fn out_of_range(doc: &DolaDocument, var: &str, fld: &str) -> bool {
        doc.validate().unwrap_err().iter().any(|e| match e {
            DolaError::ValueOutOfRange {
                variable, field, ..
            } => fld == "initial" && variable == var && field == fld,
            DolaError::TransitionValueOutOfRange {
                storyboard,
                entry_index: 0,
                variable,
                field,
                ..
            } => storyboard == "sb1" && variable == var && field == fld,
            _ => false,
        })
    }

    #[test]
    fn initial_above_max_error() {
        let doc = doc_with_float_var("x", 1.5, Some(0.0), Some(1.0));
        assert!(out_of_range(&doc, "x", "initial"));
    }

    #[test]
    fn initial_below_min_error() {
        let doc = doc_with_float_var("x", -1.0, Some(0.0), Some(1.0));
        assert!(out_of_range(&doc, "x", "initial"));
    }

    #[test]
    fn transition_to_out_of_range_error() {
        let doc = with_transition(
            doc_with_float_var("x", 0.5, Some(0.0), Some(100.0)),
            "x",
            TransitionDef {
                to: Some(TransitionValue::Scalar(200.0)), // out of range!
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert!(out_of_range(&doc, "x", "to"));
    }

    #[test]
    fn i64_variable_initial_out_of_range() {
        let doc = doc_with_int_var("count", 200, 0, 100);
        assert!(out_of_range(&doc, "count", "initial"));
    }

    // =========================================================
    // V12: from の値域・境界値・Integer 変数のトランジション値域
    // =========================================================

    #[test]
    fn transition_from_out_of_range_error() {
        let doc = with_transition(
            doc_with_float_var("x", 0.5, Some(0.0), Some(10.0)),
            "x",
            TransitionDef {
                from: Some(TransitionValue::Scalar(-5.0)), // min 未満
                to: Some(TransitionValue::Scalar(5.0)),
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert!(out_of_range(&doc, "x", "from"));
    }

    #[test]
    fn values_exactly_at_min_max_are_valid() {
        // 値域検証は排他的比較（< min / > max）— 境界値ちょうどはエラーにならない
        let doc = with_transition(
            doc_with_float_var("x", 1.0, Some(0.0), Some(1.0)), // initial == max
            "x",
            TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)), // == min
                to: Some(TransitionValue::Scalar(1.0)),   // == max
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn i64_variable_initial_below_min_error() {
        let doc = doc_with_int_var("count", -5, 0, 100);
        assert!(out_of_range(&doc, "count", "initial"));
    }

    #[test]
    fn i64_variable_transition_to_out_of_range_error() {
        // Integer 変数のトランジション値域検証（min/max の f64 変換経路）
        let doc = with_transition(
            doc_with_int_var("x", 0, 0, 10),
            "x",
            TransitionDef {
                to: Some(TransitionValue::Scalar(11.0)), // max 超過
                duration: Some(1.0),
                ..Default::default()
            },
        );
        assert!(out_of_range(&doc, "x", "to"));
    }
}
