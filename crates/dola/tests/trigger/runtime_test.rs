//! トリガー機能のテスト — 5.4: update() トリガー実行統合テスト,
//! 5.5: ループ内トリガー統合テスト, 5.6: E2E テスト

use super::common::timed_trigger_doc;
use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, InterruptionPolicy, StoryboardBuilder,
    StoryboardEntry, TransitionDef, TransitionRef, TransitionValue,
};

// ============================================================
// 5.4: update() トリガー実行統合テスト
// ============================================================

/// 基本トリガー: 発火で子SBが自動開始される
#[test]
fn trigger_starts_child_storyboard() {
    let doc = timed_trigger_doc("parent", "child", None);

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();

    // 親開始
    let parent_result = rt.start("parent", 0.0).unwrap();
    assert_eq!(parent_result.group_id, 1);

    // t=0.5: 発火前 — トリガーなし
    rt.tick(0.5);
    let result = rt.last_result().clone();
    assert!(
        result.triggered.is_empty(),
        "no triggers should fire at t=0.5"
    );

    // t=1.0: kf1 到達 → トリガー発火
    rt.tick(1.0);
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
                duration: Some(1.0),
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
                duration: Some(1.0),
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
                duration: Some(0.5),
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
    rt.start("parent", 0.0).unwrap();

    // t=1.0: 初回発火
    rt.tick(1.0);
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1);

    // t=1.016（次のフレーム）: 親はまだ走っているが再発火しない
    rt.tick(1.016);
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
    rt.start("parent", 10.0).unwrap();

    rt.tick(10.9);
    assert!(rt.last_result().triggered.is_empty(), "not yet at t=10.9");
    rt.tick(11.0);
    assert_eq!(rt.last_result().triggered.len(), 1, "fires at t=11.0");
}

/// 一時停止の間は発火の時刻も後ろへずれる
#[test]
fn trigger_fire_time_counts_pause() {
    let doc = timed_trigger_doc("parent", "child", None);

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    let gid = rt.start("parent", 0.0).unwrap().group_id;

    rt.pause(gid, 0.5).unwrap();
    rt.resume(gid, 5.0).unwrap();

    rt.tick(5.4);
    assert!(rt.last_result().triggered.is_empty(), "not yet at t=5.4");
    rt.tick(5.5);
    assert_eq!(rt.last_result().triggered.len(), 1, "fires at t=5.5");
}

/// trigger_start_offset の検証
#[test]
fn trigger_with_start_offset() {
    let doc = timed_trigger_doc("parent", "child", Some(0.5));

    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("parent", 0.0).unwrap();

    // t=1.0: 発火。子SBの start_time = fire_time(1.0) + offset(0.5) = 1.5
    rt.tick(1.0);
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1);
    match &result.triggered[0] {
        TriggerResult::Started { start_result, .. } => {
            // 子SB end_time = start_time(1.5) + duration(1.0) = 2.5
            assert_eq!(
                start_result.end_time, 2.5,
                "child end_time should be 2.5, got {}",
                start_result.end_time
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
    let ramp = |variable: &str, duration: f64| StoryboardEntry {
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
                .entry(ramp("x", 10.0))
                .build(),
        )
        .storyboard(
            "parent",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    keyframe: Some("kf1".to_string()),
                    ..ramp("y", 1.0)
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
            StoryboardBuilder::new().entry(ramp("x", 1.0)).build(),
        )
        .build()
        .expect("doc should be valid");

    let mut rt = DolaRuntime::new();
    rt.load_document(doc).unwrap();
    let blocker = rt.start("blocker", 0.0).unwrap();
    rt.start("parent", 0.0).unwrap();

    rt.tick(1.0);
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

// ============================================================
// 5.5: ループ内トリガー統合テスト
// ============================================================

/// loop_count=2 のSBでトリガーが各周回で再発火する
#[test]
fn trigger_refires_each_loop() {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };

    let parent_sb = StoryboardBuilder::new()
        .loop_count(2)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            keyframe: Some("kf1".to_string()),
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
                duration: Some(0.5),
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
    rt.start("parent", 0.0).unwrap();

    // 周回1: t=1.0 でトリガー発火
    rt.tick(1.0);
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "loop 1: trigger should fire");

    // 周回2: t=2.0 でトリガー再発火
    rt.tick(2.0);
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "loop 2: trigger should re-fire");
}

