//! Multi-variable, mixed-type, builder-flow, and serialization integration tests.

use super::*;

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
