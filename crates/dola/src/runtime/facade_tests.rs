//! DolaRuntime の中の欄を見るテスト（公開の API から見えない後始末を縛る）。

use std::collections::BTreeMap;

use super::*;
use crate::storyboard::{InterruptionPolicy, Storyboard, StoryboardEntry};
use crate::transition::{TransitionDef, TransitionRef, TransitionValue};
use crate::variable::AnimationVariableDef;

fn float(initial: f64) -> AnimationVariableDef {
    AnimationVariableDef::Float {
        initial,
        min: None,
        max: None,
    }
}

fn make_valid_doc() -> DolaDocument {
    let mut storyboard = BTreeMap::new();
    storyboard.insert(
        "fade".to_string(),
        Storyboard {
            time_scale: 1.0,
            loop_count: 1,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset: None,
            entry: vec![StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            }],
        },
    );
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable: BTreeMap::from([("x".to_string(), float(0.0))]),
        transition: BTreeMap::new(),
        storyboard,
    }
}

fn make_invalid_doc() -> DolaDocument {
    DolaDocument {
        schema_version: "99.0".to_string(),
        variable: BTreeMap::new(),
        transition: BTreeMap::new(),
        storyboard: BTreeMap::new(),
    }
}

#[test]
fn initial_state_has_no_document() {
    assert!(DolaRuntime::new().document.is_none());
}

#[test]
fn invalid_document_preserves_existing() {
    let mut rt = DolaRuntime::new();
    rt.load_document(make_valid_doc()).unwrap();

    assert!(rt.load_document(make_invalid_doc()).is_err());
    assert_eq!(rt.document.as_ref().unwrap().schema_version, "1.0");
}

/// 初めの読み込みが失敗したら文書は無いまま
#[test]
fn invalid_document_on_empty_runtime_keeps_none() {
    let mut rt = DolaRuntime::new();
    assert!(rt.load_document(make_invalid_doc()).is_err());
    assert!(rt.document.is_none());
}

#[test]
fn overwrite_document() {
    let mut rt = DolaRuntime::new();
    rt.load_document(make_valid_doc()).unwrap();

    let mut doc2 = make_valid_doc();
    doc2.variable.insert("y".to_string(), float(5.0));
    rt.load_document(doc2).unwrap();
    assert!(rt.document.as_ref().unwrap().variable.contains_key("y"));
}
