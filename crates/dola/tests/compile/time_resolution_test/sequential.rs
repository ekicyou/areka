//! sequential の時刻の解決（storyboard の頭からの隔たり）

use super::*;

#[test]
fn simple_sequential_single_variable() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
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
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: None, // infer from previous
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(2000)),
                    ..Default::default()
                })),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments.len(), 2);

    assert_eq!(tl.segments[0].start_time, delta_ms(0));
    assert_eq!(tl.segments[0].end_time, delta_ms(1000));
    assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(0.0));
    assert_eq!(tl.segments[0].to_value, TransitionValue::Scalar(100.0));

    // Second segment: 1.0 -> 3.0 (sequential, from = previous to)
    assert_eq!(tl.segments[1].start_time, delta_ms(1000));
    assert_eq!(tl.segments[1].end_time, delta_ms(3000));
    assert_eq!(tl.segments[1].from_value, TransitionValue::Scalar(100.0));
    assert_eq!(tl.segments[1].to_value, TransitionValue::Scalar(200.0));
}

#[test]
fn sequential_with_delay() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    delay: ms(500),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl = result.timelines.get("x").unwrap();

    // delay = 0.5 → segment 0.5 -> 1.5
    assert_eq!(tl.segments[0].start_time, delta_ms(500));
    assert_eq!(tl.segments[0].end_time, delta_ms(1500));
}

#[test]
fn duration_zero_instant_transition() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: None, // instant
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments[0].start_time, tl.segments[0].end_time);
}

#[test]
fn sequential_first_entry_starts_from_head() {
    // First entry for a variable: base_time = 頭（0）
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
                    delay: ms(1000),
                    duration: Some(ms(2000)),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl = result.timelines.get("x").unwrap();
    // base_time = 0 (頭), segment_start = 0 + 1.0 = 1.0
    assert_eq!(tl.segments[0].start_time, delta_ms(1000));
    assert_eq!(tl.segments[0].end_time, delta_ms(3000));
    assert_eq!(tl.segments[0].from_value, TransitionValue::Scalar(10.0));
}

fn x_to(to: f64, at: Option<KeyframeRef>) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(ms(1000)),
            ..Default::default()
        })),
        at,
        ..Default::default()
    }
}

fn at_name(name: &str) -> Option<KeyframeRef> {
    Some(KeyframeRef::Single(name.to_string()))
}

fn kf_at_start_plus(name: &str, offset: TimeDelta) -> StoryboardEntry {
    StoryboardEntry {
        keyframe: Some(name.to_string()),
        at: Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("start".to_string()),
            offset,
        }),
        ..Default::default()
    }
}

/// 前方参照があっても、連結の遷移は同じ変数の配列上の直前のエントリに続く
#[test]
fn sequential_follows_previous_entry_in_array_despite_forward_ref() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(x_to(1.0, at_name("k")))
            .entry(x_to(2.0, None))
            .entry(kf_at_start_plus("k", delta_ms(5000)))
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let segs = &result.timelines["x"].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!(
        (segs[0].start_time, segs[0].end_time),
        (delta_ms(5000), delta_ms(6000))
    );
    assert_eq!(segs[0].from_value, TransitionValue::Scalar(0.0));
    assert_eq!(segs[0].to_value, TransitionValue::Scalar(1.0));
    assert_eq!(
        (segs[1].start_time, segs[1].end_time),
        (delta_ms(6000), delta_ms(7000))
    );
    assert_eq!(segs[1].from_value, TransitionValue::Scalar(1.0));
    assert_eq!(segs[1].to_value, TransitionValue::Scalar(2.0));
}

/// at の無いキーフレームは配列の 1 つ前のエントリに続く。前方参照を挟んでも正しい文書が通る
#[test]
fn pure_keyframe_without_at_follows_previous_entry_despite_forward_ref() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(x_to(1.0, at_name("k")))
            .entry(StoryboardEntry {
                keyframe: Some("m".to_string()),
                ..Default::default()
            })
            .entry(kf_at_start_plus("k", delta_ms(5000)))
            .entry(x_to(2.0, at_name("m")))
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let segs = &result.timelines["x"].segments;
    assert_eq!(segs.len(), 2);
    assert_eq!(
        (segs[0].start_time, segs[0].end_time),
        (delta_ms(5000), delta_ms(6000))
    );
    // "m" は entry 0 の終わり（6.0）
    assert_eq!(
        (segs[1].start_time, segs[1].end_time),
        (delta_ms(6000), delta_ms(7000))
    );
}

/// 連結の遷移は処理の順で最後に解いた同じ変数のエントリでなく、配列上の直前のエントリに続く
#[test]
fn sequential_follows_array_predecessor_not_last_processed() {
    // entry 1 が [0,1] で k=1。entry 0 は k に置かれ [1,2]。entry 2 は配列上の直前＝entry 1 に続き [1,2]
    // で entry 0 と重なる（処理の順で直前の entry 0 に続けると [2,3] で黙って通ってしまう）。
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
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

    let errors = compile_storyboard(&doc, "test").unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.to_string().contains("Segment overlap")),
        "expected an overlap error, got {errors:?}"
    );
}

/// from を省いた連結の遷移は、時刻の上で間に挟まった at のセグメントでなく、
/// 配列上の直前のエントリの to から始まる
#[test]
fn sequential_from_follows_array_predecessor_not_previous_in_time() {
    let x_entry =
        |from: Option<f64>, to: f64, delay: TimeSpan, duration: TimeSpan| StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: from.map(TransitionValue::Scalar),
                to: Some(TransitionValue::Scalar(to)),
                delay,
                duration: Some(duration),
                ..Default::default()
            })),
            ..Default::default()
        };
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // P: [0,1] 0→5
            .entry(x_entry(Some(0.0), 5.0, ms(0), ms(1000)))
            // C: P に続き delay 1 で [2,3]
            .entry(x_entry(None, 9.0, ms(1000), ms(1000)))
            // A: [1.2,1.7] 5→7（時刻の上では P と C の間）
            .entry(StoryboardEntry {
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("start".to_string()),
                    offset: delta_ms(1200),
                }),
                ..x_entry(None, 7.0, ms(0), ms(500))
            })
            .build(),
    );
    let result = compile_storyboard(&doc, "test").unwrap();
    let segs = &result.timelines["x"].segments;
    assert_eq!(segs[1].from_value, TransitionValue::Scalar(5.0));
    assert_eq!(
        (segs[2].start_time, segs[2].end_time),
        (delta_ms(2000), delta_ms(3000))
    );
    assert_eq!(segs[2].from_value, TransitionValue::Scalar(5.0));
}
