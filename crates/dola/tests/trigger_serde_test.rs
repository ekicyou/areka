//! トリガー機能のテスト — 5.1: serde 往復テスト

use dola::{CompiledTrigger, StoryboardEntry};

// ============================================================
// 5.1: serde 往復テスト
// ============================================================

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn trigger_entry_minimal_json_roundtrip() {
        let entry = StoryboardEntry {
            trigger_storyboard: Some("child_sb".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("trigger_storyboard"));
        assert!(json.contains("child_sb"));
        // trigger_start_offset は None なので含まれない
        assert!(!json.contains("trigger_start_offset"));

        let deserialized: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn trigger_entry_full_json_roundtrip() {
        let entry = StoryboardEntry {
            trigger_storyboard: Some("child_sb".to_string()),
            trigger_start_offset: Some(0.5),
            at: Some(dola::KeyframeRef::Single("kf_a".to_string())),
            keyframe: Some("kf_trigger".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("trigger_storyboard"));
        assert!(json.contains("trigger_start_offset"));

        let deserialized: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn trigger_entry_absent_fields_deserialize_to_none() {
        // variable/transition のみのエントリ → trigger フィールドは None
        let json = r#"{"variable":"opacity","transition":{"from":0.0,"to":1.0,"duration":1.0}}"#;
        let entry: StoryboardEntry = serde_json::from_str(json).unwrap();
        assert!(entry.trigger_storyboard.is_none());
        assert!(entry.trigger_start_offset.is_none());
    }

    #[test]
    fn compiled_trigger_serde_roundtrip() {
        let trigger = CompiledTrigger {
            fire_time: 2.5,
            target_storyboard: "child".to_string(),
            start_offset: Some(0.1),
            source_entry_index: 3,
        };

        let json = serde_json::to_string_pretty(&trigger).unwrap();
        let deserialized: CompiledTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, deserialized);
    }

    #[test]
    fn compiled_trigger_no_offset_serde_roundtrip() {
        let trigger = CompiledTrigger {
            fire_time: 1.0,
            target_storyboard: "x".to_string(),
            start_offset: None,
            source_entry_index: 0,
        };

        let json = serde_json::to_string_pretty(&trigger).unwrap();
        assert!(!json.contains("start_offset"));

        let deserialized: CompiledTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, deserialized);
    }
}
