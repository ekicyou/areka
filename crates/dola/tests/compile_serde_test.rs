//! Compile module serde tests
//! Task 9.1

use dola::*;
use std::collections::BTreeMap;

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
            loop_offset: None,
            total_base_duration: 1.0,
            triggers: Vec::new(),
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
