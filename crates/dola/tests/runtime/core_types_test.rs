//! ランタイムコア型のテスト — EvaluatedValue, RuntimeError

use dola::runtime::{EvaluatedValue, RuntimeError};

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
    /// 毎フレーム「変化あり」と判定し続ける現行挙動の特性化
    #[test]
    fn float_nan_is_never_equal_to_itself() {
        let v = EvaluatedValue::Float(f64::NAN);
        assert_ne!(v, v.clone());
        assert_ne!(
            EvaluatedValue::Float(f64::NAN),
            EvaluatedValue::Float(f64::NAN)
        );
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
            storyboard: "sb".to_string(),
            entry_index: 0,
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
                storyboard: "sb".to_string(),
                entry_index: 0,
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
