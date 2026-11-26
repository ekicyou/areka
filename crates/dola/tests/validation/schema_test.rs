//! Validation tests for V1-V5 (schema version, keyframe names, variable/transition references)

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
// 複数ルールのエラー蓄積（fail-fast しないことの固定）
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
                        delay: TimeSpan::ZERO,
                        duration: Some(TimeSpan::from_millis(1000)),
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
// 数値の規則: 有限・符号・範囲を外れた値は InvalidNumber で拒む
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
                        delay: TimeSpan::ZERO,
                        duration: Some(TimeSpan::from_millis(1000)),
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
        let cases: [(Mutate, &str, &str, f64); 9] = [
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
                |d| sb_mut(d).loop_count = 0,
                "storyboard 'sb'",
                "loop_count",
                0.0,
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
                // 有限でも大きすぎる制御点は補間を inf／NaN にあふれさせる
                |d| {
                    trans(d).easing =
                        Some(EasingFunction::Parametric(ParametricEasing::CubicBezier {
                            x0: 0.0,
                            x1: 0.0,
                            x2: -1000.5,
                            x3: 1.0,
                        }))
                },
                "storyboard 'sb' entry 0",
                "easing.x2",
                -1000.5,
            ),
            (
                |d| {
                    sb_mut(d).loop_offset = Some(LoopOffset::Range(LoopOffsetRange {
                        min: TimeSpan::ZERO,
                        max: TimeSpan::from_millis(1000),
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
    fn bezier_points_at_the_bound_are_accepted() {
        let mut doc = base_doc();
        trans(&mut doc).easing = Some(EasingFunction::Parametric(ParametricEasing::CubicBezier {
            x0: -1000.0,
            x1: 1000.0,
            x2: 0.0,
            x3: 1.0,
        }));
        assert!(doc.validate().is_ok());
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
        let errors = doc.validate().unwrap_err();
        let fields: Vec<&str> = errors
            .iter()
            .filter_map(|e| match e {
                DolaError::InvalidNumber { field, .. } => Some(field.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(fields, ["time_scale", "loop_count"]);
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
// 時間の欄: 有限でない・負は読み込みの失敗、文書の値の上限は検査の誤り
// =============================================================

mod time_field_tests {
    use super::*;

    const LIMIT: i64 = 1 << 51;
    /// 2^51 刻み（100ns）の秒。上限の丁度
    const LIMIT_SECS: &str = "225179981.3685248";
    /// 2^51 - 1 刻みの秒。上限の内側の端
    const EDGE_SECS: &str = "225179981.3685247";

    fn span(ticks: i64) -> TimeSpan {
        TimeSpan::try_from_ticks(ticks).unwrap()
    }

    fn transition_json(delay: &str, duration: &str) -> String {
        format!(r#"{{"to":1.0,"delay":{delay},"duration":{duration}}}"#)
    }

    #[test]
    fn negative_or_non_finite_times_fail_to_load() {
        for json in [
            transition_json("0.0", "-3.0"),
            transition_json("-1.0", "1.0"),
            transition_json("0.0", "1e400"),
            transition_json("1e400", "1.0"),
            transition_json("0.0", "-1e400"),
        ] {
            assert!(
                serde_json::from_str::<TransitionDef>(&json).is_err(),
                "{json}"
            );
        }
    }

    #[test]
    fn seconds_round_trip_read_write_read() {
        let json = transition_json("0.25", "1.5");
        let first: TransitionDef = serde_json::from_str(&json).unwrap();
        assert_eq!(first.delay, TimeSpan::from_millis(250));
        assert_eq!(first.duration, Some(TimeSpan::from_millis(1500)));
        let written = serde_json::to_string(&first).unwrap();
        let second: TransitionDef = serde_json::from_str(&written).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn limit_edge_round_trips_and_the_limit_fails_everywhere() {
        let at_edge = TransitionDef {
            delay: span(LIMIT - 1),
            duration: Some(span(LIMIT - 1)),
            ..Default::default()
        };
        let json = serde_json::to_string(&at_edge).unwrap();
        assert_eq!(
            serde_json::from_str::<TransitionDef>(&json).unwrap(),
            at_edge
        );
        assert!(
            serde_json::from_str::<TransitionDef>(&transition_json(EDGE_SECS, EDGE_SECS)).is_ok()
        );

        for json in [
            transition_json(LIMIT_SECS, "1.0"),
            transition_json("0.0", LIMIT_SECS),
        ] {
            assert!(
                serde_json::from_str::<TransitionDef>(&json).is_err(),
                "{json}"
            );
        }

        let over_delay = TransitionDef {
            delay: span(LIMIT),
            ..Default::default()
        };
        let over_duration = TransitionDef {
            duration: Some(span(LIMIT)),
            ..Default::default()
        };
        for over in [&over_delay, &over_duration] {
            assert!(serde_json::to_string(over).is_err());
        }

        let mut doc = minimal_valid_doc();
        doc.transition.insert("t".to_string(), over_delay);
        doc.transition.insert("u".to_string(), over_duration);
        let errors = doc.validate().unwrap_err();
        let fields: Vec<(&str, &str)> = errors
            .iter()
            .map(|e| match e {
                DolaError::TimeOutOfDocumentRange { location, field } => {
                    (location.as_str(), field.as_str())
                }
                other => panic!("unexpected error: {other:?}"),
            })
            .collect();
        assert_eq!(
            fields,
            [("transition 't'", "delay"), ("transition 'u'", "duration")]
        );
    }

    #[test]
    fn keyframe_offset_limit_edges_on_both_sides() {
        let with_offset = |ticks: i64| KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("kf".to_string()),
            offset: TimeDelta::from_ticks(ticks),
        };
        for ticks in [LIMIT - 1, -(LIMIT - 1)] {
            let json = serde_json::to_string(&with_offset(ticks)).unwrap();
            assert_eq!(
                serde_json::from_str::<KeyframeRef>(&json).unwrap(),
                with_offset(ticks)
            );
        }
        for ticks in [LIMIT, -LIMIT] {
            assert!(serde_json::to_string(&with_offset(ticks)).is_err());
            let secs = if ticks < 0 {
                format!("-{LIMIT_SECS}")
            } else {
                LIMIT_SECS.to_string()
            };
            let json = format!(r#"{{"keyframes":"kf","offset":{secs}}}"#);
            assert!(serde_json::from_str::<KeyframeRef>(&json).is_err());
        }
    }

    #[test]
    fn storyboard_times_beyond_the_range_are_reported_by_validate() {
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
                        duration: Some(span(LIMIT)),
                        ..Default::default()
                    })),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("start".to_string()),
                        offset: TimeDelta::from_ticks(-LIMIT),
                    }),
                    ..Default::default()
                })
                .build(),
        );
        let errors = doc.validate().unwrap_err();
        let fields: Vec<&str> = errors
            .iter()
            .filter_map(|e| match e {
                DolaError::TimeOutOfDocumentRange { field, .. } => Some(field.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(fields, ["offset", "duration"]);
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
                            delay: TimeSpan::ZERO,
                            duration: Some(TimeSpan::from_millis(1000)),
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
                            delay: TimeSpan::ZERO,
                            duration: Some(TimeSpan::from_millis(1000)),
                        })),
                        keyframe: Some("visible".to_string()),
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
                        delay: TimeSpan::ZERO,
                        duration: Some(TimeSpan::from_millis(1000)),
                    })),
                    keyframe: Some("start".to_string()),
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
        // 場所がエラーの文に載る
        assert!(
            errors
                .iter()
                .any(|e| e.to_string().contains("storyboard 'sb1' entry 0")),
            "{errors:?}"
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
                        delay: TimeSpan::ZERO,
                        duration: Some(TimeSpan::from_millis(1000)),
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
