//! ランタイムコア型のテスト — InstanceState, EvaluatedValue, RuntimeError

use dola::InterruptionPolicy;
use dola::runtime::{EvaluatedValue, InstanceState, RuntimeError, StartResult};

// =============================================================================
// Task 1.1: InstanceState 状態遷移テスト
// =============================================================================

mod instance_state_transitions {
    use super::*;

    // --- 許可される遷移 ---

    #[test]
    fn created_to_playing() {
        let state = InstanceState::Created;
        assert_eq!(
            state.try_transition(InstanceState::Playing),
            Ok(InstanceState::Playing)
        );
    }

    #[test]
    fn playing_to_paused() {
        let state = InstanceState::Playing;
        assert_eq!(
            state.try_transition(InstanceState::Paused),
            Ok(InstanceState::Paused)
        );
    }

    #[test]
    fn paused_to_playing() {
        let state = InstanceState::Paused;
        assert_eq!(
            state.try_transition(InstanceState::Playing),
            Ok(InstanceState::Playing)
        );
    }

    #[test]
    fn playing_to_concluded() {
        let state = InstanceState::Playing;
        assert_eq!(
            state.try_transition(InstanceState::Concluded),
            Ok(InstanceState::Concluded)
        );
    }

    #[test]
    fn playing_to_cancelled() {
        let state = InstanceState::Playing;
        assert_eq!(
            state.try_transition(InstanceState::Cancelled),
            Ok(InstanceState::Cancelled)
        );
    }

    #[test]
    fn playing_to_trimmed() {
        let state = InstanceState::Playing;
        assert_eq!(
            state.try_transition(InstanceState::Trimmed),
            Ok(InstanceState::Trimmed)
        );
    }

    #[test]
    fn playing_to_compressed() {
        let state = InstanceState::Playing;
        assert_eq!(
            state.try_transition(InstanceState::Compressed),
            Ok(InstanceState::Compressed)
        );
    }

    #[test]
    fn paused_to_concluded() {
        let state = InstanceState::Paused;
        assert_eq!(
            state.try_transition(InstanceState::Concluded),
            Ok(InstanceState::Concluded)
        );
    }

    #[test]
    fn paused_to_cancelled() {
        let state = InstanceState::Paused;
        assert_eq!(
            state.try_transition(InstanceState::Cancelled),
            Ok(InstanceState::Cancelled)
        );
    }

    #[test]
    fn paused_to_trimmed() {
        let state = InstanceState::Paused;
        assert_eq!(
            state.try_transition(InstanceState::Trimmed),
            Ok(InstanceState::Trimmed)
        );
    }

    #[test]
    fn paused_to_compressed() {
        let state = InstanceState::Paused;
        assert_eq!(
            state.try_transition(InstanceState::Compressed),
            Ok(InstanceState::Compressed)
        );
    }

    // --- 拒否される遷移 ---

    #[test]
    fn created_to_paused_rejected() {
        let state = InstanceState::Created;
        assert_eq!(
            state.try_transition(InstanceState::Paused),
            Err(InstanceState::Created)
        );
    }

    #[test]
    fn created_to_concluded_rejected() {
        let state = InstanceState::Created;
        assert_eq!(
            state.try_transition(InstanceState::Concluded),
            Err(InstanceState::Created)
        );
    }

    #[test]
    fn concluded_to_playing_rejected() {
        let state = InstanceState::Concluded;
        assert_eq!(
            state.try_transition(InstanceState::Playing),
            Err(InstanceState::Concluded)
        );
    }

    #[test]
    fn cancelled_to_playing_rejected() {
        let state = InstanceState::Cancelled;
        assert_eq!(
            state.try_transition(InstanceState::Playing),
            Err(InstanceState::Cancelled)
        );
    }

    #[test]
    fn trimmed_to_playing_rejected() {
        let state = InstanceState::Trimmed;
        assert_eq!(
            state.try_transition(InstanceState::Playing),
            Err(InstanceState::Trimmed)
        );
    }

    #[test]
    fn compressed_to_playing_rejected() {
        let state = InstanceState::Compressed;
        assert_eq!(
            state.try_transition(InstanceState::Playing),
            Err(InstanceState::Compressed)
        );
    }

    #[test]
    fn concluded_to_concluded_rejected() {
        let state = InstanceState::Concluded;
        assert_eq!(
            state.try_transition(InstanceState::Concluded),
            Err(InstanceState::Concluded)
        );
    }

    #[test]
    fn playing_to_created_rejected() {
        let state = InstanceState::Playing;
        assert_eq!(
            state.try_transition(InstanceState::Created),
            Err(InstanceState::Playing)
        );
    }
}

mod instance_state_self_transitions {
    use super::*;

