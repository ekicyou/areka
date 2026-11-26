//! compile のメタデータとヒントのテスト

use dola::*;

use super::common::{delta_ms, float_var, make_doc_with_storyboard, ms};

#[test]
fn time_scale_propagated_not_applied() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .time_scale(2.0)
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    assert_eq!(result.time_scale.get(), 2.0);
    // Segment times should NOT be affected by time_scale
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments[0].start_time, delta_ms(0));
    assert_eq!(tl.segments[0].end_time, delta_ms(1000));
}

#[test]
fn loop_count_propagated() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .loop_count(3)
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    assert_eq!(result.loop_count, 3);
}

#[test]
fn interruption_policy_propagated() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .interruption_policy(InterruptionPolicy::Cancel)
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    assert_eq!(result.interruption_policy, InterruptionPolicy::Cancel);
}

#[test]
fn base_duration_and_total_base_duration() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // x: 0.0 -> 1.0 (duration 1.0)
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            // y: 0.0 -> 3.0 (duration 3.0)
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(3000)),
                    ..Default::default()
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    assert_eq!(
        result.timelines.get("x").unwrap().base_duration,
        delta_ms(1000)
    );
    assert_eq!(
        result.timelines.get("y").unwrap().base_duration,
        delta_ms(3000)
    );
    assert_eq!(result.total_base_duration, ms(3000));
}
