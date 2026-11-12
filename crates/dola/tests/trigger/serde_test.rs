//! トリガー機能のテスト — 5.1: serde 往復テスト

use dola::StoryboardEntry;

// ============================================================
// 5.1: serde 往復テスト
// ============================================================

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
