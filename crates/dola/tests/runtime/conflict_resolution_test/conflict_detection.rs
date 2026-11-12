//! 競合検出ロジック（research.md §R3: conflict_detection シーム）

// =========================================================================
// 6.1 競合検出ロジックのテスト
// =========================================================================
mod conflict_detection {
    use super::super::*;

    #[test]
    fn no_conflict_when_different_variables() {
        // 異なる変数の SB は競合しない
        let doc = DolaDocumentBuilder::new("1.0")
            .variable("x", float())
            .variable("y", float())
            .storyboard(
                "sb_x",
                StoryboardBuilder::new()
                    .entry(linear("x", 0.0, 100.0, 2.0))
                    .build(),
            )
            .storyboard(
                "sb_y",
                StoryboardBuilder::new()
                    .entry(linear("y", 0.0, 100.0, 2.0))
                    .build(),
            )
            .build()
            .unwrap();

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

    /// Paused 状態のインスタンスも競合検出の対象（Playing/Paused の両方を生きた項目と見る）
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

    /// 競合は「同じ変数に生きた項目がある」で決まり、時刻の重なりは見ない（要件 11.8）。
    /// 壁時計で隣り合う [0,2] と [2,4] でも、前のインスタンスが生きていれば競合する。
    #[test]
    fn time_shifted_start_on_same_variable_still_conflicts() {
        let doc = make_doc_with_policy("fade", InterruptionPolicy::Cancel);
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        // 既存: 壁時計 [0.0, 2.0]
        let r1 = rt.start("fade", 0.0).unwrap();
        // 新規: 壁時計 [2.0, 4.0] — 壁時計では隣り合うだけだが、r1 がまだ生きているので競合する
        let r2 = rt.start("fade", 2.0).unwrap();
        assert_eq!(
            r2.affected_group_ids,
            vec![r1.group_id],
            "a live entry on the same variable conflicts"
        );
    }

    /// A（Never）の x は delay 5 で壁時計 [5,6]・B の x は t=5.0 から [5,6]。
    /// 相対の [5,6] と [0,1] は重ならないが、x に A の生きた項目があるので B は拒まれる。
    #[test]
    fn never_blocks_overlap_on_wall_clock() {
        let x_sb = |policy, delay| {
            StoryboardBuilder::new()
                .interruption_policy(policy)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        delay,
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    ..Default::default()
                })
                .build()
        };
        let doc = DolaDocumentBuilder::new("1.0")
            .variable("x", float())
            .storyboard("a", x_sb(InterruptionPolicy::Never, 5.0))
            .storyboard("b", x_sb(InterruptionPolicy::Conclude, 0.0))
            .build()
            .unwrap();
        let mut rt = DolaRuntime::new();
        let _x_id = rt.subscribe("x");
        rt.load_document(doc).unwrap();

        let a = rt.start("a", 0.0).unwrap();
        match rt.start("b", 5.0) {
            Err(RuntimeError::Conflict {
                conflicting_group_ids,
            }) => assert_eq!(conflicting_group_ids, vec![a.group_id]),
            other => panic!("expected RuntimeError::Conflict, got {other:?}"),
        }
    }

    /// 新 SB が複数変数を操作し、各変数を別インスタンスが保持している場合、
    /// 全インスタンスが affected に含まれる
    #[test]
    fn overlap_with_multiple_instances_affects_all() {
        let make_sb = |vars: &[&str]| {
            let mut b = StoryboardBuilder::new().interruption_policy(InterruptionPolicy::Cancel);
            for v in vars {
                b = b.entry(linear(v, 0.0, 100.0, 2.0));
            }
            b.build()
        };
        let doc = DolaDocumentBuilder::new("1.0")
            .variable("x", float())
            .variable("y", float())
            .storyboard("sb_x", make_sb(&["x"]))
            .storyboard("sb_y", make_sb(&["y"]))
            .storyboard("sb_xy", make_sb(&["x", "y"]))
            .build()
            .unwrap();

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
        assert_eq!(r3.affected_group_ids, vec![r1.group_id, r2.group_id]);
    }
}
