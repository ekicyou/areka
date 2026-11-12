//! コンパイルパイプライン統合 + E2E 統合（Task 6.4 で分割）
use super::*;

// =============================================================
// Task 2.2: コンパイルパイプライン統合テスト
// =============================================================

mod compile_integration_tests {
    use super::*;

    fn doc_with_fade_and_offset(loop_offset: Option<LoopOffset>) -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "blink".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: -1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset,
                entry: vec![StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(1.0)),
                        to: Some(TransitionValue::Scalar(0.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(0.5),
                    })),
                    ..Default::default()
                }],
            },
        );

        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    #[test]
    fn compile_with_scalar_loop_offset() {
        let doc = doc_with_fade_and_offset(Some(LoopOffset::Scalar(5.0)));
        let compiled = compile_storyboard(&doc, "blink", 0.0).unwrap();
        assert_eq!(compiled.loop_offset, Some(LoopOffset::Scalar(5.0)));
        assert_eq!(compiled.loop_count, -1);
    }

    #[test]
    fn compile_with_range_loop_offset() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: 1.0,
            max: 5.0,
            easing: EasingFunction::Named(EasingName::QuadraticOut),
        });
        let doc = doc_with_fade_and_offset(Some(lo.clone()));
        let compiled = compile_storyboard(&doc, "blink", 0.0).unwrap();
        assert_eq!(compiled.loop_offset, Some(lo));
    }

    #[test]
    fn compile_without_loop_offset() {
        let doc = doc_with_fade_and_offset(None);
        let compiled = compile_storyboard(&doc, "blink", 0.0).unwrap();
        assert_eq!(compiled.loop_offset, None);
    }

    #[test]
    fn compile_loop_offset_preserved_across_serialization() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: 2.0,
            max: 8.0,
            easing: EasingFunction::Named(EasingName::SineOut),
        });
        let doc = doc_with_fade_and_offset(Some(lo.clone()));
        let compiled = compile_storyboard(&doc, "blink", 0.0).unwrap();

        let json = serde_json::to_string_pretty(&compiled).unwrap();
        let deserialized: CompiledStoryboard = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.loop_offset, Some(lo));
    }
}

// =============================================================
// Task 4.1 + 6.1 + 6.2: E2E 統合テスト
// =============================================================

mod e2e_tests {
    use dola::runtime::{DolaRuntime, EvaluatedValue};
    use dola::*;
    use std::collections::BTreeMap;

