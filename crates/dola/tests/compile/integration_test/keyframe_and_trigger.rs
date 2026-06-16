//! Keyframe-reference resolution and trigger-entry time-resolution integration tests.

use super::*;

#[test]
fn pure_keyframe_with_at() {
    // Pure KF + at reference, used as anchor for another entry
    let doc = make_doc(
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
            // x: 0->100 in 2s
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
            // Pure KF: at kf1 + offset 1.0 → time = 3.0
            .entry(StoryboardEntry {
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("kf1".to_string()),
                    offset: 1.0,
                }),
                keyframe: Some("marker".to_string()),
                ..Default::default()
            })
            // y at marker → 3.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                at: Some(KeyframeRef::Single("marker".to_string())),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 3.0);
    assert_eq!(tl_y.segments[0].end_time, 4.0);
}

#[test]
fn multiple_keyframe_wait() {
    // Multiple keyframe reference: waits for all KFs
    let doc = make_doc(
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
            (
                "z",
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
            // x: 0..1s → kf_x at 1.0
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                keyframe: Some("kf_x".to_string()),
                ..Default::default()
            })
            // y: 0..3s → kf_y at 3.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(3.0),
                })),
                keyframe: Some("kf_y".to_string()),
                ..Default::default()
            })
            // z: at [kf_x, kf_y] → waits for max(1.0, 3.0) = 3.0
            .entry(StoryboardEntry {
                variable: Some("z".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                at: Some(KeyframeRef::Multiple(vec![
                    "kf_x".to_string(),
                    "kf_y".to_string(),
                ])),
                keyframe: Some("kf_z".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_z = result.timelines.get("z").unwrap();
    assert_eq!(tl_z.segments[0].start_time, 3.0); // max(1.0, 3.0)
    assert_eq!(tl_z.segments[0].end_time, 4.0);
}

// =========================================================
// D2-T gap tests: トリガーエントリの時刻解決（compile_storyboard 内ロジック）
// =========================================================

fn float_x_entry(duration: f64, kf: &str) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            duration: Some(duration),
            ..Default::default()
        })),
        keyframe: Some(kf.to_string()),
        ..Default::default()
    }
}

fn simple_child_storyboard() -> Storyboard {
    StoryboardBuilder::new().entry(float_x_entry(1.0, "ckf")).build()
}

#[test]
fn trigger_without_at_inherits_previous_entry_time() {
    // at なしトリガーは配列直前エントリの keyframe 時刻で発火する
    let parent = StoryboardBuilder::new()
        // entry 0: x 0.0..1.0, kf1 = 1.0
        .entry(float_x_entry(1.0, "kf1"))
        // entry 1: trigger（at なし）→ fire_time = 1.0
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .storyboard("parent", parent)
        .storyboard("child", simple_child_storyboard())
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();
    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    assert!(
        (compiled.triggers[0].fire_time - 1.0).abs() < 1e-9,
        "fire_time should inherit previous entry keyframe time 1.0, got {}",
        compiled.triggers[0].fire_time
    );
    assert_eq!(compiled.triggers[0].source_entry_index, 1);
}

#[test]
fn entry_anchored_at_trigger_keyframe() {
    // トリガーエントリの keyframe は fire_time（0秒完了）として登録され、
    // 後続エントリの at 参照のアンカーとして使える
    let parent = StoryboardBuilder::new()
        // entry 0: x 0.0..1.0, kf1 = 1.0
        .entry(float_x_entry(1.0, "kf1"))
        // entry 1: trigger at kf1, keyframe "trig_kf" = 1.0
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: Some(KeyframeRef::Single("kf1".to_string())),
            keyframe: Some("trig_kf".to_string()),
            ..Default::default()
        })
        // entry 2: y at trig_kf → 1.0..2.0
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(50.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            at: Some(KeyframeRef::Single("trig_kf".to_string())),
            keyframe: Some("kf2".to_string()),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .variable(
            "y",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .storyboard("parent", parent)
        .storyboard("child", simple_child_storyboard())
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();
    let tl_y = compiled.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 1.0);
    assert_eq!(tl_y.segments[0].end_time, 2.0);
}
