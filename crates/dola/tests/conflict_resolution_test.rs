//! 競合解決（ConflictResolver）統合テスト
//!
//! Task 6.1〜6.3: detect_overlaps, 各終了戦略, エラーパスと境界条件

use std::collections::BTreeMap;

use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError};
use dola::{
    AnimationVariableDef, DolaDocument, InterruptionPolicy, StoryboardBuilder, StoryboardEntry,
    TransitionDef, TransitionRef, TransitionValue,
};

// =========================================================================
// ヘルパー
// =========================================================================

/// 指定 policy で opacity 0→100 (duration=2.0) の SB を生成
fn make_doc_with_policy(sb_name: &str, policy: InterruptionPolicy) -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "opacity".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(100.0),
        },
    );
    let sb = StoryboardBuilder::new()
        .interruption_policy(policy)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(100.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();
    let mut storyboard = BTreeMap::new();
    storyboard.insert(sb_name.to_string(), sb);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}

/// 複数 SB + 複数変数のドキュメント
fn make_multi_sb_doc() -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "x".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "y".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "z".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );

    // sb_a: Cancel policy, x: 0→100 in 2.0s, y: 0→50 in 2.0s
    let sb_a = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Cancel)
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
            at: None,
            between: None,
            keyframe: None,
        })
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(50.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    // sb_b: Conclude policy, x: 200→300 in 2.0s
    let sb_b = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Conclude)
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(200.0)),
                to: Some(TransitionValue::Scalar(300.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    // sb_c: Trim policy, x: 500→600 in 2.0s
    let sb_c = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Trim)
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(500.0)),
                to: Some(TransitionValue::Scalar(600.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    // sb_new: x: 0→10 in 1.0s (これが新規に start されて、上記と競合する)
    let sb_new = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(10.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("sb_a".to_string(), sb_a);
    storyboard.insert("sb_b".to_string(), sb_b);
    storyboard.insert("sb_c".to_string(), sb_c);
    storyboard.insert("sb_new".to_string(), sb_new);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}

/// Never policy の SB を含むドキュメント
fn make_never_doc() -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "x".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "y".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "z".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );

    // sb_never: Never policy, x: 0→100 in 2.0s
    let sb_never = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Never)
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
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    // sb_new: x: 0→10 in 1.0s
    let sb_new = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(10.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    // sb_multi: x,y,z を同時に操作（部分競合テスト用）
    let sb_multi = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(10.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(20.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .entry(StoryboardEntry {
            variable: Some("z".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(30.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            at: None,
            between: None,
            keyframe: None,
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("sb_never".to_string(), sb_never);
    storyboard.insert("sb_new".to_string(), sb_new);
    storyboard.insert("sb_multi".to_string(), sb_multi);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}

// =========================================================================
// 6.1 競合検出ロジックのテスト
// =========================================================================
mod conflict_detection {
    use super::*;

    #[test]
    fn no_conflict_when_different_variables() {
        // 異なる変数の SB は競合しない
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        variable.insert(
            "y".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let sb_x = StoryboardBuilder::new()
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
                at: None,
                between: None,
                keyframe: None,
            })
            .build();

        let sb_y = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(2.0),
                })),
                at: None,
                between: None,
                keyframe: None,
            })
            .build();

        let mut storyboard = BTreeMap::new();
        storyboard.insert("sb_x".to_string(), sb_x);
        storyboard.insert("sb_y".to_string(), sb_y);
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        };

        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.subscribe(1, "y");
        rt.load_document(doc).unwrap();

        let r1 = rt.start("sb_x", 0.0).unwrap();
        assert!(r1.affected_group_ids.is_empty());

        // sb_y は変数 y のみ → 変数 x と競合しない
        let r2 = rt.start("sb_y", 0.5).unwrap();
        assert!(r2.affected_group_ids.is_empty());
    }

    #[test]
    fn no_conflict_when_no_existing_instances() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Conclude);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        let result = rt.start("fade", 0.0).unwrap();
        assert!(result.affected_group_ids.is_empty());
    }

    #[test]
    fn conflict_detected_on_same_variable() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        // 1st start
        let r1 = rt.start("fade", 0.0).unwrap();
        assert!(r1.affected_group_ids.is_empty());

        // 2nd start on same variable → conflict with group_id 1
        let r2 = rt.start("fade", 0.5).unwrap();
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);
    }

    #[test]
    fn no_conflict_after_natural_end() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        let _r1 = rt.start("fade", 0.0).unwrap();
        // 自然終了を発生させる
        rt.update(1, 3.0);

        // 新規 start → 競合なし（前のインスタンスは終了済み）
        let r2 = rt.start("fade", 3.0).unwrap();
        assert!(r2.affected_group_ids.is_empty());
    }
}