    /// D1b-T 追加: 自己遷移（同一状態への遷移）はすべて不正
    #[test]
    fn playing_to_playing_rejected() {
        assert_eq!(
            InstanceState::Playing.try_transition(InstanceState::Playing),
            Err(InstanceState::Playing)
        );
    }

    #[test]
    fn paused_to_paused_rejected() {
        assert_eq!(
            InstanceState::Paused.try_transition(InstanceState::Paused),
            Err(InstanceState::Paused)
        );
    }

    #[test]
    fn created_to_created_rejected() {
        assert_eq!(
            InstanceState::Created.try_transition(InstanceState::Created),
            Err(InstanceState::Created)
        );
    }
}

mod instance_state_properties {
    use super::*;

    #[test]
    fn terminal_states() {
        assert!(!InstanceState::Created.is_terminal());
        assert!(!InstanceState::Playing.is_terminal());
        assert!(!InstanceState::Paused.is_terminal());
        assert!(InstanceState::Concluded.is_terminal());
        assert!(InstanceState::Cancelled.is_terminal());
        assert!(InstanceState::Trimmed.is_terminal());
        assert!(InstanceState::Compressed.is_terminal());
    }

    #[test]
    fn from_policy_mapping() {
        assert_eq!(
            InstanceState::from_policy(InterruptionPolicy::Cancel),
            Some(InstanceState::Cancelled)
        );
        assert_eq!(
            InstanceState::from_policy(InterruptionPolicy::Conclude),
            Some(InstanceState::Concluded)
        );
        assert_eq!(
            InstanceState::from_policy(InterruptionPolicy::Trim),
            Some(InstanceState::Trimmed)
        );
        assert_eq!(
            InstanceState::from_policy(InterruptionPolicy::Compress),
            Some(InstanceState::Compressed)
        );
        assert_eq!(InstanceState::from_policy(InterruptionPolicy::Never), None);
    }

    /// D1b-V 追加: from_policy が返す Some(state) は全て終了状態である
    /// （conflict_resolver の expect / debug_assert が依存する不変条件）
    #[test]
    fn from_policy_results_are_terminal() {
        let policies = [
            InterruptionPolicy::Cancel,
            InterruptionPolicy::Conclude,
            InterruptionPolicy::Trim,
            InterruptionPolicy::Compress,
            InterruptionPolicy::Never,
        ];
        for policy in policies {
            if let Some(state) = InstanceState::from_policy(policy) {
                assert!(
                    state.is_terminal(),
                    "from_policy({policy:?}) returned non-terminal state {state:?}"
                );
            }
        }
    }
}

// =============================================================================
// Task 1.2: EvaluatedValue, RuntimeError テスト
// =============================================================================

mod evaluated_value_tests {
    use super::*;
    use dola::DynamicValue;
    use std::rc::Rc;

    #[test]
    fn float_variant() {
        // 任意の float 値でよい（円周率の意図はない・clippy::approx_constant の誤検知回避）。
        let v = EvaluatedValue::Float(1.25);
        assert_eq!(v, EvaluatedValue::Float(1.25));
    }

    #[test]
    fn integer_variant() {
        let v = EvaluatedValue::Integer(42);
        assert_eq!(v, EvaluatedValue::Integer(42));
    }

    #[test]
    fn object_variant_ptr_eq() {
        let rc = Rc::new(DynamicValue::String("hello".to_string()));
        let v1 = EvaluatedValue::Object(rc.clone());
        let v2 = EvaluatedValue::Object(rc.clone());
        // 同一 Rc ポインタなので等しい
        assert_eq!(v1, v2);
    }

    #[test]
    fn object_variant_different_rc_not_eq() {
        let v1 = EvaluatedValue::Object(Rc::new(DynamicValue::String("hello".to_string())));
        let v2 = EvaluatedValue::Object(Rc::new(DynamicValue::String("hello".to_string())));
        // 内容は同じだが異なる Rc ポインタなので等しくない
        assert_ne!(v1, v2);
    }

    /// D1b-T 追加: 異種バリアント間は値が数値的に同じでも常に不等
    #[test]
    fn cross_variant_never_equal() {
        assert_ne!(EvaluatedValue::Float(1.0), EvaluatedValue::Integer(1));
        assert_ne!(EvaluatedValue::Integer(1), EvaluatedValue::Float(1.0));
        assert_ne!(
            EvaluatedValue::Float(1.0),
            EvaluatedValue::Object(Rc::new(DynamicValue::Float(1.0)))
        );
    }

    /// D1b-V 追加: Float(NaN) は自分自身と不等（IEEE 754）。NaN 混入時は差分検出が
    /// 毎フレーム「変化あり」と判定し続ける現行挙動の特性化（proposals.md P8/P14 参照）
    #[test]
    fn float_nan_is_never_equal_to_itself() {
        let v = EvaluatedValue::Float(f64::NAN);
        assert_ne!(v, v.clone());
        assert_ne!(
            EvaluatedValue::Float(f64::NAN),
            EvaluatedValue::Float(f64::NAN)
        );
    }

