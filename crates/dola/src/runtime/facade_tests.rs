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

fn fade_x(from: f64, to: f64, duration: f64) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(from)),
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(duration),
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// 競合で終わったインスタンスのトリガーの記録は残らない（#35）
#[test]
fn conflict_termination_drops_trigger_store() {
    use crate::builder::{DolaDocumentBuilder, StoryboardBuilder};
    use crate::storyboard::KeyframeRef;

    // 親: x 0→1（2 秒・kf1）＋ kf1 で child。interrupt: x を触る（親と競合）
    let parent = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Cancel)
        .entry(StoryboardEntry {
            keyframe: Some("kf1".to_string()),
            ..fade_x(0.0, 1.0, 2.0)
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: Some(KeyframeRef::Single("kf1".to_string())),
            ..Default::default()
        })
        .build();
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float(0.0))
        .storyboard("parent", parent)
        .storyboard(
            "child",
            StoryboardBuilder::new()
                .entry(fade_x(1.0, 0.0, 1.0))
                .build(),
        )
        .storyboard(
            "interrupt",
            StoryboardBuilder::new()
                .entry(fade_x(0.0, 0.5, 1.0))
                .build(),
        )
        .build()
        .unwrap();

    let mut rt = DolaRuntime::new();
    rt.load_document(doc).unwrap();
    let parent_gid = rt.start("parent", 0.0).unwrap().group_id;
    assert!(rt.trigger_store.contains_key(&parent_gid));

    let r = rt.start("interrupt", 1.0).unwrap();
    assert_eq!(r.affected_group_ids, vec![parent_gid]);
    assert!(
        rt.trigger_store.is_empty(),
        "trigger records left: {:?}",
        rt.trigger_store.keys().collect::<Vec<_>>()
    );
}

/// Never と競合して拒まれた新しいインスタンスは表に残らない（#43 の 3 本目）
#[test]
fn never_conflict_removes_new_instance() {
    use crate::builder::{DolaDocumentBuilder, StoryboardBuilder};

    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float(0.0))
        .storyboard(
            "never",
            StoryboardBuilder::new()
                .interruption_policy(InterruptionPolicy::Never)
                .entry(fade_x(0.0, 1.0, 2.0))
                .build(),
        )
        .storyboard(
            "new",
            StoryboardBuilder::new()
                .entry(fade_x(1.0, 0.0, 1.0))
                .build(),
        )
        .build()
        .unwrap();

    let mut rt = DolaRuntime::new();
    rt.load_document(doc).unwrap();
    let never_gid = rt.start("never", 0.0).unwrap().group_id;

    assert!(matches!(
        rt.start("new", 0.5),
        Err(RuntimeError::Conflict { .. })
    ));
    assert_eq!(
        rt.instance_manager.instances().keys().collect::<Vec<_>>(),
        vec![&never_gid]
    );
}

/// start で作るトリガーの記録は添字 0..n の順で、どれも未発火
#[test]
fn start_initializes_trigger_states_in_order() {
    use crate::builder::{DolaDocumentBuilder, StoryboardBuilder};
    use crate::storyboard::KeyframeRef;

    let trigger = || StoryboardEntry {
        trigger_storyboard: Some("child".to_string()),
        at: Some(KeyframeRef::Single("kf1".to_string())),
        ..Default::default()
    };
    let parent = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            keyframe: Some("kf1".to_string()),
            ..fade_x(0.0, 1.0, 2.0)
        })
        .entry(trigger())
        .entry(trigger())
        .entry(trigger())
        .build();
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float(0.0))
        .variable("y", float(0.0))
        .storyboard("parent", parent)
        .storyboard(
            "child",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    ..fade_x(0.0, 1.0, 1.0)
                })
                .build(),
        )
        .build()
        .unwrap();

    let mut rt = DolaRuntime::new();
    rt.load_document(doc).unwrap();
    let gid = rt.start("parent", 0.0).unwrap().group_id;

    let states = &rt.instance_manager.get(gid).unwrap().trigger_states;
    let got: Vec<(usize, bool)> = states.iter().map(|t| (t.trigger_index, t.fired)).collect();
    assert_eq!(got, vec![(0, false), (1, false), (2, false)]);
}
