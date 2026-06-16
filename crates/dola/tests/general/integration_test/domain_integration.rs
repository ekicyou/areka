//! Domain integration edge-case tests
//! Task 12.2

// =============================================================
// Task 12.2: エッジケーステスト
// =============================================================

mod edge_case_tests {
    use super::super::*;

    #[test]
    fn empty_storyboard_ok() {
        let doc = DolaDocumentBuilder::new("1.0")
            .storyboard("empty", StoryboardBuilder::new().build())
            .build()
            .unwrap();
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: DolaDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn pure_keyframe_only_storyboard() {
        let doc = DolaDocumentBuilder::new("1.0")
            .storyboard(
                "kf_only",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        keyframe: Some("marker".to_string()),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn typewriter_variable_with_transition() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "chars",
                AnimationVariableDef::Integer {
                    initial: 0,
                    min: Some(0),
                    max: None,
                    typewriter: Some("こんにちは".to_string()),
                },
            )
            .storyboard(
                "type_sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("chars".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(5.0)),
                            relative_to: None,
                            easing: Some(EasingFunction::Named(EasingName::Linear)),
                            delay: 0.0,
                            duration: Some(3.0),
                        })),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn bezier_easing_inline_transition() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "bezier_sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(100.0)),
                            relative_to: None,
                            easing: Some(EasingFunction::Parametric(
                                ParametricEasing::CubicBezier {
                                    x0: 0.0,
                                    x1: 0.42,
                                    x2: 0.58,
                                    x3: 1.0,
                                },
                            )),
                            delay: 0.0,
                            duration: Some(2.0),
                        })),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();

        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("cubic_bezier"));
        let deserialized: DolaDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn delay_only_instant_transition() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "delay_sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(1.0)),
                            relative_to: None,
                            easing: None,
                            delay: 2.0,
                            duration: None, // instant transition after delay
                        })),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn at_start_keyword() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "start_sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(1.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        at: Some(KeyframeRef::Single("start".to_string())),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn multiple_keyframe_wait() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "multi_kf",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(1.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        keyframe: Some("a".to_string()),
                        ..Default::default()
                    })
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(2.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        keyframe: Some("b".to_string()),
                        ..Default::default()
                    })
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(3.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        at: Some(KeyframeRef::Multiple(vec![
                            "a".to_string(),
                            "b".to_string(),
                        ])),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn keyframe_ref_with_offset() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "offset_sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
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
                    })
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(2.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        at: Some(KeyframeRef::WithOffset {
                            keyframes: KeyframeNames::Single("visible".to_string()),
                            offset: 0.5,
                        }),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn object_transition_dynamic_value() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "bg",
                AnimationVariableDef::Object {
                    initial: DynamicValue::Map({
                        let mut m = BTreeMap::new();
                        m.insert(
                            "path".to_string(),
                            DynamicValue::String("default.png".to_string()),
                        );
                        m
                    }),
                },
            )
            .storyboard(
                "obj_sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("bg".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Dynamic(DynamicValue::Map({
                                let mut m = BTreeMap::new();
                                m.insert(
                                    "path".to_string(),
                                    DynamicValue::String("image.png".to_string()),
                                );
                                m
                            }))),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: None,
                        })),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn value_out_of_range_v12_error() {
        let result = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 1.5,
                    min: Some(0.0),
                    max: Some(1.0),
                },
            )
            .build();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::ValueOutOfRange { .. }))
        );
    }

    #[test]
    fn type_mismatch_v13_error() {
        let result = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "sb",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
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
                    })
                    .build(),
            )
            .build();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TypeMismatch { .. }))
        );
    }
}
