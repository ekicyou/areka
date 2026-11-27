//! トリガー機能のテスト — トリガーの連鎖・親の終了と子の独立・同時発火の順序

use super::common::{at_ms, ms};
use dola::runtime::{DolaRuntime, EvaluatedValue, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry, TimeSpan,
    TransitionDef, TransitionRef, TransitionValue,
};

// ============================================================
// E2E テスト
// ============================================================

/// A（トリガーだけ）→ B（500 ms・終わりで C）→ C（500 ms）の文書
fn chain_abc_doc() -> dola::DolaDocument {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };

    // SB_A: trigger → SB_B at start
    let sb_a = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            trigger_storyboard: Some("sb_b".to_string()),
            ..Default::default()
        })
        .build();

    // SB_B: opacity 0→0.5 (0.5s) + trigger → SB_C at kf1
    let sb_b = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(0.5)),
                duration: Some(ms(500)),
                ..Default::default()
            })),
            keyframe: Some("kf1".to_string()),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some("sb_c".to_string()),
            at: Some(dola::KeyframeRef::Single("kf1".to_string())),
            ..Default::default()
        })
        .build();

    // SB_C: opacity 0.5→1.0 (0.5s)
    let sb_c = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.5)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(500)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();

    DolaDocumentBuilder::new("1.0")
        .variable("opacity", opacity_var)
        .storyboard("sb_a", sb_a)
        .storyboard("sb_b", sb_b)
        .storyboard("sb_c", sb_c)
        .build()
        .expect("chain doc should be valid")
}

/// トリガーチェーン A→B→C の3段連鎖起動
#[test]
fn trigger_chain_a_b_c() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(chain_abc_doc()).unwrap();

    // start A
    let a_result = rt.start("sb_a", at_ms(0)).unwrap();
    assert_eq!(a_result.group_id, 1);

    // t=0ms: A はトリガーのみ → B が起動される
    rt.tick(at_ms(0));
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "A should trigger B at t=0ms");
    let b_group_id = match &result.triggered[0] {
        TriggerResult::Started {
            start_result,
            target_storyboard,
            ..
        } => {
            assert_eq!(target_storyboard, "sb_b");
            start_result.group_id
        }
        other => panic!("expected Started, got {other:?}"),
    };
    assert!(b_group_id > a_result.group_id);

    // t=500ms: B の kf1 到達 → C が起動される
    rt.tick(at_ms(500));
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "B should trigger C at t=500ms");
    let c_target = match &result.triggered[0] {
        TriggerResult::Started {
            target_storyboard,
            start_result,
            ..
        } => {
            assert_eq!(target_storyboard, "sb_c");
            start_result.group_id
        }
        other => panic!("expected Started for C, got {other:?}"),
    };
    assert!(c_target > b_group_id);

    // t=1000ms: C が完了し、opacity の最終値 1.0 が配られる
    rt.tick(at_ms(1000));
    let result = rt.last_result().clone();
    let (_, val) = result
        .changes
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("C delivers opacity at t=1000ms");
    assert_eq!(*val, EvaluatedValue::Float(1.0), "opacity at t=1000ms");
}

/// 起こした子がその tick で終わっても、子のトリガーは同じ tick で発火する（初めの tick がコマ落ちで 600 ms）
#[test]
fn trigger_chain_a_b_c_first_tick_late() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(chain_abc_doc()).unwrap();
    rt.start("sb_a", at_ms(0)).unwrap();

    // B は 0〜500 ms でこの tick に終わる。C は B の終わりの 500 ms に始まる
    rt.tick(at_ms(600));
    let result = rt.last_result().clone();
    let targets: Vec<&str> = result
        .triggered
        .iter()
        .map(|r| match r {
            TriggerResult::Started {
                target_storyboard, ..
            } => target_storyboard.as_str(),
            other => panic!("expected Started, got {other:?}"),
        })
        .collect();
    assert_eq!(targets, ["sb_b", "sb_c"]);
    let (_, val) = result
        .changes
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("C delivers opacity at t=600ms");
    match val {
        EvaluatedValue::Float(v) => assert!((v - 0.6).abs() < 1e-9, "opacity at t=600ms: {v}"),
        other => panic!("expected Float, got {other:?}"),
    }
}

/// 長さ 0 の B を挟んだ A→B→C が 1 回の tick で C まで起きる
#[test]
fn trigger_chain_through_zero_length_child() {
    let trigger_only = |target: &str| {
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some(target.to_string()),
                ..Default::default()
            })
            .build()
    };
    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .storyboard("sb_a", trigger_only("sb_b"))
        .storyboard("sb_b", trigger_only("sb_c"))
        .storyboard(
            "sb_c",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        to: Some(TransitionValue::Scalar(1.0)),
                        duration: Some(ms(1000)),
                        ..Default::default()
                    })),
                    ..Default::default()
                })
                .build(),
        )
        .build()
        .expect("chain doc should be valid");

    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("sb_a", at_ms(0)).unwrap();

    rt.tick(at_ms(0));
    let started: Vec<&str> = rt
        .last_result()
        .triggered
        .iter()
        .filter_map(|r| match r {
            TriggerResult::Started {
                target_storyboard, ..
            } => Some(target_storyboard.as_str()),
            TriggerResult::Error { .. } => None,
        })
        .collect();
    assert_eq!(started, ["sb_b", "sb_c"]);

    rt.tick(at_ms(500));
    let (_, val) = rt
        .last_result()
        .changes
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("C moves opacity at t=500ms");
    assert_eq!(*val, EvaluatedValue::Float(0.5));
}

