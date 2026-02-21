//! Compile module transition resolution tests
//! Task 9.3

use dola::*;

mod compile_common;
use compile_common::make_doc_with_storyboard;

#[cfg(test)]
mod transition_resolution_tests {
    use super::*;

    #[test]
    fn named_transition_resolved() {
        let doc = make_doc_with_storyboard(
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )],
            vec![(
                "fade",
                TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: Some(EasingFunction::Named(EasingName::CubicInOut)),
                    delay: 0.0,
                    duration: Some(0.5),
                },
            )],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Named("fade".to_string())),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                        to: Some(TransitionValue::Scalar(50.0)),
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
                        from: None, // should be inferred as 50.0
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 42.0,
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
                        from: None, // inferred as initial = 42.0
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
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(42.0));
    }

    #[test]
    fn relative_to_calculation() {
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
                        from: Some(TransitionValue::Scalar(10.0)),
                        to: None,
                        relative_to: Some(25.0), // to = from + 25 = 35
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
        assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(35.0));
    }

    #[test]
    fn object_type_instant_switch() {
        let doc = make_doc_with_storyboard(
            vec![(
                "state",
                AnimationVariableDef::Object {
                    initial: DynamicValue::String("idle".to_string()),
                },
            )],
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                        easing: Some(EasingFunction::Named(EasingName::BounceOut)),
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
        assert_eq!(
            tl.segments[0].easing,
            Some(EasingFunction::Named(EasingName::BounceOut))
        );
    }
}
