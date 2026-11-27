//! `between` のキーフレーム区間の時刻解決

use super::*;

#[test]
fn between_placement() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(2000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(100.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(3000)),
                    ..Default::default()
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            // y between kf1 and kf2: from_t=2000 ms, to_t=5000 ms
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(ms(999000)), // ignored for between
                    ..Default::default()
                })),
                between: Some(BetweenKeyframes {
                    from: "kf1".to_string(),
                    to: "kf2".to_string(),
                }),
                keyframe: Some("kf3".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, delta_ms(2000));
    assert_eq!(tl_y.segments[0].end_time, delta_ms(5000));
}

#[test]
fn between_from_start_pseudo_keyframe() {
    // between の from に疑似KF "start" を使用できる
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(2000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            // y between start(0) → kf1(2000 ms)
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    ..Default::default()
                })),
                between: Some(BetweenKeyframes {
                    from: "start".to_string(),
                    to: "kf1".to_string(),
                }),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, delta_ms(0));
    assert_eq!(tl_y.segments[0].end_time, delta_ms(2000));
}

#[test]
fn between_with_valid_delay() {
    // 区間内に収まる delay: segment_start = from_t + delay
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(2000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(100.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(3000)),
                    ..Default::default()
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            // y between kf1(2000 ms) → kf2(5000 ms)、delay 1.0 → 3000..5000 ms
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    delay: ms(1000),
                    ..Default::default()
                })),
                between: Some(BetweenKeyframes {
                    from: "kf1".to_string(),
                    to: "kf2".to_string(),
                }),
                keyframe: Some("kf3".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, delta_ms(3000));
    assert_eq!(tl_y.segments[0].end_time, delta_ms(5000));
}
