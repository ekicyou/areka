//! Compile module metadata and hints tests
//! Task 9.4

use dola::*;

use super::common::make_doc_with_storyboard;

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn time_scale_propagated_not_applied() {
        let doc = make_doc_with_storyboard(
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(result.loop_count, 3);
    }

    #[test]
    fn interruption_policy_propagated() {
        let doc = make_doc_with_storyboard(
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        assert_eq!(result.interruption_policy, InterruptionPolicy::Cancel);
    }

    #[test]
    fn variable_type_hint_float() {
        let doc = make_doc_with_storyboard(
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
            vec![(
                "idx",
                AnimationVariableDef::Integer {
                    initial: 0,
                    min: Some(0),
                    max: Some(5),
                    typewriter: Some("Hello".to_string()),
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                (
                    "x",
                    AnimationVariableDef::Float {
                        initial: 0.0,
                        min: None,
                        max: None,
                    },
                ),
                (
                    "y",
                    AnimationVariableDef::Float {
                        initial: 0.0,
                        min: None,
                        max: None,
                    },
                ),
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.5,
                    min: Some(0.0),
                    max: Some(1.0),
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
            vec![(
                "idx",
                AnimationVariableDef::Integer {
                    initial: 0,
                    min: Some(-10),
                    max: Some(100),
                    typewriter: None,
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("idx").unwrap();
        assert_eq!(tl.min_value, Some(-10.0));
        assert_eq!(tl.max_value, Some(100.0));
    }
}
