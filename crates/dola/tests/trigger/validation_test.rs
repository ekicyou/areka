//! トリガー機能のテスト — 5.2: バリデーションユニットテスト（V9更新, V14t–V18t）

use dola::{
    AnimationVariableDef, DolaDocumentBuilder, DolaError, StoryboardBuilder, StoryboardEntry,
    TransitionDef, TransitionRef, TransitionValue, Validate,
};
use super::common::minimal_trigger_doc;

// ============================================================
// 5.2: バリデーションユニットテスト
// ============================================================

#[cfg(test)]
mod validation_tests {
    use super::*;

    /// V9 更新: trigger_storyboard のみのエントリは V9 エラーにならない
    #[test]
    fn v9_trigger_only_entry_is_valid() {
        let doc = minimal_trigger_doc("parent", "child");
        assert!(doc.validate().is_ok());
    }

    /// V9: variable も transition も keyframe も trigger もない空エントリはエラー
    #[test]
    fn v9_no_variable_no_trigger_is_error() {
        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                // 何も設定しない空エントリ
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .storyboard("test", sb)
            .build();

        // V9: entry without variable/transition must have keyframe or trigger_storyboard
        assert!(doc.is_err());
    }

    /// V14t: 自己参照検出
    #[test]
    fn v14t_self_reference_detected() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };
        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("self_ref".to_string()),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("self_ref", sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerSelfReference { .. }))
        );
    }

    /// V15t: A→B→A 循環検出
    #[test]
    fn v15t_cycle_a_b_a() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .build();

        let sb_b = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_a".to_string()),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", sb_b)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerCycle { .. }))
        );
    }

    /// V15t: A→B→C→A 循環検出
    #[test]
    fn v15t_cycle_a_b_c_a() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .build();

        let sb_b = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_c".to_string()),
                ..Default::default()
            })
            .build();

        let sb_c = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_a".to_string()),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", sb_b)
            .storyboard("sb_c", sb_c)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerCycle { .. }))
        );
    }

    /// V15t: 非循環チェーン A→B→C は OK
    #[test]
    fn v15t_no_cycle_chain_is_valid() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .build();

        let sb_b = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_c".to_string()),
                ..Default::default()
            })
            .build();

        let sb_c = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", sb_b)
            .storyboard("sb_c", sb_c)
            .build();

        assert!(
            doc.is_ok(),
            "Non-cyclic chain should be valid: {:?}",
            doc.err()
        );
    }

    /// V16t: trigger + variable の同時指定はエラー
    #[test]
    fn v16t_trigger_with_variable_is_error() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("child".to_string()),
                variable: Some("opacity".to_string()),
                ..Default::default()
            })
            .build();

        let child_sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("parent", sb)
            .storyboard("child", child_sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerExclusiveViolation { .. }))
        );
    }

    /// V16t: trigger + transition の同時指定はエラー
    #[test]
    fn v16t_trigger_with_transition_is_error() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("child".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let child_sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("parent", sb)
            .storyboard("child", child_sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerExclusiveViolation { .. }))
        );
    }

    /// V18t: 存在しないストーリーボードへのトリガーはエラー
    #[test]
    fn v18t_trigger_target_not_found() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("nonexistent".to_string()),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("parent", sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerTargetNotFound { .. }))
        );
    }
}
