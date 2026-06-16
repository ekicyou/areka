//! `between` keyframe interval time resolution.

use super::*;

#[test]
fn between_placement() {
    let doc = make_doc_with_storyboard(
        vec![
            (
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
            (
                "y",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
        ],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(2.0),
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(100.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(3.0),
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            // y between kf1 and kf2: from_t=2.0, to_t=5.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(999.0), // ignored for between
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

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 2.0); // from_kf time
    assert_eq!(tl_y.segments[0].end_time, 5.0); // to_kf time
}

#[test]
fn between_from_start_pseudo_keyframe() {
    // between の from に疑似KF "start" を使用できる
    let doc = make_doc_with_storyboard(
        vec![
            (
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
            (
                "y",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
        ],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(2.0),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            // y between start(0.0) → kf1(2.0)
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

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 0.0);
    assert_eq!(tl_y.segments[0].end_time, 2.0);
}

#[test]
fn between_with_valid_delay() {
    // 区間内に収まる delay: segment_start = from_t + delay
    let doc = make_doc_with_storyboard(
        vec![
            (
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
            (
                "y",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            ),
        ],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(2.0),
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
                    duration: Some(3.0),
                    ..Default::default()
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            // y between kf1(2.0) → kf2(5.0)、delay 1.0 → 3.0..5.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    delay: 1.0,
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

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 3.0);
    assert_eq!(tl_y.segments[0].end_time, 5.0);
}
