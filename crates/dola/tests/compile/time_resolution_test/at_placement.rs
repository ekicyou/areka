//! `at` placement and pure-keyframe time resolution.

use super::*;

#[test]
fn at_reference_keyframe() {
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // Entry 0: x, sequential, keyframe "kf1" ends at 2.0
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
            // Entry 1: y, at "kf1", starts at 2.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::Single("kf1".to_string())),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 2.0);
    assert_eq!(tl_y.segments[0].end_time, 3.0);
}

#[test]
fn at_with_offset() {
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
                    duration: Some(2.0),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("kf1".to_string()),
                    offset: 0.5,
                }),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    // kf1 ends at 2.0, offset 0.5 → base_time = 2.5, segment 2.5 -> 3.5
    assert_eq!(tl_y.segments[0].start_time, 2.5);
    assert_eq!(tl_y.segments[0].end_time, 3.5);
}

#[test]
fn pure_keyframe_without_at_inherits_previous_entry_time() {
    // 純粋KF（at なし）は配列直前エントリの keyframe 時刻を継承する
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // entry 0: x 0.0..2.0, kf1 = 2.0
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
            // entry 1: 純粋KF（at なし）→ 直前エントリの keyframe 時刻 2.0 を継承
            .entry(StoryboardEntry {
                keyframe: Some("marker".to_string()),
                ..Default::default()
            })
            // entry 2: y at marker → 2.0..3.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::Single("marker".to_string())),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 2.0);
    assert_eq!(tl_y.segments[0].end_time, 3.0);
}

#[test]
fn pure_keyframe_without_at_as_first_entry_falls_back_to_start() {
    // 先頭エントリの純粋KF（at なし）は "start"（= compile start_time）へフォールバック
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // entry 0: 純粋KF（at なし）が先頭 → start_time を継承
            .entry(StoryboardEntry {
                keyframe: Some("marker".to_string()),
                ..Default::default()
            })
            // entry 1: x at marker
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::Single("marker".to_string())),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 5.0).unwrap();
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments[0].start_time, 5.0);
    assert_eq!(tl.segments[0].end_time, 6.0);
}

#[test]
fn at_with_offset_multiple_keyframes() {
    // WithOffset + Multiple: 全KFの最大時刻 + offset
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var()), ("z", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // x: 0..1s → kf_x = 1.0
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                keyframe: Some("kf_x".to_string()),
                ..Default::default()
            })
            // y: 0..3s → kf_y = 3.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(3.0),
                    ..Default::default()
                })),
                keyframe: Some("kf_y".to_string()),
                ..Default::default()
            })
            // z: at [kf_x, kf_y] + 0.5 → max(1.0, 3.0) + 0.5 = 3.5
            .entry(StoryboardEntry {
                variable: Some("z".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Multiple(vec![
                        "kf_x".to_string(),
                        "kf_y".to_string(),
                    ]),
                    offset: 0.5,
                }),
                keyframe: Some("kf_z".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_z = result.timelines.get("z").unwrap();
    assert_eq!(tl_z.segments[0].start_time, 3.5);
    assert_eq!(tl_z.segments[0].end_time, 4.5);
}

#[test]
fn at_with_negative_offset() {
    // 負オフセット: base_time + offset で基準より前に配置できる
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
                    duration: Some(2.0),
                    ..Default::default()
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            // kf1 = 2.0, offset -0.5 → segment 1.5..2.5
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("kf1".to_string()),
                    offset: -0.5,
                }),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test", 0.0).unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, 1.5);
    assert_eq!(tl_y.segments[0].end_time, 2.5);
}

/// from を省いた at の遷移は、処理の順でなく時刻の上で直前のセグメントの to から始まる。
/// キーフレームの並べ方を入れ替えても from は同じ（完了 dola-compiled-transition 要件 4.2）
#[test]
fn at_from_follows_previous_segment_in_time_regardless_of_keyframe_order() {
    let at_entry = |kf: &str, to: f64| StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(1.0),
            ..Default::default()
        })),
        at: Some(KeyframeRef::Single(kf.to_string())),
        ..Default::default()
    };
    let kf_entry = |name: &str, offset: f64| StoryboardEntry {
        keyframe: Some(name.to_string()),
        at: Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("start".to_string()),
            offset,
        }),
        ..Default::default()
    };
    for kfs in [
        [("k", 5.0), ("j", 1.0)], // 処理は k→[0]→j→[1]
        [("j", 1.0), ("k", 5.0)], // 処理は j→[1]→k→[0]
    ] {
        let doc = make_doc_with_storyboard(
            vec![("x", float_var())],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(at_entry("k", 10.0))
                .entry(at_entry("j", 3.0))
                .entry(kf_entry(kfs[0].0, kfs[0].1))
                .entry(kf_entry(kfs[1].0, kfs[1].1))
                .build(),
        );
        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let fromto: Vec<_> = result.timelines["x"]
            .segments
            .iter()
            .map(|s| (s.start_time, s.from_value.clone(), s.to_value.clone()))
            .collect();
        assert_eq!(
            fromto,
            vec![
                (
                    1.0,
                    TransitionValue::Scalar(0.0),
                    TransitionValue::Scalar(3.0)
                ),
                (
                    5.0,
                    TransitionValue::Scalar(3.0),
                    TransitionValue::Scalar(10.0)
                ),
            ],
            "keyframe order {kfs:?}"
        );
    }
}

/// 同じ時刻の長さ 0 の at の遷移は、キーフレームの並べ方（処理の順）に関わらず配列の順に並ぶ
#[test]
fn zero_length_at_segments_at_same_time_keep_array_order() {
    let at_entry = |kf: &str, to: f64| StoryboardEntry {
        variable: Some("x".to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(0.0),
            ..Default::default()
        })),
        at: Some(KeyframeRef::Single(kf.to_string())),
        ..Default::default()
    };
    let kf_entry = |name: &str| StoryboardEntry {
        keyframe: Some(name.to_string()),
        at: Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("start".to_string()),
            offset: 1.0,
        }),
        ..Default::default()
    };
    let compile = |kfs: [&str; 2]| {
        let doc = make_doc_with_storyboard(
            vec![("x", float_var())],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(at_entry("k", 3.0))
                .entry(at_entry("j", 10.0))
                .entry(kf_entry(kfs[0]))
                .entry(kf_entry(kfs[1]))
                .build(),
        );
        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        result.timelines["x"]
            .segments
            .iter()
            .map(|s| (s.from_value.clone(), s.to_value.clone()))
            .collect::<Vec<_>>()
    };
    let expected = vec![
        (TransitionValue::Scalar(0.0), TransitionValue::Scalar(3.0)),
        (TransitionValue::Scalar(3.0), TransitionValue::Scalar(10.0)),
    ];
    assert_eq!(compile(["k", "j"]), expected);
    assert_eq!(compile(["j", "k"]), expected);
}
