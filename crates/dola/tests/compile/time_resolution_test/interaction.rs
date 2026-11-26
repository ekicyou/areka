//! sequential と `at` 配置の相互作用、セグメントの並べ替え

use super::*;

#[test]
fn sequential_after_at_placement_continues_from_segment_end() {
    // at 配置されたセグメントの end_time が同一変数の後続 sequential の基準になる
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // entry 0: at start+5.0 → 5.0..6.0
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("start".to_string()),
                    offset: delta_ms(5000),
                }),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            // entry 1: sequential → 6.0..7.0（配列の直前のエントリの終わりから継続）
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: None,
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(1000)),
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
    assert_eq!(tl.segments[1].start_time, delta_ms(6000));
    assert_eq!(tl.segments[1].end_time, delta_ms(7000));
    assert_eq!(tl.segments[1].from_value, TransitionValue::Scalar(100.0));
}

#[test]
fn out_of_order_at_entries_sorted_by_start_time() {
    // エントリ記述順と逆の時刻でも、コンパイル結果のセグメントは start_time 昇順
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // entry 0: at start+2.0 → 2.0..3.0（後の時刻を先に記述）
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(100.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("start".to_string()),
                    offset: delta_ms(2000),
                }),
                keyframe: Some("kf_late".to_string()),
                ..Default::default()
            })
            // entry 1: at start → 0.0..1.0
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::Single("start".to_string())),
                keyframe: Some("kf_early".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl = result.timelines.get("x").unwrap();
    assert_eq!(tl.segments.len(), 2);
    assert_eq!(tl.segments[0].start_time, delta_ms(0));
    assert_eq!(tl.segments[0].end_time, delta_ms(1000));
    assert_eq!(tl.segments[1].start_time, delta_ms(2000));
    assert_eq!(tl.segments[1].end_time, delta_ms(3000));
}
