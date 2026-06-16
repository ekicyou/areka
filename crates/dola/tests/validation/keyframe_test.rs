//! Validation tests for V6, V8, V9 (keyframe reference, at/between exclusivity, pure KF entries)
//! Tasks 7.6, 8.6

use dola::*;
use std::collections::BTreeMap;

use super::common::minimal_valid_doc;

// =============================================================
// V6: キーフレーム参照検証（前方参照許可 + 暗黙的KF追跡）
// =============================================================

mod v6_tests {
    use super::*;

    #[test]
    fn forward_reference_ok() {
        // entry[0] で at="kf_from_entry_1" を参照, entry[1] で keyframe="kf_from_entry_1" を定義
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
                        at: Some(KeyframeRef::Single("kf_from_entry_1".to_string())),
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
                        keyframe: Some("kf_from_entry_1".to_string()),
                        ..Default::default()
                    },
                ],
            },
        );
        doc.storyboard = storyboard;

        assert!(doc.validate().is_ok());
    }

    #[test]
    fn undefined_keyframe_reference_detected() {
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
                    at: Some(KeyframeRef::Single("nonexistent".to_string())),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedKeyframe { name, .. }
            if name == "nonexistent"
        )));
    }

    #[test]
    fn implicit_keyframe_forward_reference_ok() {
        // entry[0] で at="__implicit_1" を参照、entry[1] は keyframe 省略 → 暗黙的KF __implicit_1
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
                        at: Some(KeyframeRef::Single("__implicit_1".to_string())),
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
                        ..Default::default()
                    },
                ],
            },
        );
        doc.storyboard = storyboard;

        assert!(doc.validate().is_ok());
    }

    #[test]
    fn at_start_reserved_keyframe_ok() {
        // at = "start" は予約KFなのでOK
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
                    at: Some(KeyframeRef::Single("start".to_string())),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        assert!(doc.validate().is_ok());
    }
}

// =============================================================
// V6 追加（D3-T）: KeyframeRef 全バリアント × between 参照の検証
// =============================================================

mod v6_ref_shape_tests {
    use super::*;

    /// ヘルパー: Float 変数 "x" への to=val トランジションエントリ
    fn float_entry(val: f64) -> StoryboardEntry {
        StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: None,
                to: Some(TransitionValue::Scalar(val)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        }
    }

    /// ヘルパー: 変数 "x" 定義済みドキュメントに sb1 として entries を設定
    fn doc_with_entries(entries: Vec<StoryboardEntry>) -> DolaDocument {
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
                entry: entries,
            },
        );
        doc.storyboard = storyboard;
        doc
    }

    #[test]
    fn at_multiple_with_one_undefined_detected() {
        // Multiple のうち 1 件のみ未定義 → 未定義分だけがエラーになる
        let mut e0 = float_entry(1.0);
        e0.keyframe = Some("kf1".to_string());
        let mut e1 = float_entry(2.0);
        e1.at = Some(KeyframeRef::Multiple(vec![
            "kf1".to_string(),
            "missing".to_string(),
        ]));
        let doc = doc_with_entries(vec![e0, e1]);

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedKeyframe { storyboard, name }
            if storyboard == "sb1" && name == "missing"
        )));
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, DolaError::UndefinedKeyframe { name, .. } if name == "kf1"))
        );
    }

    #[test]
    fn at_with_offset_single_undefined_detected() {
        // WithOffset(Single) の参照先未定義もエラーになる
        let mut e0 = float_entry(1.0);
        e0.at = Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single("nonexistent".to_string()),
            offset: 0.5,
        });
        let doc = doc_with_entries(vec![e0]);

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedKeyframe { name, .. }
            if name == "nonexistent"
        )));
    }

    #[test]
    fn at_with_offset_multiple_all_known_ok() {
        // WithOffset(Multiple) で全参照先が既知（予約 "start" + 明示 KF）→ OK
        let mut e0 = float_entry(1.0);
        e0.keyframe = Some("kf1".to_string());
        let mut e1 = float_entry(2.0);
        e1.at = Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Multiple(vec!["start".to_string(), "kf1".to_string()]),
            offset: 0.25,
        });
        let doc = doc_with_entries(vec![e0, e1]);

        assert!(doc.validate().is_ok());
    }

    #[test]
    fn between_undefined_from_and_to_both_detected() {
        // between の from / to 両方が未定義 → 2 件のエラーを蓄積
        let mut e0 = float_entry(1.0);
        e0.between = Some(BetweenKeyframes {
            from: "no_from".to_string(),
            to: "no_to".to_string(),
        });
        let doc = doc_with_entries(vec![e0]);

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedKeyframe { name, .. } if name == "no_from"
        )));
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::UndefinedKeyframe { name, .. } if name == "no_to"
        )));
    }
}

// =============================================================
// V8: at と between は排他
// =============================================================

mod v8_tests {
    use super::*;

    #[test]
    fn at_and_between_mutually_exclusive() {
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
                    // Need a KF first
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
                        keyframe: Some("kf1".to_string()),
                        ..Default::default()
                    },
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
                        at: Some(KeyframeRef::Single("kf1".to_string())),
                        between: Some(BetweenKeyframes {
                            from: "start".to_string(),
                            to: "kf1".to_string(),
                        }),
                        ..Default::default()
                    },
                ],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::InvalidEntry { reason, .. }
            if reason.contains("at and between are mutually exclusive")
        )));
    }
}

// =============================================================
// V9: 純粋KFエントリ（variable/transition なし）→ keyframe 必須
// =============================================================

mod v9_tests {
    use super::*;

    #[test]
    fn empty_entry_without_keyframe_error() {
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
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::InvalidEntry { reason, .. }
            if reason.contains("must have keyframe")
        )));
    }

    #[test]
    fn pure_keyframe_entry_ok() {
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
                    keyframe: Some("sync_point".to_string()),
                    ..Default::default()
                }],
            },
        );
        doc.storyboard = storyboard;

        assert!(doc.validate().is_ok());
    }
}