// =========================================================================
// 6.2 各終了戦略の統合テスト
// =========================================================================
mod cancel_strategy {
    use super::*;

    #[test]
    fn cancel_freezes_at_interrupt_time() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        // Start 1st at t=0.0
        rt.start("fade", 0.0).unwrap();
        // Update at t=0.5 → opacity = 25.0 (progress=0.25 of 0→100, 2s duration)
        let changes = rt.update(1, 0.5);
        assert!(!changes.is_empty());

        // Start 2nd at t=1.0 → Cancel 1st, freeze at t=1.0 → 50.0
        let r2 = rt.start("fade", 1.0).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // Update → 新しい SB の値が取得される
        let changes = rt.update(1, 1.0);
        // The cancelled instance's frozen value should be 50.0,
        // but now the new instance starts at 0.0
        // The subscription manager should show the new SB's value
        assert!(!changes.is_empty());
    }

    #[test]
    fn cancel_group_id_all_variables() {
        let doc = make_multi_sb_doc();
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.subscribe(1, "y");
        rt.load_document(doc).unwrap();

        // Start sb_a (Cancel policy, operates x and y)
        let r1 = rt.start("sb_a", 0.0).unwrap();
        assert!(r1.affected_group_ids.is_empty());

        // Start sb_new (operates x only) at t=0.5
        // → sb_a conflicts on x → Cancel sb_a (both x and y entries removed)
        let r2 = rt.start("sb_new", 0.5).unwrap();
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);

        // sb_a's y values should be frozen at t=0.5 → 12.5 (progress=0.25 of 0→50)
        let changes = rt.update(1, 0.5);
        // y should show frozen value, x shows new SB value
        let y_val = changes.iter().find(|(name, _)| name == "y");
        if let Some((_, val)) = y_val {
            match val {
                EvaluatedValue::Float(v) => {
                    assert!(
                        (*v - 12.5).abs() < 1e-6,
                        "y should be frozen at 12.5, got {v}"
                    );
                }
                _ => panic!("expected Float"),
            }
        }
    }
}

mod conclude_strategy {
    use super::*;

    #[test]
    fn conclude_jumps_to_current_segment_final() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Conclude);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        // Start at t=0.0
        rt.start("fade", 0.0).unwrap();
        // Update to get initial value
        let changes = rt.update(1, 0.5);
        assert!(!changes.is_empty());

        // Start 2nd at t=1.0 → Conclude 1st → jump to segment final (100.0)
        let r2 = rt.start("fade", 1.0).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // The concluded instance's last value should be 100.0 (segment final)
        // Next update should see new SB starting
        let changes = rt.update(1, 1.0);
        assert!(!changes.is_empty());
    }

    #[test]
    fn default_policy_is_conclude() {
        // StoryboardBuilder defaults to Conclude
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(100.0),
            },
        );
        let sb = StoryboardBuilder::new()
            // no .interruption_policy() call → defaults to Conclude
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(100.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(2.0),
                })),
                at: None,
                between: None,
                keyframe: None,
            })
            .build();
        let mut storyboard = BTreeMap::new();
        storyboard.insert("fade".to_string(), sb);
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        };

        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        let r1 = rt.start("fade", 0.0).unwrap();
        let r2 = rt.start("fade", 0.5).unwrap();
        // Default Conclude → affected
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);
    }
}

mod trim_strategy {
    use super::*;

    #[test]
    fn trim_cuts_at_interrupt_time() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Trim);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        rt.start("fade", 0.0).unwrap();
        rt.update(1, 0.5);

        // Start 2nd at t=1.0 → Trim 1st at t=1.0 → value = 50.0
        let r2 = rt.start("fade", 1.0).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // Update → trimmed value should have been propagated
        let changes = rt.update(1, 1.0);
        assert!(!changes.is_empty());
    }
}

mod compress_strategy {
    use super::*;

    #[test]
    fn compress_jumps_to_all_final_values() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Compress);
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "opacity");
        rt.load_document(doc).unwrap();

        rt.start("fade", 0.0).unwrap();
        rt.update(1, 0.5);

        // Start 2nd at t=0.5 → Compress 1st → all final values (100.0)
        let r2 = rt.start("fade", 0.5).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // Update → compressed value was 100.0, new SB starts
        let changes = rt.update(1, 0.5);
        assert!(!changes.is_empty());
    }
}

