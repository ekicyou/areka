//! Compile integration tests
//! Task 9.6

use dola::*;

use super::common::make_doc_with_storyboard as make_doc;

#[test]
fn complex_multi_variable_mixed_placement() {
    // 複数変数 + at/between/sequential 混在の複合ストーリーボード
    let doc = make_doc(
        vec![
            (
                "opacity",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: Some(0.0),
                    max: Some(1.0),
                },
            ),
            (
                "x_pos",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
            (
                "y_pos",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
        ],
        vec![],
        "entrance",
        StoryboardBuilder::new()
            .time_scale(1.5)
            .interruption_policy(InterruptionPolicy::Conclude)
            // Entry 0: opacity fade-in sequential 0.0 -> 1.0
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: Some(EasingFunction::Named(EasingName::CubicInOut)),
                    delay: 0.0,
                    duration: Some(0.5),
                })),
                keyframe: Some("visible".to_string()),
                ..Default::default()
            })
            // Entry 1: x_pos at "visible" → starts at 0.5
            .entry(StoryboardEntry {
                variable: Some("x_pos".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    relative_to: None,
                    easing: Some(EasingFunction::Named(EasingName::QuadraticOut)),
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                at: Some(KeyframeRef::Single("visible".to_string())),
                keyframe: Some("moved".to_string()),
                ..Default::default()
            })
            // Entry 2: y_pos between "visible" and "moved" → 0.5..1.5
            .entry(StoryboardEntry {
                variable: Some("y_pos".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(999.0), // ignored for between
                })),
                between: Some(BetweenKeyframes {
                    from: "visible".to_string(),
                    to: "moved".to_string(),
                }),
                keyframe: Some("settled".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "entrance", 0.0).unwrap();

    // Verify metadata
    assert_eq!(result.storyboard_name, "entrance");
    assert_eq!(result.start_time, 0.0);
    assert_eq!(result.time_scale, 1.5);
    assert_eq!(result.interruption_policy, InterruptionPolicy::Conclude);

    // opacity: 0.0 -> 0.5
    let tl_opacity = result.timelines.get("opacity").unwrap();
    assert_eq!(tl_opacity.segments.len(), 1);
    assert_eq!(tl_opacity.segments[0].start_time, 0.0);
    assert_eq!(tl_opacity.segments[0].end_time, 0.5);
    assert_eq!(tl_opacity.min_value, Some(0.0));
    assert_eq!(tl_opacity.max_value, Some(1.0));
    assert_eq!(tl_opacity.variable_type, VariableTypeHint::Float);

    // x_pos: at "visible" (0.5) → 0.5..1.5
    let tl_x = result.timelines.get("x_pos").unwrap();
    assert_eq!(tl_x.segments.len(), 1);
    assert_eq!(tl_x.segments[0].start_time, 0.5);
    assert_eq!(tl_x.segments[0].end_time, 1.5);

    // y_pos: between "visible"(0.5) and "moved"(1.5) → 0.5..1.5
    let tl_y = result.timelines.get("y_pos").unwrap();
    assert_eq!(tl_y.segments.len(), 1);
    assert_eq!(tl_y.segments[0].start_time, 0.5);
    assert_eq!(tl_y.segments[0].end_time, 1.5);

    // total_base_duration = max of all timelines' base_duration
    // opacity: 0.5-0.0=0.5, x_pos: 1.5-0.5=1.0, y_pos: 1.5-0.5=1.0
    assert_eq!(result.total_base_duration, 1.0);
}

#[test]
fn all_variable_types_mixed() {
    // Float + Integer + Object の同一ストーリーボード
    let doc = make_doc(
        vec![
            (
                "opacity",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: Some(0.0),
                    max: Some(1.0),
                },
            ),
            (
                "frame",
                AnimationVariableDef::Integer {
                    initial: 0,
                    min: Some(0),
                    max: Some(10),
                    typewriter: None,
                },
            ),
            (
                "state",
                AnimationVariableDef::Object {
                    initial: DynamicValue::String("idle".to_string()),
                },
            ),
        ],
        vec![],
        "mixed",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: Some(EasingFunction::Named(EasingName::Linear)),
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("frame".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(10.0)),
                    relative_to: None,
                    easing: Some(EasingFunction::Named(EasingName::Linear)),
                    delay: 0.0,
                    duration: Some(2.0),
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
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
                    duration: None,
                })),
                keyframe: Some("kf3".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "mixed", 0.0).unwrap();

    // Check variable type hints
    assert_eq!(
        result.timelines.get("opacity").unwrap().variable_type,
        VariableTypeHint::Float
    );
    assert_eq!(
        result.timelines.get("frame").unwrap().variable_type,
        VariableTypeHint::Integer { typewriter: None }
    );
    assert_eq!(
        result.timelines.get("state").unwrap().variable_type,
        VariableTypeHint::Object
    );

    // Object type: easing should be None
    let state_seg = &result.timelines.get("state").unwrap().segments[0];
    assert_eq!(state_seg.easing, None);
    assert_eq!(
        state_seg.from_value,
        TransitionValue::Dynamic(DynamicValue::String("idle".to_string()))
    );
    assert_eq!(
        state_seg.to_value,
        TransitionValue::Dynamic(DynamicValue::String("active".to_string()))
    );

    // Integer min/max
    let frame_tl = result.timelines.get("frame").unwrap();
    assert_eq!(frame_tl.min_value, Some(0.0));
    assert_eq!(frame_tl.max_value, Some(10.0));
}

