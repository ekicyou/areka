//! Shared helpers for compile tests

use dola::*;
use std::collections::BTreeMap;

pub fn make_doc_with_storyboard(
    variables: Vec<(&str, AnimationVariableDef)>,
    transitions: Vec<(&str, TransitionDef)>,
    storyboard_name: &str,
    sb: Storyboard,
) -> DolaDocument {
    let mut variable = BTreeMap::new();
    for (name, def) in variables {
        variable.insert(name.to_string(), def);
    }
    let mut transition = BTreeMap::new();
    for (name, def) in transitions {
        transition.insert(name.to_string(), def);
    }
    let mut storyboard = BTreeMap::new();
    storyboard.insert(storyboard_name.to_string(), sb);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition,
        storyboard,
    }
}

/// 制約の無い f64 変数（初期値 0.0）
pub fn float_var() -> AnimationVariableDef {
    AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    }
}

/// 変数 `var` を 0.0→1.0 へ動かすインラインの遷移のエントリ
pub fn transition_entry(var: &str, delay: f64, duration: Option<f64>) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            delay,
            duration,
            ..Default::default()
        })),
        ..Default::default()
    }
}
