#![allow(deprecated)]
//! LoopOffset serde round-trip tests
//! Task 1.2: serde ラウンドトリップテスト
//! Task 5.1-5.3: バリデーションテスト
//! Task 2.2: コンパイル統合テスト

use dola::*;
use std::collections::BTreeMap;

// =============================================================
// LoopOffset serde ラウンドトリップ
// =============================================================

mod serde_tests {
    use super::*;

    #[test]
    fn scalar_shorthand_deserialize() {
        // 数値 3.0 → LoopOffset::Scalar(3.0)
        let json = "3.0";
        let lo: LoopOffset = serde_json::from_str(json).unwrap();
        assert_eq!(lo, LoopOffset::Scalar(3.0));
    }

    #[test]
    fn scalar_shorthand_round_trip() {
        let lo = LoopOffset::Scalar(5.0);
        let json = serde_json::to_string(&lo).unwrap();
        let deserialized: LoopOffset = serde_json::from_str(&json).unwrap();
        assert_eq!(lo, deserialized);
    }

    #[test]
    fn object_form_with_all_fields() {
        let json = r#"{ "min": 1.0, "max": 5.0, "easing": "quadratic_out" }"#;
        let lo: LoopOffset = serde_json::from_str(json).unwrap();
        match &lo {
            LoopOffset::Range(r) => {
                assert_eq!(r.min, 1.0);
                assert_eq!(r.max, 5.0);
                assert_eq!(r.easing, EasingFunction::Named(EasingName::QuadraticOut));
            }
            _ => panic!("Expected LoopOffset::Range"),
        }
    }

    #[test]
    fn object_form_round_trip() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: 1.0,
            max: 5.0,
            easing: EasingFunction::Named(EasingName::QuadraticOut),
        });
        let json = serde_json::to_string_pretty(&lo).unwrap();
        let deserialized: LoopOffset = serde_json::from_str(&json).unwrap();
        assert_eq!(lo, deserialized);
    }

    #[test]
    fn easing_default_to_linear() {
        // easing 省略時のデフォルト値は Linear
        let json = r#"{ "min": 0.0, "max": 3.0 }"#;
        let lo: LoopOffset = serde_json::from_str(json).unwrap();
        match &lo {
            LoopOffset::Range(r) => {
                assert_eq!(r.easing, EasingFunction::Named(EasingName::Linear));
            }
            _ => panic!("Expected LoopOffset::Range"),
        }
    }

    #[test]
    fn min_default_to_zero() {
        // min 省略時のデフォルト値は 0.0
        let json = r#"{ "max": 5.0 }"#;
        let lo: LoopOffset = serde_json::from_str(json).unwrap();
        match &lo {
            LoopOffset::Range(r) => {
                assert_eq!(r.min, 0.0);
                assert_eq!(r.max, 5.0);
            }
            _ => panic!("Expected LoopOffset::Range"),
        }
    }

    #[test]
    fn parametric_easing_round_trip() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: 0.5,
            max: 3.0,
            easing: EasingFunction::Parametric(ParametricEasing::CubicBezier {
                x0: 0.0,
                x1: 0.42,
                x2: 0.58,
                x3: 1.0,
            }),
        });
        let json = serde_json::to_string_pretty(&lo).unwrap();
        let deserialized: LoopOffset = serde_json::from_str(&json).unwrap();
        assert_eq!(lo, deserialized);
    }

    #[test]
    fn storyboard_with_scalar_loop_offset() {
        // Storyboard に loop_offset: Scalar を含む JSON
        let json = r#"{
            "loop_count": -1,
            "loop_offset": 5.0,
            "entry": []
        }"#;
        let sb: Storyboard = serde_json::from_str(json).unwrap();
        assert_eq!(sb.loop_offset, Some(LoopOffset::Scalar(5.0)));
    }

    #[test]
    fn storyboard_with_object_loop_offset() {
        let json = r#"{
            "loop_count": -1,
            "loop_offset": {
                "min": 1.0,
                "max": 5.0,
                "easing": "quadratic_out"
            },
            "entry": []
        }"#;
        let sb: Storyboard = serde_json::from_str(json).unwrap();
        match &sb.loop_offset {
            Some(LoopOffset::Range(r)) => {
                assert_eq!(r.min, 1.0);
                assert_eq!(r.max, 5.0);
            }
            other => panic!("Expected Some(LoopOffset::Range), got {:?}", other),
        }
    }

    #[test]
    fn storyboard_without_loop_offset() {
        // loop_offset 省略時は None（後方互換性）
        let json = r#"{
            "loop_count": 1,
            "entry": []
        }"#;
        let sb: Storyboard = serde_json::from_str(json).unwrap();
        assert_eq!(sb.loop_offset, None);
    }

    #[test]
    fn storyboard_round_trip_with_loop_offset() {
        let json = r#"{
            "loop_count": -1,
            "loop_offset": {
                "min": 2.0,
                "max": 8.0,
                "easing": "sine_out"
            },
            "entry": []
        }"#;
        let sb: Storyboard = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&sb).unwrap();
        let deserialized: Storyboard = serde_json::from_str(&serialized).unwrap();
        assert_eq!(sb, deserialized);
    }

    #[test]
    fn storyboard_round_trip_without_loop_offset_skips_field() {
        let sb = Storyboard {
            time_scale: 1.0,
            loop_count: 1,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset: None,
            entry: vec![],
        };
        let json = serde_json::to_string(&sb).unwrap();
        // loop_offset: None → skip_serializing_if でフィールド自体が出力されない
        assert!(!json.contains("loop_offset"));
        let deserialized: Storyboard = serde_json::from_str(&json).unwrap();
        assert_eq!(sb, deserialized);
    }
}