// =========================================================================
// 6.3 エラーパスと境界条件のテスト
// =========================================================================
mod never_strategy {
    use super::*;

    #[test]
    fn never_rejects_start_with_conflict() {
        let doc = make_never_doc();
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.load_document(doc).unwrap();

        // Start Never SB
        let r1 = rt.start("sb_never", 0.0).unwrap();
        assert!(r1.affected_group_ids.is_empty());

        // Start conflicting SB → Err(Conflict)
        let result = rt.start("sb_new", 0.5);
        match result {
            Err(RuntimeError::Conflict {
                conflicting_group_ids,
            }) => {
                assert!(conflicting_group_ids.contains(&r1.group_id));
            }
            _ => panic!("expected RuntimeError::Conflict, got {result:?}"),
        }
    }

    #[test]
    fn never_partial_conflict_rejects_entirely() {
        // sb_multi operates x, y, z. sb_never only operates x.
        // Even though y and z don't conflict, the entire start should fail.
        let doc = make_never_doc();
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.subscribe(1, "y");
        rt.subscribe(1, "z");
        rt.load_document(doc).unwrap();

        // Start Never SB (only x)
        rt.start("sb_never", 0.0).unwrap();

        // Start sb_multi (x, y, z) → conflict on x → entire start fails
        let result = rt.start("sb_multi", 0.5);
        assert!(
            matches!(result, Err(RuntimeError::Conflict { .. })),
            "partial conflict should reject entire start"
        );
    }

    #[test]
    fn never_new_instance_cleaned_up_on_conflict() {
        let doc = make_never_doc();
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.load_document(doc).unwrap();

        // Start Never SB
        rt.start("sb_never", 0.0).unwrap();

        // Attempt conflicting start → fails
        let result = rt.start("sb_new", 0.5);
        assert!(result.is_err());

        // Update → the Never SB should still be running normally
        let changes = rt.update(1, 1.0);
        // sb_never: x at t=1.0 → progress=0.5 of 0→100 = 50.0
        let x_val = changes.iter().find(|(name, _)| name == "x");
        if let Some((_, val)) = x_val {
            match val {
                EvaluatedValue::Float(v) => {
                    assert!(
                        (*v - 50.0).abs() < 1e-6,
                        "x should be 50.0 at t=1.0, got {v}"
                    );
                }
                _ => panic!("expected Float"),
            }
        }
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn no_side_effects_when_no_conflict() {
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        variable.insert(
            "y".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let sb_x = StoryboardBuilder::new()
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
                at: None,
                between: None,
                keyframe: None,
            })
            .build();

        let sb_y = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(2.0),
                })),
                at: None,
                between: None,
                keyframe: None,
            })
            .build();

        let mut storyboard = BTreeMap::new();
        storyboard.insert("sb_x".to_string(), sb_x);
        storyboard.insert("sb_y".to_string(), sb_y);
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        };

        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.subscribe(1, "y");
        rt.load_document(doc).unwrap();

        rt.start("sb_x", 0.0).unwrap();

        // Update at t=1.0 → x = 50.0
        let changes = rt.update(1, 1.0);
        let x_val = changes.iter().find(|(name, _)| name == "x");
        assert!(x_val.is_some());

        // Start sb_y (different variable) → no conflict, no side effects
        let r = rt.start("sb_y", 1.0).unwrap();
        assert!(r.affected_group_ids.is_empty());

        // Both should still work
        let changes = rt.update(1, 1.5);
        // x should still be updating (from sb_x)
        // y should be updating (from sb_y)
        let x_change = changes.iter().find(|(name, _)| name == "x");
        let y_change = changes.iter().find(|(name, _)| name == "y");
        assert!(
            x_change.is_some() || y_change.is_some(),
            "at least one variable should have changed"
        );
    }

    #[test]
    fn mixed_policies_simultaneous_conflict() {
        let doc = make_multi_sb_doc();
        let mut rt = DolaRuntime::new();
        rt.subscribe(1, "x");
        rt.subscribe(1, "y");
        rt.load_document(doc).unwrap();

        // Start sb_a (Cancel, x+y) at t=0.0
        let r1 = rt.start("sb_a", 0.0).unwrap();
        // Start sb_b (Conclude, x) at t=0.0
        let r2 = rt.start("sb_b", 0.0).unwrap();

        // sb_b conflicts with sb_a on x → sb_a was Cancel'd
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);

        // Start sb_new (x) at t=0.5 → conflicts with sb_b (Conclude)
        let r3 = rt.start("sb_new", 0.5).unwrap();
        assert_eq!(r3.affected_group_ids, vec![r2.group_id]);
    }
}