/// 親を cancel しても、親が起こした子は止まらず自分の変数を動かし続ける（ライフサイクル独立性）
#[test]
fn parent_cancel_does_not_affect_child() {
    let fade = |var: &str, duration: TimeSpan| StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            duration: Some(duration),
            ..Default::default()
        })),
        ..Default::default()
    };
    let float = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    // 親は a を 2 秒かけて動かし、始まりで子を起こす。子は b を 1 秒かけて動かす
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("a", float())
        .variable("b", float())
        .storyboard(
            "parent",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    trigger_storyboard: Some("child".to_string()),
                    ..Default::default()
                })
                .entry(fade("a", ms(2000)))
                .build(),
        )
        .storyboard(
            "child",
            StoryboardBuilder::new().entry(fade("b", ms(1000))).build(),
        )
        .build()
        .unwrap();

    let mut rt = DolaRuntime::new();
    let _a_id = rt.subscribe("a");
    let b_id = rt.subscribe("b");
    rt.load_document(doc).unwrap();
    let parent_gid = rt.start("parent", at_ms(0)).unwrap().group_id;

    rt.tick(at_ms(0));
    assert_eq!(rt.last_result().triggered.len(), 1, "child starts at t=0");

    // 親がまだ走っているうちに親だけを cancel する
    rt.cancel(parent_gid).unwrap();

    // 子は b を動かし続け、cancel した親の a はもう配られない
    rt.tick(at_ms(500));
    assert_eq!(
        rt.last_result().changes,
        vec![(b_id, EvaluatedValue::Float(0.5))]
    );
    rt.tick(at_ms(1000));
    assert_eq!(
        rt.last_result().changes,
        vec![(b_id, EvaluatedValue::Float(1.0))]
    );
}

/// 同じ tick で 2 つの親が同じ x を触る子を起こしても、処理は親の group_id の順。
/// 走るたびに新しい DolaRuntime（＝新しい表）で組み直し、20 回とも同じ結果になる
#[test]
fn simultaneous_triggers_resolve_in_group_order() {
    let fade = |var: &str, keyframe: Option<&str>| StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            duration: Some(ms(1000)),
            ..Default::default()
        })),
        keyframe: keyframe.map(str::to_string),
        ..Default::default()
    };
    // 親 p1・p2 は別の変数を触り（互いに競合しない）、t=1 でそれぞれ c1・c2 を起こす
    let parent = |var: &str, child: &str| {
        StoryboardBuilder::new()
            .entry(fade(var, Some("kf1")))
            .entry(StoryboardEntry {
                trigger_storyboard: Some(child.to_string()),
                at: Some(dola::KeyframeRef::Single("kf1".to_string())),
                ..Default::default()
            })
            .build()
    };
    let float = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("a", float())
        .variable("b", float())
        .variable("x", float())
        .storyboard("p1", parent("a", "c1"))
        .storyboard("p2", parent("b", "c2"))
        .storyboard(
            "c1",
            StoryboardBuilder::new().entry(fade("x", None)).build(),
        )
        .storyboard(
            "c2",
            StoryboardBuilder::new().entry(fade("x", None)).build(),
        )
        .build()
        .unwrap();

    for _ in 0..20 {
        let mut rt = DolaRuntime::new();
        rt.load_document(doc.clone()).unwrap();
        assert_eq!(rt.start("p1", at_ms(0)).unwrap().group_id, 1);
        assert_eq!(rt.start("p2", at_ms(0)).unwrap().group_id, 2);
        rt.tick(at_ms(1000));
        let started: Vec<(String, u64, Vec<u64>)> = rt
            .last_result()
            .triggered
            .iter()
            .map(|t| match t {
                TriggerResult::Started {
                    target_storyboard,
                    start_result,
                    ..
                } => (
                    target_storyboard.clone(),
                    start_result.group_id,
                    start_result.affected_group_ids.clone(),
                ),
                TriggerResult::Error { error, .. } => panic!("trigger failed: {error}"),
            })
            .collect();
        // C1 が先に 3 を取り、後の C2（4）が C1 を終わらせて残る
        assert_eq!(
            started,
            vec![
                ("c1".to_string(), 3, vec![]),
                ("c2".to_string(), 4, vec![3]),
            ]
        );
    }
}
