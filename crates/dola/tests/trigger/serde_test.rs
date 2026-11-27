//! トリガー機能のテスト — serde 往復テスト

use dola::{StoryboardEntry, TimeDelta};

// ============================================================
// serde 往復テスト
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
        trigger_start_offset: Some(TimeDelta::from_millis_saturating(-500)),
        at: Some(dola::KeyframeRef::Single("kf_a".to_string())),
        keyframe: Some("kf_trigger".to_string()),
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&entry).unwrap();
    assert!(json.contains("trigger_storyboard"));
    assert!(json.contains("trigger_start_offset"));
    assert!(json.contains("-0.5"));

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
fn trigger_start_offset_limit_edges_on_both_sides() {
    const LIMIT: i64 = 1 << 51;
    let entry = |ticks: i64| StoryboardEntry {
        trigger_storyboard: Some("child_sb".to_string()),
        trigger_start_offset: Some(TimeDelta::from_ticks(ticks)),
        ..Default::default()
    };
    for ticks in [LIMIT - 1, -(LIMIT - 1)] {
        let json = serde_json::to_string(&entry(ticks)).unwrap();
        let back: StoryboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back, entry(ticks));
    }
    for ticks in [LIMIT, -LIMIT] {
        assert!(serde_json::to_string(&entry(ticks)).is_err());
    }
    for secs in ["225179981.3685248", "-225179981.3685248"] {
        let json = format!(r#"{{"trigger_storyboard":"c","trigger_start_offset":{secs}}}"#);
        assert!(serde_json::from_str::<StoryboardEntry>(&json).is_err());
    }
}

#[test]
fn non_finite_trigger_start_offset_fails_to_load() {
    let json = r#"{"trigger_storyboard":"c","trigger_start_offset":1e400}"#;
    assert!(serde_json::from_str::<StoryboardEntry>(json).is_err());
}
