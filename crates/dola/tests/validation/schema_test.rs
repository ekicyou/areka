//! Validation tests for V1-V5 (schema version, keyframe names, variable/transition references)
//! Tasks 7.6, 8.6

use dola::*;
use std::collections::BTreeMap;

use super::common::minimal_valid_doc;

// =============================================================
// V1: スキーマバージョン検証
// =============================================================

mod v1_tests {
    use super::*;

    #[test]
    fn schema_version_1_0_ok() {
        let doc = minimal_valid_doc();
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn schema_version_mismatch() {
        let doc = DolaDocument {
            schema_version: "2.0".to_string(),
            ..minimal_valid_doc()
        };
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::SchemaVersionMismatch { expected, found }
            if expected == "1.0" && found == "2.0"
        )));
    }
}

// =============================================================
// D3-T 追加: 複数ルールのエラー蓄積（fail-fast しないことの固定）
// =============================================================

mod error_accumulation_tests {
    use super::*;

    #[test]
    fn validate_collects_errors_from_multiple_rules_in_one_pass() {
        // V1（スキーマ不一致）と V4（未定義変数）が 1 回の validate() で両方報告される
        let mut doc = DolaDocument {
            schema_version: "9.9".to_string(),
            ..minimal_valid_doc()
        };
        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("undefined_var".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.len() >= 2, "expected >=2 errors, got {errors:?}");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::SchemaVersionMismatch { .. }))
        );
        assert!(errors.iter().any(
            |e| matches!(e, DolaError::UndefinedVariable { name, .. } if name == "undefined_var")
        ));
    }
}

// =============================================================
// 数値の規則（#29）: 有限・符号・範囲を外れた値は InvalidNumber で拒む
// =============================================================

mod numeric_rule_tests {
    use super::*;

    /// 変数 "x" と、"x" へインラインの遷移を 1 本張るストーリーボード "sb"
    fn base_doc() -> DolaDocument {
        let mut doc = minimal_valid_doc();
        doc.variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        doc.storyboard.insert(
            "sb".to_string(),
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        to: Some(TransitionValue::Scalar(1.0)),
                        delay: 0.0,
                        duration: Some(1.0),
                        ..Default::default()
                    })),
                    ..Default::default()
                })
                .build(),
        );
        doc
    }

    fn sb_mut(doc: &mut DolaDocument) -> &mut Storyboard {
        doc.storyboard.get_mut("sb").unwrap()
    }

    fn trans(doc: &mut DolaDocument) -> &mut TransitionDef {
        match sb_mut(doc).entry[0].transition.as_mut() {
            Some(TransitionRef::Inline(def)) => def,
            _ => unreachable!(),
        }
    }

    #[test]
    fn invalid_numbers_are_rejected() {
        type Mutate = fn(&mut DolaDocument);
        let cases: [(Mutate, &str, &str, f64); 12] = [
            (
                |d| sb_mut(d).time_scale = 0.0,
                "storyboard 'sb'",
                "time_scale",
                0.0,
            ),
            (
                |d| sb_mut(d).time_scale = -1.0,
                "storyboard 'sb'",
                "time_scale",
                -1.0,
            ),
            (
                |d| sb_mut(d).time_scale = f64::NAN,
                "storyboard 'sb'",
                "time_scale",
                f64::NAN,
            ),
            (
                |d| trans(d).duration = Some(f64::NAN),
                "storyboard 'sb' entry 0",
                "duration",
                f64::NAN,
            ),
            (
                |d| trans(d).duration = Some(-3.0),
                "storyboard 'sb' entry 0",
                "duration",
                -3.0,
            ),
            (
                |d| trans(d).delay = -1.0,
                "storyboard 'sb' entry 0",
                "delay",
                -1.0,
            ),
            (
                |d| sb_mut(d).loop_count = 0,
                "storyboard 'sb'",
                "loop_count",
                0.0,
            ),
            (
                |d| sb_mut(d).loop_offset = Some(LoopOffset::Scalar(f64::NAN)),
                "storyboard 'sb'",
                "loop_offset",
                f64::NAN,
            ),
            (
                |d| {
                    d.variable.insert(
                        "x".to_string(),
                        AnimationVariableDef::Float {
                            initial: f64::INFINITY,
                            min: None,
                            max: None,
                        },
                    );
                },
                "variable 'x'",
                "initial",
                f64::INFINITY,
            ),
            (
                |d| trans(d).to = Some(TransitionValue::Scalar(f64::NEG_INFINITY)),
                "storyboard 'sb' entry 0",
                "to",
                f64::NEG_INFINITY,
            ),
            (
                |d| {
                    trans(d).easing =
                        Some(EasingFunction::Parametric(ParametricEasing::CubicBezier {
                            x0: 0.0,
                            x1: f64::NAN,
                            x2: 1.0,
                            x3: 1.0,
                        }))
                },
                "storyboard 'sb' entry 0",
                "easing.x1",
                f64::NAN,
            ),
            (
                |d| {
                    sb_mut(d).loop_offset = Some(LoopOffset::Range(LoopOffsetRange {
                        min: 0.0,
                        max: 1.0,
                        easing: EasingFunction::Parametric(ParametricEasing::QuadraticBezier {
                            x0: 0.0,
                            x1: 0.5,
                            x2: f64::INFINITY,
                        }),
                    }))
                },
                "storyboard 'sb'",
                "loop_offset.easing.x2",
                f64::INFINITY,
            ),
        ];

        assert!(base_doc().validate().is_ok());
        for (mutate, location, field, value) in cases {
            let mut doc = base_doc();
            mutate(&mut doc);
            let errors = doc.validate().unwrap_err();
            assert_eq!(errors.len(), 1, "{field} = {value}: {errors:?}");
            assert!(
                matches!(
                    &errors[0],
                    DolaError::InvalidNumber { location: l, field: f, value: v }
                    if l == location && f == field && v.to_bits() == value.to_bits()
                ),
                "{field} = {value}: {errors:?}"
            );
        }
    }

    #[test]
    fn named_transition_numbers_are_checked() {
        let mut doc = minimal_valid_doc();
        doc.transition.insert(
            "slide".to_string(),
            TransitionDef {
                relative_to: Some(f64::NAN),
                ..Default::default()
            },
        );
        let errors = doc.validate().unwrap_err();
        assert!(matches!(
            &errors[..],
            [DolaError::InvalidNumber { location, field, .. }]
            if location == "transition 'slide'" && field == "relative_to"
        ));
    }

    #[test]
    fn all_violations_are_collected() {
        let mut doc = base_doc();
        sb_mut(&mut doc).time_scale = 0.0;
        sb_mut(&mut doc).loop_count = -2;
        trans(&mut doc).duration = Some(-1.0);
        let errors = doc.validate().unwrap_err();
        let fields: Vec<&str> = errors
            .iter()
            .filter_map(|e| match e {
                DolaError::InvalidNumber { field, .. } => Some(field.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(fields, ["time_scale", "loop_count", "duration"]);
    }

    #[test]
    fn loop_count_must_be_positive_or_minus_one() {
        // storyboard.rs の説明どおり 0 と −2 以下は拒み、−1（無限）と正の値は通す
        for loop_count in [0, -2] {
            let mut doc = base_doc();
            sb_mut(&mut doc).loop_count = loop_count;
            assert!(doc.validate().is_err(), "loop_count = {loop_count}");
        }
        for loop_count in [-1, 1, 3] {
            let mut doc = base_doc();
            sb_mut(&mut doc).loop_count = loop_count;
            assert!(doc.validate().is_ok(), "loop_count = {loop_count}");
        }
    }
}

// =============================================================
// V2: キーフレーム名重複検出
// =============================================================

mod v2_tests {
    use super::*;

    #[test]
    fn duplicate_keyframe_detected() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![
                    StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(1.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        keyframe: Some("visible".to_string()),
                        ..Default::default()
                    },
                    StoryboardEntry {
                        variable: Some("x".to_string()),
                        transition: Some(TransitionRef::Inline(TransitionDef {
                            from: None,
                            to: Some(TransitionValue::Scalar(2.0)),
                            relative_to: None,
                            easing: None,
                            delay: 0.0,
                            duration: Some(1.0),
                        })),
                        keyframe: Some("visible".to_string()), // duplicate!
                        ..Default::default()
                    },
                ],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::DuplicateKeyframe { storyboard, name }
            if storyboard == "sb1" && name == "visible"
        )));
    }
}