#[test]
fn builder_to_compile_flow() {
    // Builder → Compile の完全フロー
    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "alpha",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        )
        .transition(
            "fade_in",
            TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                relative_to: None,
                easing: Some(EasingFunction::Named(EasingName::CubicInOut)),
                delay: 0.0,
                duration: Some(0.5),
            },
        )
        .storyboard(
            "show",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("alpha".to_string()),
                    transition: Some(TransitionRef::Named("fade_in".to_string())),
                    keyframe: Some("visible".to_string()),
                    ..Default::default()
                })
                .build(),
        )
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "show", 0.0).unwrap();
    assert_eq!(compiled.storyboard_name, "show");
    let tl = compiled.timelines.get("alpha").unwrap();
    assert_eq!(tl.segments.len(), 1);
    assert_eq!(tl.segments[0].start_time, 0.0);
    assert_eq!(tl.segments[0].end_time, 0.5);
    assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(0.0));
    assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(1.0));
    assert_eq!(
        tl.segments[0].easing,
        Some(EasingFunction::Named(EasingName::CubicInOut))
    );
}

#[test]
fn pure_keyframe_with_at() {
    // Pure KF + at reference, used as anchor for another entry
    let doc = make_doc(
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
            // x: 0->100 in 2s
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
            // Pure KF: at kf1 + offset 1.0 → time = 3.0
            .entry(StoryboardEntry {
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("kf1".to_string()),
                    offset: 1.0,
                }),
                keyframe: Some("marker".to_string()),
                ..Default::default()
            })
            // y at marker → 3.0
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
                at: Some(KeyframeRef::Single("marker".to_string())),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 3.0);
    assert_eq!(tl_y.segments[0].end_time, 4.0);
}

#[test]
fn multiple_keyframe_wait() {
    // Multiple keyframe reference: waits for all KFs
    let doc = make_doc(
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
            // x: 0..1s → kf_x at 1.0
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
                keyframe: Some("kf_x".to_string()),
                ..Default::default()
            })
            // y: 0..3s → kf_y at 3.0
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
                keyframe: Some("kf_y".to_string()),
                ..Default::default()
            })
            // z: at [kf_x, kf_y] → waits for max(1.0, 3.0) = 3.0
            .entry(StoryboardEntry {
                variable: Some("z".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                at: Some(KeyframeRef::Multiple(vec![
                    "kf_x".to_string(),
                    "kf_y".to_string(),
                ])),
                keyframe: Some("kf_z".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_z = result.timelines.get("z").unwrap();
    assert_eq!(tl_z.segments[0].start_time, 3.0); // max(1.0, 3.0)
    assert_eq!(tl_z.segments[0].end_time, 4.0);
}

#[test]
fn compile_result_serializable() {
    let doc = make_doc(
        vec![(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        )],
        vec![],
        "fade",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: Some(EasingFunction::Named(EasingName::CubicInOut)),
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                keyframe: Some("visible".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let compiled = compile_storyboard(&doc, "fade", 0.0).unwrap();
    let json = serde_json::to_string_pretty(&compiled).unwrap();
    let deserialized: CompiledStoryboard = serde_json::from_str(&json).unwrap();
    assert_eq!(compiled, deserialized);
}

#[test]
fn empty_storyboard_compiles() {
    let doc = make_doc(vec![], vec![], "empty", StoryboardBuilder::new().build());

    let result = compile_storyboard(&doc, "empty", 0.0).unwrap();
    assert_eq!(result.timelines.len(), 0);
    assert_eq!(result.total_base_duration, 0.0);
    assert_eq!(result.storyboard_name, "empty");
}

// =========================================================
// D2-T gap tests: トリガーエントリの時刻解決（compile_storyboard 内ロジック）
// =========================================================

fn float_x_entry(duration: f64, kf: &str) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            duration: Some(duration),
            ..Default::default()
        })),
        keyframe: Some(kf.to_string()),
        ..Default::default()
    }
}

fn simple_child_storyboard() -> Storyboard {
    StoryboardBuilder::new().entry(float_x_entry(1.0, "ckf")).build()
}

#[test]
fn trigger_without_at_inherits_previous_entry_time() {
    // at なしトリガーは配列直前エントリの keyframe 時刻で発火する
    let parent = StoryboardBuilder::new()
        // entry 0: x 0.0..1.0, kf1 = 1.0
        .entry(float_x_entry(1.0, "kf1"))
        // entry 1: trigger（at なし）→ fire_time = 1.0
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .storyboard("parent", parent)
        .storyboard("child", simple_child_storyboard())
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();
    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    assert!(
        (compiled.triggers[0].fire_time - 1.0).abs() < 1e-9,
        "fire_time should inherit previous entry keyframe time 1.0, got {}",
        compiled.triggers[0].fire_time
    );
    assert_eq!(compiled.triggers[0].source_entry_index, 1);
}

#[test]
fn entry_anchored_at_trigger_keyframe() {
    // トリガーエントリの keyframe は fire_time（0秒完了）として登録され、
    // 後続エントリの at 参照のアンカーとして使える
    let parent = StoryboardBuilder::new()
        // entry 0: x 0.0..1.0, kf1 = 1.0
        .entry(float_x_entry(1.0, "kf1"))
        // entry 1: trigger at kf1, keyframe "trig_kf" = 1.0
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: Some(KeyframeRef::Single("kf1".to_string())),
            keyframe: Some("trig_kf".to_string()),
            ..Default::default()
        })
        // entry 2: y at trig_kf → 1.0..2.0
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(50.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            at: Some(KeyframeRef::Single("trig_kf".to_string())),
            keyframe: Some("kf2".to_string()),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .variable(
            "y",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .storyboard("parent", parent)
        .storyboard("child", simple_child_storyboard())
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();
    let tl_y = compiled.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 1.0);
    assert_eq!(tl_y.segments[0].end_time, 2.0);
}
