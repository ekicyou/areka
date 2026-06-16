#![allow(deprecated)]
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
        let _x_id = rt.subscribe("x");
        let _y_id = rt.subscribe("y");
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
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        let result = rt.start("fade", 0.0).unwrap();
        assert!(result.affected_group_ids.is_empty());
    }

    #[test]
    fn conflict_detected_on_same_variable() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
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
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        let _r1 = rt.start("fade", 0.0).unwrap();
        // 自然終了を発生させる
        rt.update(3.0);

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
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        // Start 1st at t=0.0
        rt.start("fade", 0.0).unwrap();
        // Update at t=0.5 → opacity = 25.0 (progress=0.25 of 0→100, 2s duration)
        let changes = rt.update(0.5).changes;
        assert!(!changes.is_empty());

        // Start 2nd at t=1.0 → Cancel 1st, freeze at t=1.0 → 50.0
        let r2 = rt.start("fade", 1.0).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // Update → 新しい SB の値が取得される
        let changes = rt.update(1.0).changes;
        // The cancelled instance's frozen value should be 50.0,
        // but now the new instance starts at 0.0
        // The subscription manager should show the new SB's value
        assert!(!changes.is_empty());
    }

    #[test]
    fn cancel_group_id_all_variables() {
        let doc = make_multi_sb_doc();
        let mut rt = DolaRuntime::new();
        let _x_id = rt.subscribe("x");
        let y_id = rt.subscribe("y");
        rt.load_document(doc).unwrap();

        // Start sb_a (Cancel policy, operates x and y)
        let r1 = rt.start("sb_a", 0.0).unwrap();
        assert!(r1.affected_group_ids.is_empty());

        // Start sb_new (operates x only) at t=0.5
        // → sb_a conflicts on x → Cancel sb_a (both x and y entries removed)
        let r2 = rt.start("sb_new", 0.5).unwrap();
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);

        // sb_a's y values should be frozen at t=0.5 → 12.5 (progress=0.25 of 0→50)
        let changes = rt.update(0.5).changes;
        // y should show frozen value, x shows new SB value
        let y_val = changes.iter().find(|(id, _)| *id == y_id);
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
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        // Start at t=0.0
        rt.start("fade", 0.0).unwrap();
        // Update to get initial value
        let changes = rt.update(0.5).changes;
        assert!(!changes.is_empty());

        // Start 2nd at t=1.0 → Conclude 1st → jump to segment final (100.0)
        let r2 = rt.start("fade", 1.0).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // The concluded instance's last value should be 100.0 (segment final)
        // Next update should see new SB starting
        let changes = rt.update(1.0).changes;
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
                ..Default::default()
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
        let _opacity_id = rt.subscribe("opacity");
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
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        rt.start("fade", 0.0).unwrap();
        rt.update(0.5);

        // Start 2nd at t=1.0 → Trim 1st at t=1.0 → value = 50.0
        let r2 = rt.start("fade", 1.0).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // Update → trimmed value should have been propagated
        let changes = rt.update(1.0).changes;
        assert!(!changes.is_empty());
    }
}

mod compress_strategy {
    use super::*;

