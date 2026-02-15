//! Compile module unit tests
//! Task 9.1 - 9.5

use dola::*;
use std::collections::BTreeMap;

// ============================================================
// Helper: Building test documents
// ============================================================

fn make_doc_with_storyboard(
    variables: Vec<(&str, AnimationVariableDef)>,
    transitions: Vec<(&str, TransitionDef)>,
    storyboard_name: &str,
    sb: Storyboard,
) -> DolaDocument {
    let mut variable = BTreeMap::new();
    for (name, def) in variables {
        variable.insert(name.to_string(), def);
    }
    let mut transition = BTreeMap::new();
    for (name, def) in transitions {
        transition.insert(name.to_string(), def);
    }
    let mut storyboard = BTreeMap::new();
    storyboard.insert(storyboard_name.to_string(), sb);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition,
        storyboard,
    }
}

// ============================================================
// Task 9.1: Serialize/Deserialize tests
// ============================================================

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn compiled_storyboard_json_roundtrip() {
        let mut timelines = BTreeMap::new();
        timelines.insert(
            "opacity".to_string(),
            CompiledVariableTimeline {
                variable_type: VariableTypeHint::Float,
                segments: vec![CompiledSegment {
                    start_time: 0.0,
                    end_time: 1.0,
                    from_value: TransitionValue::Scalar(0.0),
                    to_value: TransitionValue::Scalar(1.0),
                    easing: Some(EasingFunction::Named(EasingName::CubicInOut)),
                }],
                base_duration: 1.0,
                min_value: Some(0.0),
                max_value: Some(1.0),
            },
        );

        let compiled = CompiledStoryboard {
            storyboard_name: "fade_in".to_string(),
            start_time: 0.0,
            timelines,
            time_scale: 1.0,
            loop_count: 1,
            interruption_policy: InterruptionPolicy::Conclude,
            total_base_duration: 1.0,
        };

        let json = serde_json::to_string_pretty(&compiled).unwrap();
        let deserialized: CompiledStoryboard = serde_json::from_str(&json).unwrap();
        assert_eq!(compiled, deserialized);
    }

    #[test]
    fn variable_type_hint_serde_tagged() {
        // Float
        let json = serde_json::to_string(&VariableTypeHint::Float).unwrap();
        assert!(json.contains("\"type\":\"float\""));
        let rt: VariableTypeHint = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, VariableTypeHint::Float);

        // Integer with typewriter
        let json = serde_json::to_string(&VariableTypeHint::Integer {
            typewriter: Some("Hello".to_string()),
        })
        .unwrap();
        assert!(json.contains("\"type\":\"integer\""));
        assert!(json.contains("\"typewriter\":\"Hello\""));

        // Integer without typewriter
        let json = serde_json::to_string(&VariableTypeHint::Integer { typewriter: None }).unwrap();
        assert!(json.contains("\"type\":\"integer\""));
        assert!(!json.contains("typewriter"));

        // Object
        let json = serde_json::to_string(&VariableTypeHint::Object).unwrap();
        assert!(json.contains("\"type\":\"object\""));
    }

    #[test]
    fn compiled_segment_instant_transition_serde() {
        let seg = CompiledSegment {
            start_time: 2.0,
            end_time: 2.0, // instant
            from_value: TransitionValue::Scalar(0.0),
            to_value: TransitionValue::Scalar(100.0),
            easing: None,
        };
        let json = serde_json::to_string(&seg).unwrap();
        let rt: CompiledSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(seg, rt);
        assert!(!json.contains("easing")); // skip_serializing_if
    }
}

// ============================================================
// Task 9.2: Time resolution unit tests
// ============================================================

#[cfg(test)]
mod time_resolution_tests {
    use super::*;

    #[test]
    fn simple_sequential_single_variable() {
        // 1 variable, 2 sequential entries
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None, // infer from previous
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments.len(), 2);

