//! トリガー機能のテスト — 5.2: バリデーションユニットテスト（V9更新, V14t–V18t）

use super::common::minimal_trigger_doc;
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, DolaError, StoryboardBuilder, StoryboardEntry,
    TransitionDef, TransitionRef, TransitionValue, Validate,
};

// ============================================================
// 5.2: バリデーションユニットテスト
// ============================================================

#[cfg(test)]
mod validation_tests {
    use super::*;

    /// V9 更新: trigger_storyboard のみのエントリは V9 エラーにならない
    #[test]
    fn v9_trigger_only_entry_is_valid() {
        let doc = minimal_trigger_doc("parent", "child");
        assert!(doc.validate().is_ok());
    }

    /// V9: variable も transition も keyframe も trigger もない空エントリはエラー
    #[test]
    fn v9_no_variable_no_trigger_is_error() {
        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                // 何も設定しない空エントリ
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .storyboard("test", sb)
            .build();

        // V9: entry without variable/transition must have keyframe or trigger_storyboard
        assert!(doc.is_err());
    }

    /// V15t: A→B→C→A 循環検出
    #[test]
    fn v15t_cycle_a_b_c_a() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .build();

        let sb_b = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_c".to_string()),
                ..Default::default()
            })
            .build();

        let sb_c = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_a".to_string()),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", sb_b)
            .storyboard("sb_c", sb_c)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerCycle { .. }))
        );
    }

    /// V15t: 非循環チェーン A→B→C は OK
    #[test]
    fn v15t_no_cycle_chain_is_valid() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .build();

        let sb_b = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_c".to_string()),
                ..Default::default()
            })
            .build();

        let sb_c = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", sb_b)
            .storyboard("sb_c", sb_c)
            .build();

        assert!(
            doc.is_ok(),
            "Non-cyclic chain should be valid: {:?}",
            doc.err()
        );
    }

    /// V16t: trigger + variable の同時指定はエラー
    #[test]
    fn v16t_trigger_with_variable_is_error() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("child".to_string()),
                variable: Some("opacity".to_string()),
                ..Default::default()
            })
            .build();

        let child_sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("parent", sb)
            .storyboard("child", child_sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerExclusiveViolation { .. }))
        );
    }

    /// V16t: trigger + transition の同時指定はエラー
    #[test]
    fn v16t_trigger_with_transition_is_error() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("child".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let child_sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("parent", sb)
            .storyboard("child", child_sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerExclusiveViolation { .. }))
        );
    }

    /// V15t（#49）: 深いトリガーの鎖を 64 KiB のスタックで検証できる
    ///
    /// `dfs_detect_cycle` が鎖の深さに比例してコールスタックを使うと、このスレッドは
    /// スタックを使い切って落ちる（再帰に戻すと赤）。
    #[test]
    fn v15t_long_chain_validates_ok_on_small_stack() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };
        let mut builder = DolaDocumentBuilder::new("1.0").variable("opacity", opacity_var);

        const CHAIN_LEN: usize = 20_000;
        for i in 0..CHAIN_LEN - 1 {
            let sb = StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    trigger_storyboard: Some(format!("sb_{:05}", i + 1)),
                    ..Default::default()
                })
                .build();
            builder = builder.storyboard(format!("sb_{i:05}"), sb);
        }
        // 終端: 通常のトランジションエントリ
        let terminal = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();
        builder = builder.storyboard(format!("sb_{:05}", CHAIN_LEN - 1), terminal);

        let doc = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || builder.build().map(|_| ()))
            .unwrap()
            .join()
            .expect("validation must not exhaust a 64 KiB stack");
        assert!(doc.is_ok(), "long chain should validate: {:?}", doc.err());
    }

    /// V15t 特性化（D3-T）: 循環エラーのパスは閉路（先頭 == 末尾）として報告される
    #[test]
    fn v15t_cycle_path_is_closed_and_contains_members() {
        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .build();
        let sb_b = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_a".to_string()),
                ..Default::default()
            })
            .build();

        let errors = DolaDocumentBuilder::new("1.0")
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", sb_b)
            .build()
            .unwrap_err();

        let cycle = errors
            .iter()
            .find_map(|e| match e {
                DolaError::TriggerCycle { cycle } => Some(cycle),
                _ => None,
            })
            .expect("TriggerCycle error expected");

        // 根は名前の順で回るので、閉路は sb_a から始まる
        assert_eq!(cycle.as_slice(), ["sb_a", "sb_b", "sb_a"]);
    }

    /// V15t（#6）: 循環を見つけた根の道とスタックの印を次の根へ持ち越さない
    ///
    /// {a→b, b→a, c→a} で、閉路に入らない c が閉路の一員として 2 件目に報告されない。
    #[test]
    fn trigger_cycle_reported_once_without_bystander() {
        let trigger_to = |target: &str| {
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    trigger_storyboard: Some(target.to_string()),
                    ..Default::default()
                })
                .build()
        };

        let errors = DolaDocumentBuilder::new("1.0")
            .storyboard("a", trigger_to("b"))
            .storyboard("b", trigger_to("a"))
            .storyboard("c", trigger_to("a"))
            .build()
            .unwrap_err();

        let cycles: Vec<&[String]> = errors
            .iter()
            .filter_map(|e| match e {
                DolaError::TriggerCycle { cycle } => Some(cycle.as_slice()),
                _ => None,
            })
            .collect();
        assert_eq!(cycles, [["a", "b", "a"].map(String::from).as_slice()]);
    }

    /// V15t（D3-T）: ダイヤモンド型（合流のみで閉路なし）は循環と誤検出されない
    #[test]
    fn v15t_diamond_join_is_not_a_cycle() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };
        // a → b, a → c, b → d, c → d（d は終端）
        let sb_a = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_b".to_string()),
                ..Default::default()
            })
            .entry(StoryboardEntry {
                trigger_storyboard: Some("sb_c".to_string()),
                ..Default::default()
            })
            .build();
        fn to_d() -> dola::Storyboard {
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    trigger_storyboard: Some("sb_d".to_string()),
                    ..Default::default()
                })
                .build()
        }
        let sb_d = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(1.0),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("sb_a", sb_a)
            .storyboard("sb_b", to_d())
            .storyboard("sb_c", to_d())
            .storyboard("sb_d", sb_d)
            .build();

        assert!(doc.is_ok(), "diamond join is not a cycle: {:?}", doc.err());
    }

    /// V14t/V15t 特性化（D3-T）: 自己参照は SelfReference と TriggerCycle の両方を報告する
    #[test]
    fn v14t_self_reference_also_reports_cycle() {
        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("self_ref".to_string()),
                ..Default::default()
            })
            .build();

        let errors = DolaDocumentBuilder::new("1.0")
            .storyboard("self_ref", sb)
            .build()
            .unwrap_err();

        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerSelfReference { .. }))
        );
        // 自己ループは長さ 2 の閉路 ["self_ref", "self_ref"] としても検出される
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::TriggerCycle { cycle }
            if cycle.as_slice() == ["self_ref".to_string(), "self_ref".to_string()]
        )));
    }

    /// V18t: 存在しないストーリーボードへのトリガーはエラー
    #[test]
    fn v18t_trigger_target_not_found() {
        let opacity_var = AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        };

        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                trigger_storyboard: Some("nonexistent".to_string()),
                ..Default::default()
            })
            .build();

        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", opacity_var)
            .storyboard("parent", sb)
            .build();

        assert!(doc.is_err());
        let errors = doc.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DolaError::TriggerTargetNotFound { .. }))
        );
    }

    /// #31: トリガー以外のエントリの trigger_start_offset は黙って捨てられるので拒む
    #[test]
    fn trigger_start_offset_on_non_trigger_is_rejected() {
        let sb = StoryboardBuilder::new()
            .entry(StoryboardEntry {
                keyframe: Some("kf".to_string()),
                trigger_start_offset: Some(1.0),
                ..Default::default()
            })
            .build();

        let errors = DolaDocumentBuilder::new("1.0")
            .storyboard("parent", sb)
            .build()
            .unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::InvalidEntry { entry_index: 0, reason, .. }
            if reason.contains("trigger_start_offset")
        )));
    }
}