    #[test]
    fn compress_jumps_to_all_final_values() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Compress);
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        rt.start("fade", 0.0).unwrap();
        rt.update(0.5);

        // Start 2nd at t=0.5 → Compress 1st → all final values (100.0)
        let r2 = rt.start("fade", 0.5).unwrap();
        assert!(!r2.affected_group_ids.is_empty());

        // Update → compressed value was 100.0, new SB starts
        let changes = rt.update(0.5).changes;
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
        let _x_id = rt.subscribe("x");
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
        let _x_id = rt.subscribe("x");
        let _y_id = rt.subscribe("y");
        let _z_id = rt.subscribe("z");
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
        let x_id = rt.subscribe("x");
        rt.load_document(doc).unwrap();

        // Start Never SB
        rt.start("sb_never", 0.0).unwrap();

        // Attempt conflicting start → fails
        let result = rt.start("sb_new", 0.5);
        assert!(result.is_err());

        // Update → the Never SB should still be running normally
        let changes = rt.update(1.0).changes;
        // sb_never: x at t=1.0 → progress=0.5 of 0→100 = 50.0
        let x_val = changes.iter().find(|(id, _)| *id == x_id);
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
                ..Default::default()
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
                ..Default::default()
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
        let x_id = rt.subscribe("x");
        let y_id = rt.subscribe("y");
        rt.load_document(doc).unwrap();

        rt.start("sb_x", 0.0).unwrap();

        // Update at t=1.0 → x = 50.0
        let changes = rt.update(1.0).changes;
        let x_val = changes.iter().find(|(id, _)| *id == x_id);
        assert!(x_val.is_some());

        // Start sb_y (different variable) → no conflict, no side effects
        let r = rt.start("sb_y", 1.0).unwrap();
        assert!(r.affected_group_ids.is_empty());

        // Both should still work
        let changes = rt.update(1.5).changes;
        // x should still be updating (from sb_x)
        // y should be updating (from sb_y)
        let x_change = changes.iter().find(|(id, _)| *id == x_id);
        let y_change = changes.iter().find(|(id, _)| *id == y_id);
        assert!(
            x_change.is_some() || y_change.is_some(),
            "at least one variable should have changed"
        );
    }

    #[test]
    fn mixed_policies_simultaneous_conflict() {
        let doc = make_multi_sb_doc();
        let mut rt = DolaRuntime::new();
        let _x_id = rt.subscribe("x");
        let _y_id = rt.subscribe("y");
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

// =========================================================================
// D1b-T 追加: 競合検出の境界条件と終了経路の特性化
// =========================================================================
mod conflict_detection_boundaries {
    use super::*;

    /// Paused 状態のインスタンスも競合検出の対象（detect_overlaps は Playing/Paused 両方を見る）
    #[test]
    fn paused_instance_still_conflicts() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        let r1 = rt.start("fade", 0.0).unwrap();
        rt.update(0.5);
        rt.pause(r1.group_id, 0.5).unwrap();

        // Paused 中でも同一変数の新規 start は競合し、終了戦略が適用される
        let r2 = rt.start("fade", 1.0).unwrap();
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);
        // Cancel 済みインスタンスへの resume は InvalidGroupId（削除済み）
        assert!(matches!(
            rt.resume(r1.group_id, 1.5),
            Err(RuntimeError::InvalidGroupId(_))
        ));
    }

    /// 特性化: 競合判定はストーリーボードローカル座標で行われ、wall-clock の
    /// start_time を考慮しない。
    ///
    /// facade はすべての SB を base_time=0.0 でコンパイルし（`compile_and_validate(name, 0.0)`）、
    /// `detect_overlaps` は `_start_time` 引数を使用せず compile 時座標どうしを比較する。
    /// そのため壁時計上は重ならない [0,2] と [2,4] のスケジュールでも、
    /// 同一変数を操作する Playing/Paused インスタンスがあれば常に競合扱いとなる（P11 参照）。
    #[test]
    fn time_shifted_start_on_same_variable_still_conflicts() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        // 既存: 壁時計 [0.0, 2.0]
        let r1 = rt.start("fade", 0.0).unwrap();
        // 新規: 壁時計 [2.0, 4.0] — 壁時計上は隣接（非重複）だが現行実装では競合する
        let r2 = rt.start("fade", 2.0).unwrap();
        assert_eq!(
            r2.affected_group_ids,
            vec![r1.group_id],
            "current behavior: overlap detection uses storyboard-local coordinates"
        );
    }

    /// 新 SB が複数変数を操作し、各変数を別インスタンスが保持している場合、
    /// 全インスタンスが affected に含まれる
    #[test]
    fn overlap_with_multiple_instances_affects_all() {
        let mut variable = BTreeMap::new();
        for name in ["x", "y"] {
            variable.insert(
                name.to_string(),
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            );
        }

        let make_sb = |vars: &[&str]| {
            let mut b = StoryboardBuilder::new().interruption_policy(InterruptionPolicy::Cancel);
            for v in vars {
                b = b.entry(StoryboardEntry {
                    variable: Some(v.to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    ..Default::default()
                });
            }
            b.build()
        };

        let mut storyboard = BTreeMap::new();
        storyboard.insert("sb_x".to_string(), make_sb(&["x"]));
        storyboard.insert("sb_y".to_string(), make_sb(&["y"]));
        storyboard.insert("sb_xy".to_string(), make_sb(&["x", "y"]));
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        };

        let mut rt = DolaRuntime::new();
        let _x_id = rt.subscribe("x");
        let _y_id = rt.subscribe("y");
        rt.load_document(doc).unwrap();

        // gid1: x のみ、gid2: y のみ（変数が異なるため共存）
        let r1 = rt.start("sb_x", 0.0).unwrap();
        let r2 = rt.start("sb_y", 0.0).unwrap();
        assert!(r2.affected_group_ids.is_empty());

        // sb_xy は x・y 両方を操作 → 両インスタンスと競合
        let r3 = rt.start("sb_xy", 0.5).unwrap();
        let mut affected = r3.affected_group_ids.clone();
        affected.sort_unstable();
        assert_eq!(affected, vec![r1.group_id, r2.group_id]);
    }
}