    /// loop_offset 付きストーリーボードのドキュメントを生成するヘルパー
    fn loop_offset_doc(
        sb_name: &str,
        loop_count: i32,
        loop_offset: Option<LoopOffset>,
    ) -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        );
        let sb = Storyboard {
            time_scale: 1.0,
            loop_count,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset,
            entry: vec![StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(1.0),
                })),
                ..Default::default()
            }],
        };
        let mut storyboard = BTreeMap::new();
        storyboard.insert(sb_name.to_string(), sb);
        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    fn setup_runtime(doc: DolaDocument) -> (DolaRuntime, i64) {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        (rt, opacity_id)
    }

    /// t まで進め、その回に出た opacity の値を返す（変わらなければ None）
    fn opacity_at(rt: &mut DolaRuntime, opacity_id: i64, t: f64) -> Option<f64> {
        rt.tick(t);
        rt.last_result()
            .changes
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .map(|(_, v)| match v {
                EvaluatedValue::Float(f) => *f,
                other => panic!("expected Float, got {other:?}"),
            })
    }

    #[test]
    fn e2e_no_offset_natural_end() {
        // loop_count=2, no offset → ends at t=2.0
        let doc = loop_offset_doc("blink", 2, None);
        let (mut rt, _opacity_id) = setup_runtime(doc);
        let result = rt.start("blink", 0.0).unwrap();
        assert_eq!(result.end_time, 1.0); // first loop end

        // Mid first loop
        let changes = rt.update(0.5).changes;
        assert!(!changes.is_empty());

        // After 2 loops → conclude
        let _changes = rt.update(2.5).changes;
        // Should have final values (opacity=1.0 at end of loop)
        // After conclude, subsequent update returns empty
        let changes = rt.update(3.0).changes;
        assert!(changes.is_empty());
    }

    #[test]
    fn e2e_with_fixed_delay_extends_playback() {
        // loop_count=2・待ち 2 秒固定 → 1 周目 [0,1]・待ち [1,3]・2 周目 [3,4]
        let doc = loop_offset_doc(
            "blink",
            2,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 2.0,
                max: 2.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("blink", 0.0).unwrap();

        let v = opacity_at(&mut rt, opacity_id, 0.5).expect("opacity at t=0.5");
        assert!((v - 0.5).abs() < 1e-9, "t=0.5: {v}");

        // 周回の終わりを越えて待ちに入る（待ちの間の値は 5.3 の値の保持の後に縛る）
        rt.tick(1.5);

        // 起点が待ちの後（3.0）にあれば 2 周目の中ほど
        let v = opacity_at(&mut rt, opacity_id, 3.5).expect("opacity at t=3.5");
        assert!((v - 0.5).abs() < 1e-9, "t=3.5: {v}");

        // 2 周目の終わり 4.0 を越えて終わる → 最終値
        let v = opacity_at(&mut rt, opacity_id, 4.5).expect("opacity at t=4.5");
        assert!((v - 1.0).abs() < 1e-9, "t=4.5: {v}");

        rt.tick(5.0);
        assert!(rt.last_result().changes.is_empty());
    }

    #[test]
    fn e2e_loop_count_1_ignores_offset() {
        // loop_count=1 + offset → offset is ignored, concludes normally
        let doc = loop_offset_doc("blink", 1, Some(LoopOffset::Scalar(10.0)));
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("blink", 0.0).unwrap();

        // At t=1.5: past end_time=1.0, loop_count=1 → conclude
        let _changes = rt.update(1.5).changes;

        // After conclude, no more updates
        let changes = rt.update(2.0).changes;
        assert!(changes.is_empty());
    }

    #[test]
    fn e2e_infinite_loop_with_offset_continues() {
        // loop_count=-1 + fixed delay 2.0s
        let doc = loop_offset_doc(
            "blink",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 2.0,
                max: 2.0, // fixed 2.0s delay
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("blink", 0.0).unwrap();
        // 周回は [0,1]・[3,4]・[6,7]…（待ちの間の値は 5.3 で縛る）
        for (t, want) in [
            (0.5, Some(0.5)),
            (1.5, None),
            (3.5, Some(0.5)),
            (4.5, None),
            (6.5, Some(0.5)),
        ] {
            let got = opacity_at(&mut rt, opacity_id, t);
            if let Some(want) = want {
                let v = got.unwrap_or_else(|| panic!("opacity at t={t}"));
                assert!((v - want).abs() < 1e-9, "t={t}: {v}");
            }
        }
    }

    #[test]
    fn e2e_pause_during_delay_preserves_timing() {
        // Task 4.1: Pause during delay period
        let doc = loop_offset_doc(
            "blink",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 5.0,
                max: 5.0, // fixed 5.0s delay
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        let result = rt.start("blink", 0.0).unwrap();
        let group_id = result.group_id;

        // t=1.5: past first loop end → advance → end_time = 1.0+1.0+5.0 = 7.0
        rt.update(1.5);

        // Pause at t=3.0 (in delay period, end_time=7.0)
        rt.pause(group_id, 3.0).unwrap();

        // Resume at t=5.0 (2.0s pause duration)
        let new_end = rt.resume(group_id, 5.0).unwrap();
        // end_time should be shifted by pause duration: 7.0 + 2.0 = 9.0
        assert_eq!(new_end, 9.0);
    }

    #[test]
    fn e2e_cancel_during_delay() {
        // Task 4.1: Cancel during delay period → immediate stop
        let doc = loop_offset_doc(
            "blink",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 10.0,
                max: 10.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        let result = rt.start("blink", 0.0).unwrap();
        let group_id = result.group_id;

        // t=1.5: advance → in delay period
        rt.update(1.5);

        // Cancel → immediate stop
        rt.cancel(group_id).unwrap();

        // After cancel, no more updates
        let changes = rt.update(2.0).changes;
        assert!(changes.is_empty());
    }

    #[test]
    fn e2e_without_offset_backward_compatible() {
        // No loop_offset → identical behavior to before
        let doc = loop_offset_doc("fade", 3, None);
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("fade", 0.0).unwrap();

        // Mid-loop updates should work normally
        let changes = rt.update(0.5).changes;
        assert!(!changes.is_empty());

        // All 3 loops complete at t=3.0+
        let _changes = rt.update(3.5).changes;
        let changes = rt.update(4.0).changes;
        assert!(changes.is_empty()); // concluded
    }

    #[test]
    fn e2e_extreme_small_delay() {
        // 待ち 0.001 秒固定・3 周 → 周回は [0,1]・[1.001,2.001]・[2.002,3.002]
        let doc = loop_offset_doc(
            "fast",
            3,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 0.001,
                max: 0.001,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("fast", 0.0).unwrap();

        // 終わり 3.002 の直前は 3 周目の中で生きている
        let v = opacity_at(&mut rt, opacity_id, 3.0).expect("opacity at t=3.0");
        assert!((v - 0.998).abs() < 1e-9, "t=3.0: {v}");

        // 直後に終わって最終値、その後は差分なし
        let v = opacity_at(&mut rt, opacity_id, 3.003).expect("opacity at t=3.003");
        assert!((v - 1.0).abs() < 1e-9, "t=3.003: {v}");
        rt.tick(3.01);
        assert!(rt.last_result().changes.is_empty());
    }

    #[test]
    fn e2e_large_delay() {
        // Task 6.2: Large delay (100.0s)
        let doc = loop_offset_doc(
            "slow",
            2,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 100.0,
                max: 100.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("slow", 0.0).unwrap();

        // At t=1.5: past first loop → advance, end_time=1.0+1.0+100.0=102.0
        let _changes = rt.update(1.5).changes;

        // At t=50.0: still in delay
        let _changes = rt.update(50.0).changes;
        // Should still be alive

        // At t=105.0: past end_time=102.0, 2nd loop completes → conclude
        let _changes = rt.update(105.0).changes;
        let changes = rt.update(106.0).changes;
        assert!(changes.is_empty()); // concluded
    }

    #[test]
    fn e2e_f64_precision_many_loops() {
        // Task 6.2: Many loops with delay — check no precision issues
        let doc = loop_offset_doc(
            "precision",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: 0.1,
                max: 0.1,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("precision", 0.0).unwrap();

        // Advance through many loops: each loop = 1.0s + 0.1s delay = 1.1s/loop
        // 100 loops ≈ 110s
        let _changes = rt.update(115.0).changes;
        // Should still be running (infinite loops), no crash, no infinite loop
    }
}
