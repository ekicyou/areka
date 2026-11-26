//! トリガー機能のテスト — ループ内トリガー（各周回の再発火・飛ばした周回）

use super::common::{at_ms, delta_ms, ms};
use dola::runtime::{DolaRuntime, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry, TimeDelta,
    TimePoint, TransitionDef, TransitionRef, TransitionValue,
};

// ============================================================
// ループ内トリガー統合テスト
// ============================================================

/// loop_count=2 のSBでトリガーが各周回で再発火する
#[test]
fn trigger_refires_each_loop() {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };

    let parent_sb = StoryboardBuilder::new()
        .loop_count(2)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(1000)),
                ..Default::default()
            })),
            keyframe: Some("kf1".to_string()),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
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
                duration: Some(ms(500)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable("opacity", opacity_var)
        .storyboard("parent", parent_sb)
        .storyboard("child", child_sb)
        .build()
        .expect("doc should be valid");

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("parent", at_ms(0)).unwrap();

    // 周回1: t=1.0 でトリガー発火
    rt.tick(at_ms(1000));
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "loop 1: trigger should fire");

    // 周回2: t=2.0 でトリガー再発火
    rt.tick(at_ms(2000));
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "loop 2: trigger should re-fire");
}

/// 無限ループ (-1) でもトリガーが各周回で発火
#[test]
fn infinite_loop_trigger_fires_each_cycle() {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };

    let parent_sb = StoryboardBuilder::new()
        .loop_count(-1)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(1000)),
                ..Default::default()
            })),
            keyframe: Some("kf1".to_string()),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
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
                duration: Some(ms(500)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable("opacity", opacity_var)
        .storyboard("parent", parent_sb)
        .storyboard("child", child_sb)
        .build()
        .expect("doc should be valid");

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("parent", at_ms(0)).unwrap();

    // 3 周回分チェック
    for cycle in 1..=3 {
        let t = cycle * 1000;
        rt.tick(at_ms(t));
        let result = rt.last_result().clone();
        assert_eq!(
            result.triggered.len(),
            1,
            "cycle {cycle}: trigger should fire once at t={t}ms"
        );
    }
}

/// 3 周（1 周 1 秒）の親。`at` は start からの秒で、x を 0→1 で動かす子 "child"（0.2 秒）を起こす
pub(super) fn three_loop_trigger_doc(at: TimeDelta) -> dola::DolaDocument {
    let float_var = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    let parent_sb = StoryboardBuilder::new()
        .loop_count(3)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(1000)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: Some(dola::KeyframeRef::WithOffset {
                keyframes: dola::KeyframeNames::Single("start".to_string()),
                offset: at,
            }),
            ..Default::default()
        })
        .build();
    let child_sb = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(200)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();
    DolaDocumentBuilder::new("1.0")
        .variable("opacity", float_var())
        .variable("x", float_var())
        .storyboard("parent", parent_sb)
        .storyboard("child", child_sb)
        .build()
        .expect("doc should be valid")
}

/// 1 回の tick で周回を飛ばしても、飛ばした周回のトリガーはその周回の壁時計の時刻で起きる
#[test]
fn skipped_loops_fire_their_triggers() {
    let mut rt = DolaRuntime::new();
    rt.load_document(three_loop_trigger_doc(delta_ms(500)))
        .unwrap();
    rt.start("parent", at_ms(0)).unwrap();
    rt.tick(at_ms(0));
    rt.tick(at_ms(3500));
    let child_ends: Vec<Option<TimePoint>> = rt
        .last_result()
        .triggered
        .iter()
        .map(|t| match t {
            TriggerResult::Started { start_result, .. } => start_result.end_time,
            TriggerResult::Error { error, .. } => panic!("trigger failed: {error}"),
        })
        .collect();
    // 子は 0.5・1.5・2.5 に起き、0.2 秒で終わる
    assert_eq!(
        child_ends,
        vec![Some(at_ms(700)), Some(at_ms(1700)), Some(at_ms(2700))]
    );
    assert!(!rt.has_instances(), "parent and children should all end");
}

/// 周回の長さより後ろのトリガーは、飛ばした周回でも起きない（親の周回が先に終わる）
#[test]
fn trigger_after_loop_end_does_not_fire_in_skipped_loops() {
    let mut rt = DolaRuntime::new();
    rt.load_document(three_loop_trigger_doc(delta_ms(1200)))
        .unwrap();
    rt.start("parent", at_ms(0)).unwrap();
    rt.tick(at_ms(0));
    rt.tick(at_ms(3500));
    assert!(rt.last_result().triggered.is_empty());
}
