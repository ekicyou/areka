//! Compile module time resolution tests
//! Task 9.2

use dola::*;

use super::common::make_doc_with_storyboard;

#[cfg(test)]
mod time_resolution_tests {
    use super::*;

    #[test]
    fn simple_sequential_single_variable() {
        // 1 variable, 2 sequential entries
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
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.5,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
                    between: Some(BetweenKeyframes {
                        from: "kf1".to_string(),
                        to: "kf2".to_string(),
                    }),
                    keyframe: Some("kf3".to_string()),
                    ..Default::default()
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
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None, // instant
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 10.0,
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
                        from: None, // infer from initial = 10.0
                        to: Some(TransitionValue::Scalar(20.0)),
                        relative_to: None,
                        easing: None,
                        delay: 1.0,
                        duration: Some(2.0),
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