        // First segment: 0.0 -> 1.0
        assert_eq!(tl.segments[0].start_time, 0.0);
        assert_eq!(tl.segments[0].end_time, 1.0);
        assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(0.0));
        assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(100.0));

        // Second segment: 1.0 -> 3.0 (sequential, from = previous to)
        assert_eq!(tl.segments[1].start_time, 1.0);
        assert_eq!(tl.segments[1].end_time, 3.0);
        assert_eq!(tl.segments[1].from_value, TransitionValue::Scalar(100.0));
        assert_eq!(tl.segments[1].to_value, TransitionValue::Scalar(200.0));
    }

    #[test]
    fn sequential_with_delay() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.5,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 10.0).unwrap();
        let tl = result.timelines.get("x").unwrap();

        // start_time = 10.0, delay = 0.5 → segment 10.5 -> 11.5
        assert_eq!(tl.segments[0].start_time, 10.5);
        assert_eq!(tl.segments[0].end_time, 11.5);
    }

    #[test]
    fn at_reference_keyframe() {
        let doc = make_doc_with_storyboard(
            vec![
                ("x", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
                ("y", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                // Entry 0: x, sequential, keyframe "kf1" ends at 2.0
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                // Entry 1: y, at "kf1", starts at 2.0
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("kf1".to_string())),
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl_y = result.timelines.get("y").unwrap();
        assert_eq!(tl_y.segments[0].start_time, 2.0);
        assert_eq!(tl_y.segments[0].end_time, 3.0);
    }

    #[test]
    fn at_with_offset() {
        let doc = make_doc_with_storyboard(
            vec![
                ("x", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
                ("y", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("kf1".to_string()),
                        offset: 0.5,
                    }),
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl_y = result.timelines.get("y").unwrap();
        // kf1 ends at 2.0, offset 0.5 → base_time = 2.5, segment 2.5 -> 3.5
        assert_eq!(tl_y.segments[0].start_time, 2.5);
        assert_eq!(tl_y.segments[0].end_time, 3.5);
    }

    #[test]
    fn between_placement() {
        let doc = make_doc_with_storyboard(
            vec![
                ("x", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
                ("y", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(100.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(3.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                // y between kf1 and kf2: from_t=2.0, to_t=5.0
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(999.0), // ignored for between
                    })),
                    at: None,
                    between: Some(BetweenKeyframes {
                        from: "kf1".to_string(),
                        to: "kf2".to_string(),
                    }),
                    keyframe: Some("kf3".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl_y = result.timelines.get("y").unwrap();
        assert_eq!(tl_y.segments[0].start_time, 2.0); // from_kf time
        assert_eq!(tl_y.segments[0].end_time, 5.0); // to_kf time
    }

    #[test]
    fn duration_zero_instant_transition() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None, // instant
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, tl.segments[0].end_time);
    }

    #[test]
    fn start_time_offset() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 5.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, 5.0);
        assert_eq!(tl.segments[0].end_time, 6.0);
        assert_eq!(result.start_time, 5.0);
    }

    #[test]
    fn sequential_first_entry_uses_compile_start_time() {
        // First entry for a variable: base_time = compile start_time
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 10.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None, // infer from initial = 10.0
                        to: Some(TransitionValue::Scalar(20.0)),
                        relative_to: None,
                        easing: None,
                        delay: 1.0,
                        duration: Some(2.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 3.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        // base_time = 3.0 (compile start_time), segment_start = 3.0 + 1.0 = 4.0
        assert_eq!(tl.segments[0].start_time, 4.0);
        assert_eq!(tl.segments[0].end_time, 6.0);
        assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(10.0));
    }
}

// ============================================================
// Task 9.3: Transition resolution unit tests
// ============================================================

#[cfg(test)]
mod transition_resolution_tests {
    use super::*;

    #[test]
    fn named_transition_resolved() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![("fade", TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                relative_to: None,
                easing: Some(EasingFunction::Named(EasingName::CubicInOut)),
                delay: 0.0,
                duration: Some(0.5),
            })],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Named("fade".to_string())),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(0.0));
        assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(1.0));
        assert_eq!(
            tl.segments[0].easing,
            Some(EasingFunction::Named(EasingName::CubicInOut))
        );
        assert_eq!(tl.segments[0].end_time, 0.5);
    }

    #[test]
    fn from_inferred_from_previous_segment() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None, // should be inferred as 50.0
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[1].from_value, TransitionValue::Scalar(50.0));
    }

    #[test]
    fn from_inferred_from_initial_value() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 42.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None, // inferred as initial = 42.0
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(42.0));
    }

    #[test]
    fn relative_to_calculation() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 10.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(10.0)),
                        to: None,
                        relative_to: Some(25.0), // to = from + 25 = 35
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(35.0));
    }

    #[test]
    fn object_type_instant_switch() {
        let doc = make_doc_with_storyboard(
            vec![("state", AnimationVariableDef::Object {
                initial: DynamicValue::String("idle".to_string()),
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("state".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Dynamic(DynamicValue::String(
                            "active".to_string(),
                        ))),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None, // instant
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("state").unwrap();
        // Object type: easing should be None
        assert_eq!(tl.segments[0].easing, None);
        assert_eq!(tl.variable_type, VariableTypeHint::Object);
        // from = initial (Dynamic("idle")), to = Dynamic("active")
        assert_eq!(
            tl.segments[0].from_value,
            TransitionValue::Dynamic(DynamicValue::String("idle".to_string()))
        );
        assert_eq!(
            tl.segments[0].to_value,
            TransitionValue::Dynamic(DynamicValue::String("active".to_string()))
        );
    }

    #[test]
    fn easing_function_preserved() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: Some(EasingFunction::Named(EasingName::BounceOut)),
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(
            tl.segments[0].easing,
            Some(EasingFunction::Named(EasingName::BounceOut))
        );
    }
}

// ============================================================
// Task 9.4: Metadata and hints unit tests
// ============================================================

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn time_scale_propagated_not_applied() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .time_scale(2.0)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(result.time_scale, 2.0);
        // Segment times should NOT be affected by time_scale
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, 0.0);
        assert_eq!(tl.segments[0].end_time, 1.0);
    }

    #[test]
    fn loop_count_propagated() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .loop_count(3)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(result.loop_count, 3);
    }

    #[test]
    fn interruption_policy_propagated() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .interruption_policy(InterruptionPolicy::Cancel)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(result.interruption_policy, InterruptionPolicy::Cancel);
    }

    #[test]
    fn variable_type_hint_float() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(
            result.timelines.get("x").unwrap().variable_type,
            VariableTypeHint::Float
        );
    }

    #[test]
    fn variable_type_hint_integer_with_typewriter() {
        let doc = make_doc_with_storyboard(
            vec![("idx", AnimationVariableDef::Integer {
                initial: 0,
                min: Some(0),
                max: Some(5),
                typewriter: Some("Hello".to_string()),
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("idx".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(5.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("idx").unwrap();
        assert_eq!(
            tl.variable_type,
            VariableTypeHint::Integer {
                typewriter: Some("Hello".to_string()),
            }
        );
    }

    #[test]
    fn base_duration_and_total_base_duration() {
        let doc = make_doc_with_storyboard(
            vec![
                ("x", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
                ("y", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                // x: 0.0 -> 1.0 (duration 1.0)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                // y: 0.0 -> 3.0 (duration 3.0)
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(3.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(result.timelines.get("x").unwrap().base_duration, 1.0);
        assert_eq!(result.timelines.get("y").unwrap().base_duration, 3.0);
        assert_eq!(result.total_base_duration, 3.0); // max of all
    }

    #[test]
    fn min_max_propagated() {
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.5,
                min: Some(0.0),
                max: Some(1.0),
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.min_value, Some(0.0));
        assert_eq!(tl.max_value, Some(1.0));
    }

    #[test]
    fn integer_min_max_converted_to_f64() {
        let doc = make_doc_with_storyboard(
            vec![("idx", AnimationVariableDef::Integer {
                initial: 0,
                min: Some(-10),
                max: Some(100),
                typewriter: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("idx".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("idx").unwrap();
        assert_eq!(tl.min_value, Some(-10.0));
        assert_eq!(tl.max_value, Some(100.0));
    }
}

// ============================================================
// Task 9.5: Error handling tests
// ============================================================

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn validate_failure_passthrough() {
        // Bad schema version should fail validation
        let doc = DolaDocument {
            schema_version: "0.1".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, DolaError::SchemaVersionMismatch { .. })));
    }

    #[test]
    fn undefined_storyboard_error() {
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };

        let result = compile_storyboard(&doc, "nonexistent", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("not found"))));
    }

    #[test]
    fn keyframe_cycle_detection() {
        // Create a cycle: entry0 (at kf1) -> entry1 (at kf0)
        let doc = make_doc_with_storyboard(
            vec![
                ("x", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
                ("y", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("kf1".to_string())),
                    between: None,
                    keyframe: Some("kf0".to_string()),
                })
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("kf0".to_string())),
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, DolaError::KeyframeCycle { .. })));
    }

    #[test]
    fn between_delay_exceeds_interval() {
        let doc = make_doc_with_storyboard(
            vec![
                ("x", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
                ("y", AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                }),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(100.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: None,
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                // between kf1 (1.0) and kf2 (2.0) with delay 2.0 → exceeds interval
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 2.0, // exceeds interval (kf2-kf1 = 1.0)
                        duration: None,
                    })),
                    at: None,
                    between: Some(BetweenKeyframes {
                        from: "kf1".to_string(),
                        to: "kf2".to_string(),
                    }),
                    keyframe: Some("kf3".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("delay") || reason.contains("exceeds"))
        ));
    }

    #[test]
    fn segment_overlap_detected() {
        // Create overlapping segments for the same variable using 'at' to force overlap
        let doc = make_doc_with_storyboard(
            vec![("x", AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            })],
            vec![],
            "test",
            StoryboardBuilder::new()
                // Entry 0: pure KF at start - sets kf_start at time 0.0
                .entry(StoryboardEntry {
                    variable: None,
                    transition: None,
                    at: Some(KeyframeRef::Single("start".to_string())),
                    between: None,
                    keyframe: Some("kf_start".to_string()),
                })
                // Entry 1: x from 0->100, at start, duration 2.0 → 0.0..2.0
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    between: None,
                    keyframe: Some("kf1".to_string()),
                })
                // Entry 2: x from 50->200, at start, duration 3.0 → 0.0..3.0 (OVERLAP!)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(50.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(3.0),
                    })),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    between: None,
                    keyframe: Some("kf2".to_string()),
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("overlap") || reason.contains("Overlap"))
        ));
    }
}