// =============================================================
// V3: 予約キーフレーム名 "start" 使用禁止
// =============================================================

mod v3_tests {
    use super::*;

    #[test]
    fn reserved_keyframe_start_rejected() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    keyframe: Some("start".to_string()), // reserved!
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::ReservedKeyframeName { storyboard, entry_index: 0, name }
                if storyboard == "sb1" && name == "start"
        )));
        // 場所がエラーの文に載る（#70）
        assert!(
            errors
                .iter()
                .any(|e| e.to_string().contains("storyboard 'sb1' entry 0")),
            "{errors:?}"
        );
    }

    /// Scalar の周回の待ちは min が 0 なので、負なら V15 の 1 件だけ（V16 の逆転は重ねない・#70）
    #[test]
    fn negative_scalar_loop_offset_reports_one_error() {
        let mut doc = minimal_valid_doc();
        doc.storyboard = BTreeMap::from([(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: Some(LoopOffset::Scalar(-1.0)),
                entry: vec![],
            },
        )]);
        let errors = doc.validate().unwrap_err();
        assert_eq!(
            errors,
            vec![DolaError::LoopOffsetNegativeMax {
                storyboard: "sb1".to_string(),
                value: -1.0,
            }]
        );
    }
}

// =============================================================
// V4: 未定義変数参照
// =============================================================

mod v4_tests {
    use super::*;

    #[test]
    fn undefined_variable_detected() {
        let mut doc = minimal_valid_doc();
        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("undefined_var".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedVariable { name, .. }
            if name == "undefined_var"
        )));
    }
}

// =============================================================
// V5: 未定義トランジション参照
// =============================================================

mod v5_tests {
    use super::*;

    #[test]
    fn undefined_transition_detected() {
        let mut doc = minimal_valid_doc();
        let mut variable = BTreeMap::new();
        variable.insert(
            "x".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );
        doc.variable = variable;

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "sb1".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: 1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset: None,
                entry: vec![StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Named("undefined_trans".to_string())),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedTransition { name, .. }
            if name == "undefined_trans"
        )));
    }
}
