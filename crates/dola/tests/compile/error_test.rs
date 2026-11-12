//! Compile module error handling tests
//! Task 9.5

use dola::*;
use std::collections::BTreeMap;

use super::common::make_doc_with_storyboard;

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn validate_failure_passthrough() {
        // Bad schema version should fail validation
        let doc = DolaDocument {
            schema_version: "0.1".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, DolaError::SchemaVersionMismatch { .. }))
        );
    }

    #[test]
    fn undefined_storyboard_error() {
        let doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };

        let result = compile_storyboard(&doc, "nonexistent", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("not found"))
        ));
    }

    #[test]
    fn keyframe_cycle_detection() {
        // Create a cycle: entry0 (at kf1) -> entry1 (at kf0)
        let doc = make_doc_with_storyboard(
            vec![
                (
                    "x",
                    AnimationVariableDef::Float {
                        initial: 0.0,
                        min: None,
                        max: None,
                    },
                ),
                (
                    "y",
                    AnimationVariableDef::Float {
                        initial: 0.0,
                        min: None,
                        max: None,
                    },
                ),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("kf1".to_string())),
                    keyframe: Some("kf0".to_string()),
                    ..Default::default()
                })
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("kf0".to_string())),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, DolaError::KeyframeCycle { .. }))
        );
    }

    #[test]
    fn between_delay_exceeds_interval() {
        let doc = make_doc_with_storyboard(
            vec![
                (
                    "x",
                    AnimationVariableDef::Float {
                        initial: 0.0,
                        min: None,
                        max: None,
                    },
                ),
                (
                    "y",
                    AnimationVariableDef::Float {
                        initial: 0.0,
                        min: None,
                        max: None,
                    },
                ),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(100.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                // between kf1 (1.0) and kf2 (2.0) with delay 2.0 → exceeds interval
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        relative_to: None,
                        easing: None,
                        delay: 2.0, // exceeds interval (kf2-kf1 = 1.0)
                        duration: None,
                    })),
                    between: Some(BetweenKeyframes {
                        from: "kf1".to_string(),
                        to: "kf2".to_string(),
                    }),
                    keyframe: Some("kf3".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("delay") || reason.contains("exceeds"))
        ));
    }

    #[test]
    fn segment_overlap_detected() {
        // Create overlapping segments for the same variable using 'at' to force overlap
        let doc = make_doc_with_storyboard(
            vec![(
                "x",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: None,
                    max: None,
                },
            )],
            vec![],
            "test",
            StoryboardBuilder::new()
                // Entry 0: pure KF at start - sets kf_start at time 0.0
                .entry(StoryboardEntry {
                    at: Some(KeyframeRef::Single("start".to_string())),
                    keyframe: Some("kf_start".to_string()),
                    ..Default::default()
                })
                // Entry 1: x from 0->100, at start, duration 2.0 → 0.0..2.0
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
                    at: Some(KeyframeRef::Single("start".to_string())),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                // Entry 2: x from 50->200, at start, duration 3.0 → 0.0..3.0 (OVERLAP!)
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(50.0)),
                        to: Some(TransitionValue::Scalar(200.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(3.0),
                    })),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::CompileError { reason, .. } if reason.contains("overlap") || reason.contains("Overlap"))
        ));
    }

    // =========================================================
    // D2-T gap test: 複数エラーの収集と未解決KF参照の連鎖
    // =========================================================

    #[test]
    fn multiple_errors_accumulated_with_unresolvable_downstream_keyframe() {
        // entry 2 の between delay 超過でエラー → kf3 が未登録のまま
        // → entry 3 の at "kf3" が解決不能（2件目のエラー）。
        // バリデーションは通過する（kf3 自体は entry 2 の keyframe として定義済み）。
        let float_var = |initial: f64| AnimationVariableDef::Float {
            initial,
            min: None,
            max: None,
        };
        let doc = make_doc_with_storyboard(
            vec![
                ("x", float_var(0.0)),
                ("y", float_var(0.0)),
                ("z", float_var(0.0)),
            ],
            vec![],
            "test",
            StoryboardBuilder::new()
                // entry 0: x 0.0..1.0, kf1
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(100.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf1".to_string()),
                    ..Default::default()
                })
                // entry 1: x 1.0..2.0, kf2
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(200.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    keyframe: Some("kf2".to_string()),
                    ..Default::default()
                })
                // entry 2: between kf1(1.0)→kf2(2.0) で delay 5.0 → エラー1
                .entry(StoryboardEntry {
                    variable: Some("y".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(50.0)),
                        delay: 5.0,
                        ..Default::default()
                    })),
                    between: Some(BetweenKeyframes {
                        from: "kf1".to_string(),
                        to: "kf2".to_string(),
                    }),
                    keyframe: Some("kf3".to_string()),
                    ..Default::default()
                })
                // entry 3: at kf3（entry 2 の失敗により未登録）→ エラー2
                .entry(StoryboardEntry {
                    variable: Some("z".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(10.0)),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::Single("kf3".to_string())),
                    keyframe: Some("kf4".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert_eq!(errs.len(), 2, "both errors should be collected: {errs:?}");
        assert!(errs.iter().any(|e| matches!(
            e,
            DolaError::CompileError { entry_index: 2, reason, .. } if reason.contains("delay")
        )));
        assert!(errs.iter().any(|e| matches!(
            e,
            DolaError::CompileError { entry_index: 3, reason, .. }
                if reason.contains("Cannot resolve 'at' keyframe reference time")
        )));
    }

    /// 前のエントリが解けないと、at の無い次の純粋なキーフレームも連鎖して解けない（#32）
    #[test]
    fn unresolved_previous_entry_cascades_to_next_pure_keyframe() {
        let doc = make_doc_with_storyboard(
            vec![],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    at: Some(KeyframeRef::Multiple(vec![])),
                    keyframe: Some("a".to_string()),
                    ..Default::default()
                })
                .entry(StoryboardEntry {
                    keyframe: Some("b".to_string()),
                    ..Default::default()
                })
                .build(),
        );

        let errs = compile_storyboard(&doc, "test", 0.0).unwrap_err();
        let indices: Vec<usize> = errs
            .iter()
            .map(|e| match e {
                DolaError::CompileError { entry_index, .. } => *entry_index,
                other => panic!("unexpected error: {other:?}"),
            })
            .collect();
        assert_eq!(indices, vec![0, 1], "{errs:?}");
        assert!(matches!(
            &errs[1],
            DolaError::CompileError { reason, .. } if reason.contains("no previous entry keyframe time")
        ));
    }

    /// relative_to で出した終わりの値も値域と比べて拒む（#33・要件 11.11）
    #[test]
    fn relative_to_out_of_range_is_rejected() {
        let doc = make_doc_with_storyboard(
            vec![(
                "opacity",
                AnimationVariableDef::Float {
                    initial: 0.5,
                    min: Some(0.0),
                    max: Some(1.0),
                },
            )],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        relative_to: Some(0.8),
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    ..Default::default()
                })
                .build(),
        );

        let errs = compile_storyboard(&doc, "test", 0.0).unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e,
                DolaError::ValueOutOfRange { variable, field, value, .. }
                    if variable == "opacity" && field == "relative_to" && (*value - 1.3).abs() < 1e-9
            )),
            "{errs:?}"
        );
    }

    /// 閉路の先にあるだけの節（c at a）は閉路の報告に入らない（#68）
    #[test]
    fn keyframe_cycle_lists_only_cycle_members() {
        let kf = |name: &str, at: &str| StoryboardEntry {
            at: Some(KeyframeRef::Single(at.to_string())),
            keyframe: Some(name.to_string()),
            ..Default::default()
        };
        let doc = make_doc_with_storyboard(
            vec![],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(kf("c", "a"))
                .entry(kf("a", "b"))
                .entry(kf("b", "a"))
                .build(),
        );

        let errs = compile_storyboard(&doc, "test", 0.0).unwrap_err();
        assert_eq!(
            errs,
            vec![DolaError::KeyframeCycle {
                storyboard: "test".to_string(),
                cycle: vec!["a".to_string(), "b".to_string()],
            }]
        );
    }

    /// 重なりのエラーは変数の名前の順に並び、後から重なったエントリを指す（#69）
    #[test]
    fn overlap_error_points_to_offending_entry_in_name_order() {
        let float = AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        };
        let seg = |var: &str, delay: f64, duration: f64| StoryboardEntry {
            variable: Some(var.to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                delay,
                duration: Some(duration),
                ..Default::default()
            })),
            at: Some(KeyframeRef::Single("start".to_string())),
            ..Default::default()
        };
        // 配列は b が先・a が後。重なるのは b の 1 番目と a の 3 番目
        let doc = make_doc_with_storyboard(
            vec![("a", float.clone()), ("b", float)],
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(seg("b", 0.0, 2.0))
                .entry(seg("b", 0.0, 1.0))
                .entry(seg("a", 0.0, 2.0))
                .entry(seg("a", 1.0, 1.0))
                .build(),
        );

        let errs = compile_storyboard(&doc, "test", 0.0).unwrap_err();
        let got: Vec<(usize, bool)> = errs
            .iter()
            .map(|e| match e {
                DolaError::CompileError {
                    entry_index,
                    reason,
                    ..
                } => (*entry_index, reason.contains("variable 'a'")),
                other => panic!("unexpected error {other:?}"),
            })
            .collect();
        assert_eq!(got, vec![(3, true), (1, false)], "{errs:?}");
    }
}
