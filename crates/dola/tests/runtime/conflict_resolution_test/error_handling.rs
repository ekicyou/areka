//! エラーパスと境界条件（research.md §R3: error_handling シーム）

// =========================================================================
// 6.3 エラーパスと境界条件のテスト
// =========================================================================
mod never_strategy {
    use super::super::*;

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

mod conflict_terminated_parent_triggers {
    use super::super::*;
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
