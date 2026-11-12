//! Validation tests for V7, V10-V13 (transition constraints, object type, value range, type mismatch)
//! Tasks 7.6, 8.6

use dola::*;
use std::collections::BTreeMap;

use super::common::minimal_valid_doc;

/// ヘルパー: f64変数付きドキュメント
fn doc_with_float_var(
    name: &str,
    initial: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> DolaDocument {
    let mut doc = minimal_valid_doc();
    doc.variable.insert(
        name.to_string(),
        AnimationVariableDef::Float { initial, min, max },
    );
    doc
}

/// ヘルパー: Object 変数 "bg" 付きドキュメント
fn doc_with_object_var() -> DolaDocument {
    let mut doc = minimal_valid_doc();
    doc.variable.insert(
        "bg".to_string(),
        AnimationVariableDef::Object {
            initial: DynamicValue::Null,
        },
    );
    doc
}

/// ヘルパー: エントリを並べた台本 "sb1" だけの storyboard
fn sb(entries: Vec<StoryboardEntry>) -> BTreeMap<String, Storyboard> {
    BTreeMap::from([(
        "sb1".to_string(),
        Storyboard {
            time_scale: 1.0,
            loop_count: 1,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset: None,
            entry: entries,
        },
    )])
}

/// ヘルパー: 変数 var へインライン遷移を張る台本 "sb1" を設定
fn with_transition(mut doc: DolaDocument, var: &str, trans: TransitionDef) -> DolaDocument {
    doc.storyboard = sb(vec![StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(trans)),
        ..Default::default()
    }]);
    doc
}

mod transition_v11_v12;
mod transition_v13_nan;
mod transition_v7_v10;
