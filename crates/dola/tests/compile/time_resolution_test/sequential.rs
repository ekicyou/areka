//! Sequential timing and start-time offset resolution.

use super::*;

#[test]
fn simple_sequential_single_variable() {
    // 1 variable, 2 sequential entries
    let doc = make_doc_with_storyboard(
        vec![(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )],
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
                    duration: Some(1.0),
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: None, // infer from previous
                    to: Some(TransitionValue::Scalar(200.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(2.0),
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments.len(), 2);

    // First segment: 0.0 -> 1.0
    assert_eq!(tl.segments[0].start_time, 0.0);
    assert_eq!(tl.segments[0].end_time, 1.0);
    assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(0.0));
    assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(100.0));

    // Second segment: 1.0 -> 3.0 (sequential, from = previous to)
    assert_eq!(tl.segments[1].start_time, 1.0);
    assert_eq!(tl.segments[1].end_time, 3.0);
    assert_eq!(tl.segments[1].from_value, TransitionValue::Scalar(100.0));
    assert_eq!(tl.segments[1].to_value, TransitionValue::Scalar(200.0));
}

#[test]
fn sequential_with_delay() {
    let doc = make_doc_with_storyboard(
        vec![(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )],
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
                    delay: 0.5,
                    duration: Some(1.0),
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 10.0).unwrap();
    let tl = result.timelines.get("x").unwrap();

    // start_time = 10.0, delay = 0.5 → segment 10.5 -> 11.5
    assert_eq!(tl.segments[0].start_time, 10.5);
    assert_eq!(tl.segments[0].end_time, 11.5);
}

#[test]
fn duration_zero_instant_transition() {
    let doc = make_doc_with_storyboard(
        vec![(
            "x",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )],
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
                    duration: None, // instant
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments[0].start_time, tl.segments[0].end_time);
}

#[test]
fn sequential_first_entry_uses_compile_start_time() {
    // First entry for a variable: base_time = compile start_time
    let doc = make_doc_with_storyboard(
        vec![(
            "x",
            AnimationVariableDef::Float {
                initial: 10.0,
                min: None,
                max: None,
            },
        )],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: None, // infer from initial = 10.0
                    to: Some(TransitionValue::Scalar(20.0)),
                    relative_to: None,
                    easing: None,
                    delay: 1.0,
                    duration: Some(2.0),
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 3.0).unwrap();
    let tl = result.timelines.get("x").unwrap();
    // base_time = 3.0 (compile start_time), segment_start = 3.0 + 1.0 = 4.0
    assert_eq!(tl.segments[0].start_time, 4.0);
    assert_eq!(tl.segments[0].end_time, 6.0);
    assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(10.0));
    assert_eq!(result.start_time, 3.0);
}

fn x_var() -> Vec<(&'static str, AnimationVariableDef)> {
    vec![(
        "x",
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    )]
}

fn x_to(to: f64, at: Option<KeyframeRef>) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(1.0),
            ..Default::default()
        })),
        at,
        ..Default::default()
    }
}

fn at_name(name: &str) -> Option<KeyframeRef> {
    Some(KeyframeRef::Single(name.to_string()))
}

fn kf_at_start_plus(name: &str, offset: f64) -> StoryboardEntry {
    StoryboardEntry {
        keyframe: Some(name.to_string()),
        at: Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("start".to_string()),
            offset,
        }),
        ..Default::default()
    }
}

/// 前方参照があっても、連結の遷移は同じ変数の配列上の直前のエントリに続く（要件 11.7・#5）
#[test]
fn sequential_follows_previous_entry_in_array_despite_forward_ref() {
    let doc = make_doc_with_storyboard(
        x_var(),
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(x_to(1.0, at_name("k")))
            .entry(x_to(2.0, None))
            .entry(kf_at_start_plus("k", 5.0))
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let segs = &result.timelines["x"].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!((segs[0].start_time, segs[0].end_time), (5.0, 6.0));
    assert_eq!(segs[0].from_value, TransitionValue::Scalar(0.0));
    assert_eq!(segs[0].to_value, TransitionValue::Scalar(1.0));
    assert_eq!((segs[1].start_time, segs[1].end_time), (6.0, 7.0));
    assert_eq!(segs[1].from_value, TransitionValue::Scalar(1.0));
    assert_eq!(segs[1].to_value, TransitionValue::Scalar(2.0));
}

/// at の無いキーフレームは配列の 1 つ前のエントリに続く。前方参照を挟んでも正しい文書が通る（要件 11.7・#5）
#[test]
fn pure_keyframe_without_at_follows_previous_entry_despite_forward_ref() {
    let doc = make_doc_with_storyboard(
        x_var(),
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(x_to(1.0, at_name("k")))
            .entry(StoryboardEntry {
                keyframe: Some("m".to_string()),
                ..Default::default()
            })
            .entry(kf_at_start_plus("k", 5.0))
            .entry(x_to(2.0, at_name("m")))
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let segs = &result.timelines["x"].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!((segs[0].start_time, segs[0].end_time), (5.0, 6.0));
    // "m" は entry 0 の終わり（6.0）
    assert_eq!((segs[1].start_time, segs[1].end_time), (6.0, 7.0));
}

/// 連結の遷移は処理の順で最後に解いた同じ変数のエントリでなく、配列上の直前のエントリに続く（要件 11.7）
#[test]
fn sequential_follows_array_predecessor_not_last_processed() {
    // entry 1 が [0,1] で k=1。entry 0 は k に置かれ [1,2]。entry 2 は配列上の直前＝entry 1 に続き [1,2]
    // で entry 0 と重なる（処理の順で直前の entry 0 に続けると [2,3] で黙って通ってしまう）。
    let doc = make_doc_with_storyboard(
        x_var(),
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(x_to(1.0, at_name("k")))
            .entry(StoryboardEntry {
                keyframe: Some("k".to_string()),
                ..x_to(2.0, at_name("start"))
            })
            .entry(x_to(3.0, None))
            .build(),
    );

    let errors = compile_storyboard(&doc, "test", 0.0).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("Segment overlap")),
        "expected an overlap error, got {errors:?}"
    );
}
