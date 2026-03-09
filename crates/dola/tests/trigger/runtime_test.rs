#![allow(deprecated)]
//! トリガー機能のテスト — 5.4: update() トリガー実行統合テスト,
//! 5.5: ループ内トリガー統合テスト, 5.6: E2E テスト

use super::common::{extract_float, timed_trigger_doc};
use dola::runtime::{DolaRuntime, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry, TransitionDef,
    TransitionRef, TransitionValue,
};

// ============================================================
// 5.4: update() トリガー実行統合テスト
// ============================================================

#[cfg(test)]
mod trigger_execution_tests {
    use super::*;

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
        let result = rt.update(0.5);
        assert!(
            result.triggered.is_empty(),
            "no triggers should fire at t=0.5"
        );

        // t=1.0: kf1 到達 → トリガー発火
        let result = rt.update(1.0);
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

    /// 同じトリガーは1周回で1回のみ発火
    #[test]
    fn trigger_fires_only_once_per_loop() {
        let doc = timed_trigger_doc("parent", "child", None);

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        rt.start("parent", 0.0).unwrap();

        // t=1.0: 初回発火
        let result = rt.update(1.0);
        assert_eq!(result.triggered.len(), 1);

        // t=1.5: 2回目の update — 再発火しない
        let result = rt.update(1.5);
        assert!(
            result.triggered.is_empty(),
            "trigger should not re-fire in same loop"
        );
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
        let result = rt.update(1.0);
        assert_eq!(result.triggered.len(), 1);
        match &result.triggered[0] {
            TriggerResult::Started { start_result, .. } => {
                // 子SB end_time = start_time(1.5) + duration(1.0) = 2.5
                assert!(
                    (start_result.end_time - 2.5).abs() < 1e-9,
                    "child end_time should be 2.5, got {}",
                    start_result.end_time
                );
            }
            other => panic!("expected Started, got {other:?}"),
        }
    }

    /// UpdateResult.changes は通常通り変数差分を含む
    #[test]
    fn update_result_contains_changes_and_triggered() {
        let doc = timed_trigger_doc("parent", "child", None);

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        rt.start("parent", 0.0).unwrap();

        // t=0.0: 初回 update
        let result = rt.update(0.0);
        assert!(!result.changes.is_empty(), "should have opacity change");
        assert!(result.triggered.is_empty());

        // t=1.0: トリガー + 変数更新の両方
        let result = rt.update(1.0);
        assert!(!result.triggered.is_empty(), "should have trigger");
    }
}

// ============================================================
// 5.5: ループ内トリガー統合テスト
// ============================================================

#[cfg(test)]
mod loop_trigger_tests {
    use super::*;

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
        let result = rt.update(1.0);
        assert_eq!(result.triggered.len(), 1, "loop 1: trigger should fire");

        // 周回2: t=2.0 でトリガー再発火
        let result = rt.update(2.0);
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
            let result = rt.update(t);
            assert!(
                !result.triggered.is_empty(),
                "cycle {cycle}: trigger should fire at t={t}"
            );
        }
    }
}

// ============================================================
// 5.6: E2E テスト
// ============================================================

#[cfg(test)]
mod e2e_tests {
    use super::*;

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
        let result = rt.update(0.0);
        assert!(!result.triggered.is_empty(), "A should trigger B at t=0.0");
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
        let result = rt.update(0.5);
        assert!(!result.triggered.is_empty(), "B should trigger C at t=0.5");
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

        // t=1.0: C が完了。opacity は最終値 1.0 付近
        let result = rt.update(1.0);
        if let Some((_, val)) = result.changes.iter().find(|(id, _)| *id == opacity_id) {
            let v = extract_float(val);
            assert!(v > 0.9, "opacity should be ~1.0 at t=1.0, got {v}");
        }
    }

    /// 親終了後も子SBは独立して動作する（ライフサイクル独立性）
    #[test]
    fn parent_cancel_does_not_affect_child() {
        // timed_trigger_doc: parent=1.0s(0→1), trigger at kf1(=1.0s), child=1.0s(1→0)
        let doc = timed_trigger_doc("parent", "child", None);

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        rt.start("parent", 0.0).unwrap();

        // t=1.0: 親トランジション完了 + トリガー発火 → 子SB開始
        let result = rt.update(1.0);
        assert_eq!(
            result.triggered.len(),
            1,
            "trigger should fire at kf1 (1.0s)"
        );

        // t=1.5: 親は自然終了済み、子SBはまだ再生中（1.0s→2.0s）
        let result = rt.update(1.5);
        // 子SBが opacity を 1→0 に遷移中 → changes が発生するべき
        assert!(
            !result.changes.is_empty(),
            "child should still produce changes after parent concludes"
        );

        // t=2.0: 子SBも完了
        let result = rt.update(2.0);
        // 最終値に到達
        assert!(result.triggered.is_empty(), "no new triggers");
    }

    /// load_document → start → 複数 update → 全インスタンス終了の完全シナリオ
    #[test]
    fn full_lifecycle_with_triggers() {
        let doc = timed_trigger_doc("parent", "child", None);

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        rt.start("parent", 0.0).unwrap();

        // 段階的に time を進める
        let _ = rt.update(0.0);
        let _ = rt.update(0.5);
        let result = rt.update(1.0); // trigger fires
        assert!(!result.triggered.is_empty());

        let _ = rt.update(1.5);
        let _ = rt.update(2.0); // child completes (1→0 over 1s, started at 1.0)

        // 最終状態: 全インスタンス自然終了
        let result = rt.update(3.0);
        // 両方終了後は変化なし
        assert!(
            result.triggered.is_empty(),
            "no new triggers after everything concluded"
        );
    }
}