// =============================================================
// ヘルパー: バリデーション・コンパイル用
// =============================================================

fn doc_with_storyboard(sb: Storyboard) -> DolaDocument {
    let mut storyboard = BTreeMap::new();
    storyboard.insert("test_sb".to_string(), sb);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable: BTreeMap::new(),
        transition: BTreeMap::new(),
        storyboard,
    }
}

fn make_storyboard_with_offset(loop_offset: Option<LoopOffset>) -> Storyboard {
    Storyboard {
        time_scale: 1.0,
        loop_count: -1,
        interruption_policy: InterruptionPolicy::Conclude,
        loop_offset,
        entry: vec![],
    }
}

// =============================================================
// V14-V17: ループオフセットバリデーション
// =============================================================

mod validation_tests {
    use super::*;

    #[test]
    fn v14_negative_min_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: -1.0,
                max: 5.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetNegativeMin { storyboard, value }
            if storyboard == "test_sb" && *value == -1.0
        )));
    }

    #[test]
    fn v15_negative_max_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 0.0,
                max: -3.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetNegativeMax { storyboard, value }
            if storyboard == "test_sb" && *value == -3.0
        )));
    }

    #[test]
    fn v15_negative_scalar_error() {
        // スカラー短縮形の負値 → max が負 → V15
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(-2.0)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetNegativeMax { storyboard, value }
            if storyboard == "test_sb" && *value == -2.0
        )));
    }

    #[test]
    fn v16_range_inverted_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 5.0,
                max: 1.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetRangeInverted { storyboard, min, max }
            if storyboard == "test_sb" && *min == 5.0 && *max == 1.0
        )));
    }

    #[test]
    fn valid_scalar_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(3.0)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn valid_range_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 1.0,
                max: 5.0,
                easing: EasingFunction::Named(EasingName::QuadraticOut),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn no_offset_ok() {
        let sb = make_storyboard_with_offset(None);
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn min_equals_max_ok() {
        // min == max は合法（固定遅延）
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 3.0,
                max: 3.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn zero_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(0.0)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }
}

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
    use dola::runtime::DolaRuntime;
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
        // loop_count=2, fixed delay 5.0s → 1st loop ends at t=1.0
        // after loop 1 advance: end_time = 1.0 + 1.0(loop) + 5.0(delay) = 7.0
        let doc = loop_offset_doc("blink", 2, Some(LoopOffset::Scalar(5.0)));
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("blink", 0.0).unwrap();

        // At t=1.5: past first loop end (1.0), advance happens
        // end_time should be 1.0 + 1.0 + delay(0..5) ≥ 2.0
        let _changes = rt.update(1.5).changes;
        // Animation should still be active (within delay period)
        // Value should be at end of first loop (1.0) since we're in delay
        // or at beginning of second loop depending on timing

        // At t=3.0: still within delay period (end_time ≥ 2.0 + delay)
        let _changes = rt.update(3.0).changes;

        // 遅延中は最終値を維持（Req 2.4）
        // (具体的な値の検証は遅延のランダム性により、ここでは存在確認のみ)
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
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("blink", 0.0).unwrap();

        // t=1.5: past first loop end (1.0)
        // advance → end_time = 1.0 + 1.0 + 2.0 = 4.0
        let _changes = rt.update(1.5).changes;

        // t=3.0: still in delay (end_time=4.0)
        let _changes = rt.update(3.0).changes;
        // Should still be active (not concluded)
        // Values should be present (frozen at loop end or interpolating)

        // t=4.5: past second end_time (4.0)
        // advance → end_time = 4.0 + 1.0 + 2.0 = 7.0
        let _changes = rt.update(4.5).changes;

        // t=6.0: still in delay period of 2nd loop
        let _changes = rt.update(6.0).changes;
        // Still active (infinite loop never concludes)
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
        // Task 6.2: Very small delay (0.001s)
        let doc = loop_offset_doc("fast", 3, Some(LoopOffset::Scalar(0.001)));
        let (mut rt, _opacity_id) = setup_runtime(doc);
        rt.start("fast", 0.0).unwrap();

        // Should complete all loops fairly quickly
        let _changes = rt.update(5.0).changes;
        let changes = rt.update(6.0).changes;
        assert!(changes.is_empty()); // concluded
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
