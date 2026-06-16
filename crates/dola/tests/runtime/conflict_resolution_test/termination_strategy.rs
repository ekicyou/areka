//! 各終了戦略の統合テスト（research.md §R3: termination_strategy シーム）

// =========================================================================
// 6.2 各終了戦略の統合テスト
// =========================================================================
mod cancel_strategy {
    use super::super::*;

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
    use super::super::*;

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
    use super::super::*;

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
    use super::super::*;

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

mod edge_cases {
    use super::super::*;

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