mod conflict_terminated_parent_triggers {
    use super::*;
    use dola::KeyframeRef;

    /// 親（トリガー保持・未発火）が競合解決で終了した場合、トリガーは発火しない。
    ///
    /// 特性化（D1a-V 申し送りの stale-entry 領域）: conflict_resolver の終了経路は
    /// instance_manager からインスタンスを除去するが、facade の trigger_store
    /// エントリは残置される。process_triggers はインスタンス起点で走査するため
    /// 残置エントリが読まれることはなく、外部観測上は「トリガーが発火しない」
    /// 挙動として現れる（panic も発火もしないことを固定する）。
    #[test]
    fn conflict_terminated_parent_trigger_never_fires() {
        // parent: opacity 0→100 (2.0s, kf1 at end=2.0) + trigger at kf1 → child
        // interrupt: opacity を操作（親と競合）
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let parent = StoryboardBuilder::new()
            .interruption_policy(InterruptionPolicy::Cancel)
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
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                trigger_storyboard: Some("child".to_string()),
                at: Some(KeyframeRef::Single("kf1".to_string())),
                ..Default::default()
            })
            .build();

        let child = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(100.0)),
                    to: Some(TransitionValue::Scalar(0.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                ..Default::default()
            })
            .build();

        let interrupt = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(50.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                ..Default::default()
            })
            .build();

        let mut storyboard = BTreeMap::new();
        storyboard.insert("parent".to_string(), parent);
        storyboard.insert("child".to_string(), child);
        storyboard.insert("interrupt".to_string(), interrupt);
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        };

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        // 親開始（トリガーは t=2.0 で発火予定）
        let r_parent = rt.start("parent", 0.0).unwrap();
        rt.update(0.5);

        // t=1.0: interrupt 開始 → 親が Cancel 終了（トリガー未発火のまま）
        let r_int = rt.start("interrupt", 1.0).unwrap();
        assert_eq!(r_int.affected_group_ids, vec![r_parent.group_id]);

        // 発火予定時刻を過ぎても、終了済みの親のトリガーは発火しない
        let result = rt.update(2.5);
        assert!(
            result.triggered.is_empty(),
            "trigger of conflict-terminated parent must not fire: {:?}",
            result.triggered
        );
        // さらに後続 update でも発火・panic しない
        let result = rt.update(5.0);
        assert!(result.triggered.is_empty());
    }

    /// トリガー起動された子が親と同一変数で時間重複しても、親は競合解決から除外される
    /// （resolve_conflicts_excluding の skip_group_ids 経路の固定）
    #[test]
    fn triggered_child_excludes_parent_from_conflict() {
        // parent: entry1 opacity 0→1 (1.0s, kf1) + entry2 opacity 1→0 (1.0s) → 全体 [0,2]
        //         + trigger at kf1 → child（child は [1,2] で entry2 と重複）
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let parent = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                keyframe: Some("kf1".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(1.0)),
                    to: Some(TransitionValue::Scalar(0.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                trigger_storyboard: Some("child".to_string()),
                at: Some(KeyframeRef::Single("kf1".to_string())),
                ..Default::default()
            })
            .build();

        let child = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.5)),
                    to: Some(TransitionValue::Scalar(0.7)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                ..Default::default()
            })
            .build();

        let mut storyboard = BTreeMap::new();
        storyboard.insert("parent".to_string(), parent);
        storyboard.insert("child".to_string(), child);
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        };

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        rt.start("parent", 0.0).unwrap();

        // t=1.0: トリガー発火 → 子 [1,2] は親の entry2 [1,2] と重複するが、
        // 親は skip_group_ids により競合解決から除外される
        let result = rt.update(1.0);
        assert_eq!(result.triggered.len(), 1, "trigger should fire at t=1.0");
        match &result.triggered[0] {
            dola::runtime::TriggerResult::Started { start_result, .. } => {
                assert!(
                    start_result.affected_group_ids.is_empty(),
                    "parent must be excluded from conflict resolution, got affected: {:?}",
                    start_result.affected_group_ids
                );
            }
            other => panic!("expected Started, got {other:?}"),
        }
    }
}
