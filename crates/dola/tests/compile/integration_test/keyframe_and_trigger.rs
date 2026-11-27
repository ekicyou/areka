//! キーフレーム参照の解決と、トリガーエントリの時刻解決の統合テスト

use super::*;

#[test]
fn pure_keyframe_with_at() {
    // Pure KF + at reference, used as anchor for another entry
    let doc = make_doc(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // x: 0->100 in 2000 ms
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
            // Pure KF: at kf1 + offset 1000 ms → time = 3000 ms
            .entry(StoryboardEntry {
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("kf1".to_string()),
                    offset: delta_ms(1000),
                }),
                keyframe: Some("marker".to_string()),
                ..Default::default()
            })
            // y at marker → 3000 ms
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::Single("marker".to_string())),
                keyframe: Some("kf2".to_string()),
                ..Default::default()
            })
            .build(),
    );

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl_y = result.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, delta_ms(3000));
    assert_eq!(tl_y.segments[0].end_time, delta_ms(4000));
}

#[test]
fn multiple_keyframe_wait() {
    // Multiple keyframe reference: waits for all KFs
    let doc = make_doc(
        vec![("x", float_var()), ("y", float_var()), ("z", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // x: 0..1000 ms → kf_x at 1000 ms
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                keyframe: Some("kf_x".to_string()),
                ..Default::default()
            })
            // y: 0..3000 ms → kf_y at 3000 ms
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(200.0)),
                    duration: Some(ms(3000)),
                    ..Default::default()
                })),
                keyframe: Some("kf_y".to_string()),
                ..Default::default()
            })
            // z: at [kf_x, kf_y] → waits for max(1000, 3000) = 3000 ms
            .entry(StoryboardEntry {
                variable: Some("z".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
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

    let result = compile_storyboard(&doc, "test").unwrap();
    let tl_z = result.timelines.get("z").unwrap();
    assert_eq!(tl_z.segments[0].start_time, delta_ms(3000)); // max(1000, 3000)
    assert_eq!(tl_z.segments[0].end_time, delta_ms(4000));
}

// =========================================================
// トリガーエントリの時刻解決（compile_storyboard 内ロジック）
// =========================================================

fn float_x_entry(duration: TimeSpan, kf: &str) -> StoryboardEntry {
    StoryboardEntry {
        keyframe: Some(kf.to_string()),
        ..transition_entry("x", ms(0), Some(duration))
    }
}

fn simple_child_storyboard() -> Storyboard {
    StoryboardBuilder::new()
        .entry(float_x_entry(ms(1000), "ckf"))
        .build()
}

#[test]
fn trigger_without_at_inherits_previous_entry_time() {
    // at なしトリガーは配列直前エントリの keyframe 時刻で発火する
    let parent = StoryboardBuilder::new()
        // entry 0: x 0..1000 ms, kf1 = 1000 ms
        .entry(float_x_entry(ms(1000), "kf1"))
        // entry 1: trigger（at なし）→ fire_time = 1000 ms
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float_var())
        .storyboard("parent", parent)
        .storyboard("child", simple_child_storyboard())
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "parent").unwrap();
    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    assert_eq!(
        compiled.triggers[0].fire_time,
        delta_ms(1000),
        "fire_time should inherit previous entry keyframe time 1.0, got {}",
        compiled.triggers[0].fire_time
    );
    assert_eq!(compiled.triggers[0].source_entry_index, 1);
}

#[test]
fn entry_anchored_at_trigger_keyframe() {
    // トリガーエントリの keyframe は fire_time（長さ 0 で完了）として登録され、
    // 後続エントリの at 参照のアンカーとして使える
    let parent = StoryboardBuilder::new()
        // entry 0: x 0..1000 ms, kf1 = 1000 ms
        .entry(float_x_entry(ms(1000), "kf1"))
        // entry 1: trigger at kf1, keyframe "trig_kf" = 1000 ms
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: Some(KeyframeRef::Single("kf1".to_string())),
            keyframe: Some("trig_kf".to_string()),
            ..Default::default()
        })
        // entry 2: y at trig_kf → 1000..2000 ms
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(50.0)),
                duration: Some(ms(1000)),
                ..Default::default()
            })),
            at: Some(KeyframeRef::Single("trig_kf".to_string())),
            keyframe: Some("kf2".to_string()),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float_var())
        .variable("y", float_var())
        .storyboard("parent", parent)
        .storyboard("child", simple_child_storyboard())
        .build()
        .unwrap();

    let compiled = compile_storyboard(&doc, "parent").unwrap();
    let tl_y = compiled.timelines.get("y").unwrap();
    assert_eq!(tl_y.segments[0].start_time, delta_ms(1000));
    assert_eq!(tl_y.segments[0].end_time, delta_ms(2000));
}
