//! E2E full-flow integration tests
//! Tasks 12.1, 11.1

use dola::*;
use std::collections::BTreeMap;

// =============================================================
// Task 12.1: E2E — 全配置パターン統合
// =============================================================

mod e2e_tests {
    use super::super::*;

    #[test]
    fn builder_validate_serialize_deserialize_revalidate() {
        // Builder API → build (validate) → serialize → deserialize → validate again
        let doc = build_complete_document();
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: DolaDocument = serde_json::from_str(&json).unwrap();
        // Re-validate
        assert!(deserialized.validate().is_ok());
        assert_eq!(doc, deserialized);
    }

    #[test]
    fn implicit_keyframe_chain_pattern() {
        // Test that omitting keyframe still allows chain pattern (via implicit KFs)
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )
            .storyboard(
                "chain",
                StoryboardBuilder::new()
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(1.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        ..Default::default()
                    })
                    .entry(StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(2.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        ..Default::default()
                    })
                    .build(),
            )
            .build()
            .unwrap();

        assert!(doc.validate().is_ok());
    }
}

// =============================================================
// Task 11.1: Feature gates 動作検証（コンパイル時チェック）
// =============================================================

#[test]
fn feature_json_enabled_by_default() {
    // serde_json が利用可能であることを確認（defaultフィーチャーにjson含む）
    let doc = DolaDocument {
        schema_version: "1.0".to_string(),
        variable: BTreeMap::new(),
        transition: BTreeMap::new(),
        storyboard: BTreeMap::new(),
    };
    let _json = serde_json::to_string(&doc).unwrap();
}