/// 無限ループ (-1) でもトリガーが各周回で発火
#[test]
fn infinite_loop_trigger_fires_each_cycle() {
    let opacity_var = AnimationVariableDef::Float {
        initial: 0.0,
        min: Some(0.0),
        max: Some(1.0),
    };

    let parent_sb = StoryboardBuilder::new()
        .loop_count(-1)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(1.0),
                ..Default::default()
            })),
            keyframe: Some("kf1".to_string()),
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
                duration: Some(0.5),
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
    rt.start("parent", 0.0).unwrap();

    // 3 周回分チェック
    for cycle in 1..=3 {
        let t = cycle as f64;
        rt.tick(t);
        let result = rt.last_result().clone();
        assert_eq!(
            result.triggered.len(),
            1,
            "cycle {cycle}: trigger should fire once at t={t}"
        );
    }
}

// ============================================================
// 5.6: E2E テスト
// ============================================================

/// トリガーチェーン A→B→C の3段連鎖起動
#[test]
fn trigger_chain_a_b_c() {
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
                duration: Some(0.5),
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
                duration: Some(0.5),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();

    let doc = DolaDocumentBuilder::new("1.0")
        .variable("opacity", opacity_var)
        .storyboard("sb_a", sb_a)
        .storyboard("sb_b", sb_b)
        .storyboard("sb_c", sb_c)
        .build()
        .expect("chain doc should be valid");

    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();

    // start A
    let a_result = rt.start("sb_a", 0.0).unwrap();
    assert_eq!(a_result.group_id, 1);

    // t=0.0: A はトリガーのみ → B が起動される
    rt.tick(0.0);
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "A should trigger B at t=0.0");
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

    // t=0.5: B の kf1 到達 → C が起動される
    rt.tick(0.5);
    let result = rt.last_result().clone();
    assert_eq!(result.triggered.len(), 1, "B should trigger C at t=0.5");
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

    // t=1.0: C が完了し、opacity の最終値 1.0 が配られる
    rt.tick(1.0);
    let result = rt.last_result().clone();
    let (_, val) = result
        .changes
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("C delivers opacity at t=1.0");
    assert_eq!(*val, EvaluatedValue::Float(1.0), "opacity at t=1.0");
}

/// 親を cancel しても、親が起こした子は止まらず自分の変数を動かし続ける（ライフサイクル独立性）
#[test]
fn parent_cancel_does_not_affect_child() {
    let fade = |var: &str, duration: f64| StoryboardEntry {
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
                .entry(fade("a", 2.0))
                .build(),
        )
        .storyboard(
            "child",
            StoryboardBuilder::new().entry(fade("b", 1.0)).build(),
        )
        .build()
        .unwrap();

    let mut rt = DolaRuntime::new();
    let _a_id = rt.subscribe("a");
    let b_id = rt.subscribe("b");
    rt.load_document(doc).unwrap();
    let parent_gid = rt.start("parent", 0.0).unwrap().group_id;

    rt.tick(0.0);
    assert_eq!(rt.last_result().triggered.len(), 1, "child starts at t=0");

    // 親がまだ走っているうちに親だけを cancel する
    rt.cancel(parent_gid).unwrap();

    // 子は b を動かし続け、cancel した親の a はもう配られない
    rt.tick(0.5);
    assert_eq!(
        rt.last_result().changes,
        vec![(b_id, EvaluatedValue::Float(0.5))]
    );
    rt.tick(1.0);
    assert_eq!(
        rt.last_result().changes,
        vec![(b_id, EvaluatedValue::Float(1.0))]
    );
}

/// 同じ tick で 2 つの親が同じ x を触る子を起こしても、処理は親の group_id の順（#36）。
/// 走るたびに新しい DolaRuntime（＝新しい表）で組み直し、20 回とも同じ結果になる
#[test]
fn simultaneous_triggers_resolve_in_group_order() {
    let fade = |var: &str, keyframe: Option<&str>| StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            duration: Some(1.0),
            ..Default::default()
        })),
        keyframe: keyframe.map(str::to_string),
        ..Default::default()
    };
    // 親 P1・P2 は別の変数を触り（互いに競合しない）、t=1 でそれぞれ C1・C2 を起こす
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
        assert_eq!(rt.start("p1", 0.0).unwrap().group_id, 1);
        assert_eq!(rt.start("p2", 0.0).unwrap().group_id, 2);
        rt.tick(1.0);
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
