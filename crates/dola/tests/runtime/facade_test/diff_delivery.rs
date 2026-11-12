//! 差分配信（research.md §R3: diff_delivery シーム）
//!
//! 指示書差し替えによる値の引き継ぎと、購読 API（unsubscribe /
//! unsubscribe_all / Default）経由の差分配信を束ねる。

// =========================================================================
// 8.3 指示書差し替えテスト
// =========================================================================
mod document_replacement {
    use super::super::*;

    /// 差し替えは配信の記録を消さない: 2 枚目を読んだ後、start せずに tick しても差分 0（#43）
    #[test]
    fn same_variable_carries_over_value() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");

        rt.load_document(simple_float_doc("fade_in")).unwrap();
        rt.start("fade_in", 0.0).unwrap();
        rt.tick(0.5);
        assert!(!rt.last_result().changes.is_empty());
        // 自然終了で最終値 1.0 が配信される
        rt.tick(1.5);

        rt.load_document(simple_float_doc("fade_in")).unwrap();
        rt.tick(2.0);
        assert!(
            rt.last_result().changes.is_empty(),
            "reload must keep delivered values: {:?}",
            rt.last_result().changes
        );
    }

    /// 差し替えの後も動いている Object の値は同じ Rc のまま: 偽の差分を出さない（#62）
    #[test]
    fn reload_keeps_animating_object_value_without_spurious_diff() {
        use dola::DynamicValue;
        let mut doc = simple_float_doc("swap");
        doc.variable.insert(
            "o".to_string(),
            AnimationVariableDef::Object {
                initial: DynamicValue::String("a".to_string()),
            },
        );
        doc.storyboard.get_mut("swap").unwrap().entry[0] = StoryboardEntry {
            variable: Some("o".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Dynamic(DynamicValue::String(
                    "b".to_string(),
                ))),
                duration: Some(1.0),
                ..Default::default()
            })),
            ..Default::default()
        };

        let mut rt = DolaRuntime::new();
        rt.subscribe("o");
        rt.load_document(doc.clone()).unwrap();
        rt.start("swap", 0.0).unwrap();
        rt.tick(0.25);
        rt.tick(0.5);
        assert!(rt.last_result().changes.is_empty());

        rt.load_document(doc).unwrap();
        rt.tick(0.75);
        assert!(
            rt.last_result().changes.is_empty(),
            "{:?}",
            rt.last_result().changes
        );
    }

    /// 検証で拒んだ文書は前の文書を消さない（#43）
    #[test]
    fn invalid_document_preserves_existing() {
        let mut rt = DolaRuntime::new();
        rt.load_document(simple_float_doc("fade_in")).unwrap();

        let mut bad_doc = simple_float_doc("fade_in");
        bad_doc.schema_version = "99.0".to_string();
        assert!(rt.load_document(bad_doc).is_err());

        rt.start("fade_in", 0.0).unwrap();
    }
}

// =========================================================================
// 購読 API のファサード経由検証（unsubscribe / unsubscribe_all / Default）
// =========================================================================
mod subscription_via_facade {
    use super::super::*;

    #[test]
    fn unsubscribe_stops_diff_delivery() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        rt.load_document(simple_float_doc("fade_in")).unwrap();
        rt.start("fade_in", 0.0).unwrap();

        // 購読中は差分が配信される
        let diff = rt.update(0.0).changes;
        assert!(diff.iter().any(|(id, _)| *id == opacity_id));

        // 購読解除後は配信されない
        rt.unsubscribe(opacity_id).unwrap();
        let diff = rt.update(0.5).changes;
        assert!(
            diff.is_empty(),
            "expected no delivery after unsubscribe, got {diff:?}"
        );
    }

    #[test]
    fn unsubscribe_all_stops_diff_delivery() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(simple_float_doc("fade_in")).unwrap();
        rt.start("fade_in", 0.0).unwrap();

        let diff = rt.update(0.0).changes;
        assert!(!diff.is_empty());

        rt.unsubscribe_all();
        let diff = rt.update(0.5).changes;
        assert!(
            diff.is_empty(),
            "expected no delivery after unsubscribe_all, got {diff:?}"
        );
    }

    #[test]
    fn unsubscribe_invalid_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.unsubscribe(9999).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidVariableId(9999)));
    }

    #[test]
    fn default_is_equivalent_to_new() {
        let mut rt = DolaRuntime::default();
        // ドキュメント未読込 → StoryboardNotFound
        let err = rt.start("any", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::StoryboardNotFound(_)));
        // 採番は 0-origin（new() と同一の初期状態）
        assert_eq!(rt.subscribe("x"), 0);
        // tick() 未呼び出し時の last_result は空
        assert!(rt.last_result().changes.is_empty());
        assert!(rt.last_result().triggered.is_empty());
    }
}

// =========================================================================
// tick が last_result を書き換える
// =========================================================================
mod tick_last_result {
    use super::super::*;

    #[test]
    fn tick_updates_last_result() {
        let mut rt = DolaRuntime::new();
        rt.subscribe("opacity");
        rt.load_document(simple_float_doc("fade")).unwrap();
        rt.start("fade", 0.0).unwrap();

        rt.tick(0.0);
        let r0 = rt.last_result().clone();

        rt.tick(0.5);
        let r1 = rt.last_result().clone();

        assert!(!r0.changes.is_empty());
        assert!(!r1.changes.is_empty());

        let val0 = match &r0.changes[0].1 {
            EvaluatedValue::Float(v) => *v,
            _ => panic!("expected Float"),
        };
        let val1 = match &r1.changes[0].1 {
            EvaluatedValue::Float(v) => *v,
            _ => panic!("expected Float"),
        };
        assert!(
            (val1 - val0).abs() > 0.1,
            "values should differ: {val0} vs {val1}"
        );
    }
}
