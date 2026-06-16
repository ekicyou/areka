//! 競合検出ロジック（research.md §R3: conflict_detection シーム）

// =========================================================================
// 6.1 競合検出ロジックのテスト
// =========================================================================
mod conflict_detection {
    use super::super::*;

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
// D1b-T 追加: 競合検出の境界条件と終了経路の特性化
// =========================================================================
mod conflict_detection_boundaries {
    use super::super::*;

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
