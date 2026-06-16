//! core_types: Storyboard / Playback serde round-trip
//! Tasks 5.7, 6.3

use super::*;

// =============================================================
// Task 5.7: Storyboard/StoryboardEntry/KeyframeRef serde round-trip
// =============================================================

mod storyboard_tests {
    use super::*;

    #[test]
    fn keyframe_ref_single_json_roundtrip() {
        let kf = KeyframeRef::Single("visible".to_string());
        let json = serde_json::to_string(&kf).unwrap();
        assert_eq!(json, r#""visible""#);
        let deserialized: KeyframeRef = serde_json::from_str(&json).unwrap();
        assert_eq!(kf, deserialized);
    }

    #[test]
    fn keyframe_ref_multiple_json_roundtrip() {
        let kf = KeyframeRef::Multiple(vec!["a".to_string(), "b".to_string()]);
        let json = serde_json::to_string(&kf).unwrap();
        let deserialized: KeyframeRef = serde_json::from_str(&json).unwrap();
        assert_eq!(kf, deserialized);
    }

    #[test]
    fn keyframe_ref_with_offset_single_json_roundtrip() {
        let kf = KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("visible".to_string()),
            offset: 0.5,
        };
        let json = serde_json::to_string(&kf).unwrap();
        let deserialized: KeyframeRef = serde_json::from_str(&json).unwrap();
        assert_eq!(kf, deserialized);
    }

    #[test]
    fn keyframe_ref_with_offset_multiple_json_roundtrip() {
        let kf = KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Multiple(vec!["a".to_string(), "b".to_string()]),
            offset: 1.0,
        };
        let json = serde_json::to_string(&kf).unwrap();
        let deserialized: KeyframeRef = serde_json::from_str(&json).unwrap();
        assert_eq!(kf, deserialized);
    }

    #[test]
    fn storyboard_entry_chain_pattern_json_roundtrip() {
        // 前エントリ連結: variable + transition のみ
        let entry = StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Named("fade_in".to_string())),
            keyframe: Some("visible".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn storyboard_entry_at_pattern_json_roundtrip() {
        // KF起点
        let entry = StoryboardEntry {
            variable: Some("char_count".to_string()),
            transition: Some(TransitionRef::Named("typewrite".to_string())),
            at: Some(KeyframeRef::Single("visible".to_string())),
            keyframe: Some("text_done".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn storyboard_entry_between_pattern_json_roundtrip() {
        // KF間
        let entry = StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: None,
                to: Some(TransitionValue::Scalar(0.0)),
                relative_to: None,
                easing: Some(EasingFunction::Named(EasingName::Linear)),
                delay: 0.0,
                duration: None,
            })),
            between: Some(BetweenKeyframes {
                from: "visible".to_string(),
                to: "text_done".to_string(),
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn storyboard_entry_pure_keyframe_json_roundtrip() {
        // 純粋KF
        let entry = StoryboardEntry {
            keyframe: Some("sync_point".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn storyboard_default_values_json() {
        // time_scale=1.0, interruption_policy=Conclude がデフォルト
        let json = r#"{"entry":[]}"#;
        let sb: Storyboard = serde_json::from_str(json).unwrap();
        assert_eq!(sb.time_scale, 1.0);
        assert_eq!(sb.loop_count, 1);
        assert_eq!(sb.interruption_policy, dola::InterruptionPolicy::Conclude);
        assert!(sb.entry.is_empty());
    }

    #[test]
    fn interruption_policy_all_variants_json_roundtrip() {
        use dola::InterruptionPolicy;
        let variants = vec![
            (InterruptionPolicy::Cancel, "\"cancel\""),
            (InterruptionPolicy::Conclude, "\"conclude\""),
            (InterruptionPolicy::Trim, "\"trim\""),
            (InterruptionPolicy::Compress, "\"compress\""),
            (InterruptionPolicy::Never, "\"never\""),
        ];
        for (variant, expected) in &variants {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(&json, expected, "Failed for {:?}", variant);
            let deserialized: InterruptionPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, deserialized);
        }
    }
}

// =============================================================
// Task 6.3: PlaybackState/ScheduleRequest serde round-trip
// =============================================================

mod playback_tests {
    use super::*;

    #[test]
    fn playback_state_all_variants_json_roundtrip() {
        let variants = vec![
            PlaybackState::Idle,
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Completed,
            PlaybackState::Cancelled,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: PlaybackState = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn schedule_request_json_roundtrip() {
        let req = ScheduleRequest {
            storyboard: "greeting".to_string(),
            start_time: 1.5,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ScheduleRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }
}
