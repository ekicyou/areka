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

    // =========================================================
    // D2-T gap tests: 純粋KF（at なし）の時刻解決
    // =========================================================

    #[test]
    fn pure_keyframe_without_at_inherits_previous_entry_time() {
        // 純粋KF（at なし）は配列直前エントリの keyframe 時刻を継承する
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
                // entry 0: x 0.0..2.0, kf1 = 2.0
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        duration: Some(2.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                // entry 1: 純粋KF（at なし）→ 直前エントリの keyframe 時刻 2.0 を継承
                .entry(StoryboardEntry {
                    keyframe: Some("marker".to_string()),
                    ..Default::default()
                })
                // entry 2: y at marker → 2.0..3.0
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::Single("marker".to_string())),
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
    fn pure_keyframe_without_at_as_first_entry_falls_back_to_start() {
        // 先頭エントリの純粋KF（at なし）は "start"（= compile start_time）へフォールバック
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
                // entry 0: 純粋KF（at なし）が先頭 → start_time を継承
                .entry(StoryboardEntry {
                    keyframe: Some("marker".to_string()),
                    ..Default::default()
                })
                // entry 1: x at marker
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::Single("marker".to_string())),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 5.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, 5.0);
        assert_eq!(tl.segments[0].end_time, 6.0);
    }

    // =========================================================
    // D2-T gap tests: KeyframeRef の未テストバリアント
    // =========================================================

    #[test]
    fn at_with_offset_multiple_keyframes() {
        // WithOffset + Multiple: 全KFの最大時刻 + offset
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
                (
                    "z",
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
                // x: 0..1s → kf_x = 1.0
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf_x".to_string()),
                    ..Default::default()
                })
                // y: 0..3s → kf_y = 3.0
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        duration: Some(3.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf_y".to_string()),
                    ..Default::default()
                })
                // z: at [kf_x, kf_y] + 0.5 → max(1.0, 3.0) + 0.5 = 3.5
                .entry(StoryboardEntry {
                    variable: Some("z".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Multiple(vec![
                            "kf_x".to_string(),
                            "kf_y".to_string(),
                        ]),
                        offset: 0.5,
                    }),
                    keyframe: Some("kf_z".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl_z = result.timelines.get("z").unwrap();
        assert_eq!(tl_z.segments[0].start_time, 3.5);
        assert_eq!(tl_z.segments[0].end_time, 4.5);
    }

    #[test]
    fn at_with_negative_offset() {
        // 負オフセット: base_time + offset で基準より前に配置できる
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
                        duration: Some(2.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                // kf1 = 2.0, offset -0.5 → segment 1.5..2.5
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("kf1".to_string()),
                        offset: -0.5,
                    }),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl_y = result.timelines.get("y").unwrap();
        assert_eq!(tl_y.segments[0].start_time, 1.5);
        assert_eq!(tl_y.segments[0].end_time, 2.5);
    }

    // =========================================================
    // D2-T gap tests: between の未テスト経路
    // =========================================================

    #[test]
    fn between_from_start_pseudo_keyframe() {
        // between の from に疑似KF "start" を使用できる
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
                        duration: Some(2.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                // y between start(0.0) → kf1(2.0)
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        ..Default::default()
                    })),
                    between: Some(BetweenKeyframes {
                        from: "start".to_string(),
                        to: "kf1".to_string(),
                    }),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl_y = result.timelines.get("y").unwrap();
        assert_eq!(tl_y.segments[0].start_time, 0.0);
        assert_eq!(tl_y.segments[0].end_time, 2.0);
    }

    #[test]
    fn between_with_valid_delay() {
        // 区間内に収まる delay: segment_start = from_t + delay
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
                        duration: Some(2.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(100.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        duration: Some(3.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                // y between kf1(2.0) → kf2(5.0)、delay 1.0 → 3.0..5.0
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        delay: 1.0,
                        ..Default::default()
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
        assert_eq!(tl_y.segments[0].start_time, 3.0);
        assert_eq!(tl_y.segments[0].end_time, 5.0);
    }

    // =========================================================
    // D2-T gap tests: sequential と at の相互作用・ソート
    // =========================================================

    #[test]
    fn sequential_after_at_placement_continues_from_segment_end() {
        // at 配置されたセグメントの end_time が同一変数の後続 sequential の基準になる
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
                // entry 0: at start+5.0 → 5.0..6.0
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("start".to_string()),
                        offset: 5.0,
                    }),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                // entry 1: sequential → 6.0..7.0（var_last_end_time から継続）
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(200.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments.len(), 2);
        assert_eq!(tl.segments[1].start_time, 6.0);
        assert_eq!(tl.segments[1].end_time, 7.0);
        assert_eq!(tl.segments[1].from_value, TransitionValue::Scalar(100.0));
    }

    #[test]
    fn out_of_order_at_entries_sorted_by_start_time() {
        // エントリ記述順と逆の時刻でも、コンパイル結果のセグメントは start_time 昇順
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
                // entry 0: at start+2.0 → 2.0..3.0（後の時刻を先に記述）
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(100.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("start".to_string()),
                        offset: 2.0,
                    }),
                    keyframe: Some("kf_late".to_string()),
                    ..Default::default()
                })
                // entry 1: at start → 0.0..1.0
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    keyframe: Some("kf_early".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments.len(), 2);
        assert_eq!(tl.segments[0].start_time, 0.0);
        assert_eq!(tl.segments[0].end_time, 1.0);
        assert_eq!(tl.segments[1].start_time, 2.0);
        assert_eq!(tl.segments[1].end_time, 3.0);
    }
}
