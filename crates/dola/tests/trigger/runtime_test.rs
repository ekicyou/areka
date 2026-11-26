//! トリガー機能のテスト — tick() トリガー実行統合テスト

use super::common::{at_ms, delta_ms, ms, timed_trigger_doc};
use dola::runtime::{DolaRuntime, RuntimeError, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, InterruptionPolicy, StoryboardBuilder,
    StoryboardEntry, TimeSpan, TransitionDef, TransitionRef, TransitionValue,
};

// ============================================================
// tick() トリガー実行統合テスト
// ============================================================

/// 基本トリガー: 発火で子SBが自動開始される
#[test]
fn trigger_starts_child_storyboard() {
    let doc = timed_trigger_doc("parent", "child", None);

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();

    // 親開始
    let parent_result = rt.start("parent", at_ms(0)).unwrap();
    assert_eq!(parent_result.group_id, 1);

    // t=0.5: 発火前 — トリガーなし
    rt.tick(at_ms(500));
    let result = rt.last_result().clone();
    assert!(
        result.triggered.is_empty(),
        "no triggers should fire at t=0.5"
    );

    // t=1.0: kf1 到達 → トリガー発火
    rt.tick(at_ms(1000));
    let result = rt.last_result().clone();
    assert_eq!(
        result.triggered.len(),
        1,
        "one trigger should fire at t=1.0"
    );
    match &result.triggered[0] {
        TriggerResult::Started {
            source_storyboard,
            target_storyboard,
            start_result,
        } => {
            assert_eq!(source_storyboard, "parent");
            assert_eq!(target_storyboard, "child");
            assert!(start_result.group_id > parent_result.group_id);
        }
        TriggerResult::Error { error, .. } => {
            panic!("trigger should succeed, got error: {error}");
        }
    }
}

/// 同じトリガーは1周回で1回のみ発火（親は発火の後も走り続ける）
#[test]
fn trigger_fires_only_once_per_loop() {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };
    // 親は 2 秒: 0→1（1 秒・kf1）の後に 1→0（1 秒）。トリガーは kf1（1.0）
    let parent_sb = StoryboardBuilder::new()
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
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(1.0)),
                to: Some(TransitionValue::Scalar(0.0)),
                duration: Some(ms(1000)),
                ..Default::default()
            })),
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

    // t=1.0: 初回発火
    rt.tick(at_ms(1000));
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1);

    // t=1.016（次のフレーム）: 親はまだ走っているが再発火しない
    rt.tick(at_ms(1016));
    let result = rt.last_result().clone();
    assert!(
        result.triggered.is_empty(),
        "trigger should not re-fire in same loop"
    );
}

/// 親の始まりが 0 でなくても、発火は始まり＋キーフレームの時刻
#[test]
fn trigger_fires_at_parent_start_plus_keyframe() {
    let doc = timed_trigger_doc("parent", "child", None);

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("parent", at_ms(10000)).unwrap();

    rt.tick(at_ms(10900));
    assert!(rt.last_result().triggered.is_empty(), "not yet at t=10.9");
    rt.tick(at_ms(11000));
    assert_eq!(rt.last_result().triggered.len(), 1, "fires at t=11.0");
}

/// 一時停止の間は発火の時刻も後ろへずれる
#[test]
fn trigger_fire_time_counts_pause() {
    let doc = timed_trigger_doc("parent", "child", None);

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    let gid = rt.start("parent", at_ms(0)).unwrap().group_id;

    rt.pause(gid, at_ms(500)).unwrap();
    rt.resume(gid, at_ms(5000)).unwrap();

    rt.tick(at_ms(5400));
    assert!(rt.last_result().triggered.is_empty(), "not yet at t=5.4");
    rt.tick(at_ms(5500));
    assert_eq!(rt.last_result().triggered.len(), 1, "fires at t=5.5");
}

/// trigger_start_offset の検証
#[test]
fn trigger_with_start_offset() {
    let doc = timed_trigger_doc("parent", "child", Some(delta_ms(500)));

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("parent", at_ms(0)).unwrap();

    // t=1.0: 発火。子SBの start_time = fire_time(1.0) + offset(0.5) = 1.5
    rt.tick(at_ms(1000));
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1);
    match &result.triggered[0] {
        TriggerResult::Started { start_result, .. } => {
            // 子SB end_time = start_time(1.5) + duration(1.0) = 2.5
            assert_eq!(
                start_result.end_time,
                Some(at_ms(2500)),
                "child end_time should be 2.5s"
            );
        }
        other => panic!("expected Started, got {other:?}"),
    }
}

/// 子の起動が Never の競合で拒まれると、トリガーは `TriggerResult::Error` を返す
/// （dola は記録しない・記録は呼び手の境目）。
#[test]
fn trigger_error_on_never_conflict() {
    let var = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    let ramp = |variable: &str, duration: TimeSpan| StoryboardEntry {
        variable: Some(variable.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            duration: Some(duration),
            ..Default::default()
        })),
        ..Default::default()
    };
    // blocker（Never）が x を 10 秒握る。parent は y を動かし t=1.0 で x を触る child を起こす
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", var())
        .variable("y", var())
        .storyboard(
            "blocker",
            StoryboardBuilder::new()
                .interruption_policy(InterruptionPolicy::Never)
                .entry(ramp("x", ms(10_000)))
                .build(),
        )
        .storyboard(
            "parent",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    keyframe: Some("kf1".to_string()),
                    ..ramp("y", ms(1000))
                })
                .entry(StoryboardEntry {
                    trigger_storyboard: Some("child".to_string()),
                    at: Some(dola::KeyframeRef::Single("kf1".to_string())),
                    ..Default::default()
                })
                .build(),
        )
        .storyboard(
            "child",
            StoryboardBuilder::new().entry(ramp("x", ms(1000))).build(),
        )
        .build()
        .expect("doc should be valid");

    let mut rt = DolaRuntime::new();
    rt.load_document(doc).unwrap();
    let blocker = rt.start("blocker", at_ms(0)).unwrap();
    rt.start("parent", at_ms(0)).unwrap();

    rt.tick(at_ms(1000));
    match rt.last_result().triggered.as_slice() {
        [
            TriggerResult::Error {
                source_storyboard,
                target_storyboard,
                error:
                    RuntimeError::Conflict {
                        conflicting_group_ids,
                    },
            },
        ] => {
            assert_eq!(source_storyboard, "parent");
            assert_eq!(target_storyboard, "child");
            assert_eq!(conflicting_group_ids, &vec![blocker.group_id]);
        }
        other => panic!("expected one Conflict TriggerResult::Error, got {other:?}"),
    }
}