    #[test]
    fn display_float() {
        // 小数 5 桁 → 6 桁への 0 埋めを検証する意図。円周率の意図はないため、
        // 桁数を保ったまま clippy::approx_constant が反応しない値を使う。
        let v = EvaluatedValue::Float(1.23456);
        let s = format!("{v}");
        assert_eq!(s, "1.234560");
    }

    #[test]
    fn display_integer() {
        let v = EvaluatedValue::Integer(-42);
        let s = format!("{v}");
        assert_eq!(s, "-42");
    }

    #[test]
    fn display_object() {
        let v = EvaluatedValue::Object(Rc::new(DynamicValue::String("test".to_string())));
        let s = format!("{v}");
        assert!(s.contains("test"));
    }
}

mod runtime_error_tests {
    use super::*;

    #[test]
    fn storyboard_not_found_display() {
        let err = RuntimeError::StoryboardNotFound("walk".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("walk"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn invalid_group_id_display() {
        let err = RuntimeError::InvalidGroupId(99);
        let msg = format!("{err}");
        assert!(msg.contains("99"));
    }

    #[test]
    fn zero_duration_with_loop_display() {
        let err = RuntimeError::ZeroDurationWithLoop {
            storyboard: "blink".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("blink"));
        assert!(msg.contains("zero duration"));
    }

    /// D1b-T 追加: Display 未検証バリアントの固定（InvalidLoopCount）
    #[test]
    fn invalid_loop_count_display() {
        let err = RuntimeError::InvalidLoopCount(-2);
        let msg = format!("{err}");
        assert!(msg.contains("-2"), "should contain the value: {msg}");
        assert!(
            msg.contains("loop_count"),
            "should mention loop_count: {msg}"
        );
    }

    /// D1b-T 追加: Display 未検証バリアントの固定（TooShortDurationWithInfiniteLoop）
    #[test]
    fn too_short_duration_with_infinite_loop_display() {
        let err = RuntimeError::TooShortDurationWithInfiniteLoop {
            storyboard: "spin".to_string(),
            duration: 0.05,
        };
        let msg = format!("{err}");
        assert!(
            msg.contains("spin"),
            "should contain storyboard name: {msg}"
        );
        assert!(
            msg.contains("0.050"),
            "should contain duration (3 digits): {msg}"
        );
        assert!(
            msg.contains("0.1"),
            "should contain MIN_LOOP_DURATION: {msg}"
        );
    }

    /// D1b-T 追加: Display 未検証バリアントの固定（CompileError）
    #[test]
    fn compile_error_display() {
        use dola::DolaError;
        let err = RuntimeError::CompileError(vec![DolaError::ReservedKeyframeName {
            name: "begin".to_string(),
        }]);
        let msg = format!("{err}");
        assert!(
            msg.contains("compile error"),
            "should mention compile error: {msg}"
        );
        assert!(
            msg.contains("begin"),
            "should contain inner error detail: {msg}"
        );
    }

    /// D1b-T 追加: Display 未検証バリアントの固定（InvalidVariableId）
    #[test]
    fn invalid_variable_id_display() {
        let err = RuntimeError::InvalidVariableId(-7);
        let msg = format!("{err}");
        assert!(msg.contains("-7"), "should contain the id: {msg}");
        assert!(
            msg.contains("variable_id"),
            "should mention variable_id: {msg}"
        );
    }

    #[test]
    fn runtime_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(RuntimeError::InvalidGroupId(1));
        assert!(err.to_string().contains("1"));
    }

    /// Req 4.4: `From<Vec<DolaError>>` による `?` 演算子自動変換
    #[test]
    fn from_vec_dola_error_conversion() {
        use dola::DolaError;

        fn fallible() -> Result<(), RuntimeError> {
            let errors: Vec<DolaError> = vec![DolaError::ReservedKeyframeName {
                name: "test".to_string(),
            }];
            // `?` 演算子で Vec<DolaError> → RuntimeError::CompileError に自動変換
            Err(errors)?
        }

        let result = fallible();
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            RuntimeError::CompileError(errors) => {
                assert_eq!(errors.len(), 1);
            }
            _ => panic!("expected CompileError variant"),
        }
    }
}

mod start_result_tests {
    use super::*;

    #[test]
    fn start_result_fields() {
        let r = StartResult {
            group_id: 1,
            end_time: 5.0,
            affected_group_ids: vec![],
        };
        assert_eq!(r.group_id, 1);
        assert_eq!(r.end_time, 5.0);
        assert!(r.affected_group_ids.is_empty());
    }

    #[test]
    fn start_result_infinity() {
        let r = StartResult {
            group_id: 2,
            end_time: f64::INFINITY,
            affected_group_ids: vec![],
        };
        assert!(r.end_time.is_infinite());
    }
}
