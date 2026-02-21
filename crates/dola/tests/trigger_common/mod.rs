//! Shared helpers for trigger tests

use dola::runtime::EvaluatedValue;
use dola::{
    AnimationVariableDef, DolaDocument, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry,
    TransitionDef, TransitionRef, TransitionValue,
};

/// 最小の有効ドキュメント（変数 opacity + トランジション fade_t + ストーリーボード）
pub fn minimal_trigger_doc(parent_name: &str, child_name: &str) -> DolaDocument {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };
    let parent_sb = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            trigger_storyboard: Some(child_name.to_string()),
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

    DolaDocumentBuilder::new("1.0")
        .variable("opacity", opacity_var)
        .storyboard(parent_name, parent_sb)
        .storyboard(child_name, child_sb)
        .build()
        .expect("minimal_trigger_doc should be valid")
}

/// 親SBにタイミング付きトリガーを持つドキュメント
/// 親: opacity 0→1 (1s) + trigger at keyframe "kf1"
/// 子: opacity 1→0 (1s)
pub fn timed_trigger_doc(
    parent_name: &str,
    child_name: &str,
    trigger_offset: Option<f64>,
) -> DolaDocument {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };

    let parent_sb = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            keyframe: Some("kf1".to_string()),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some(child_name.to_string()),
            trigger_start_offset: trigger_offset,
            at: Some(dola::KeyframeRef::Single("kf1".to_string())),
            ..Default::default()
        })
        .build();

    let child_sb = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(1.0)),
                to: Some(TransitionValue::Scalar(0.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();

    DolaDocumentBuilder::new("1.0")
        .variable("opacity", opacity_var)
        .storyboard(parent_name, parent_sb)
        .storyboard(child_name, child_sb)
        .build()
        .expect("timed_trigger_doc should be valid")
}

pub fn extract_float(val: &EvaluatedValue) -> f64 {
    match val {
        EvaluatedValue::Float(v) => *v,
        other => panic!("expected Float, got {other:?}"),
    }
}
