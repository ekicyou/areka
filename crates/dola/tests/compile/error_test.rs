//! Compile module error handling tests
//! Task 9.5

use dola::*;
use std::collections::BTreeMap;

use super::common::make_doc_with_storyboard;

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
        assert!(
            errs.iter()
                .any(|e| matches!(e, DolaError::SchemaVersionMismatch { .. }))
        );
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
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("not found"))
        ));
    }

    #[test]
    fn keyframe_cycle_detection() {
        // Create a cycle: entry0 (at kf1) -> entry1 (at kf0)
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
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("kf1".to_string())),
                    keyframe: Some("kf0".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, DolaError::KeyframeCycle { .. }))
        );
    }

    #[test]
    fn between_delay_exceeds_interval() {
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
                        duration: Some(1.0),
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
                        duration: Some(1.0),
                    })),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
                    between: Some(BetweenKeyframes {
                        from: "kf1".to_string(),
                        to: "kf2".to_string(),
                    }),
                    keyframe: Some("kf3".to_string()),
                    ..Default::default()
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
                // Entry 0: pure KF at start - sets kf_start at time 0.0
                .entry(StoryboardEntry {
                    at: Some(KeyframeRef::Single("start".to_string())),
                    keyframe: Some("kf_start".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
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
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
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
