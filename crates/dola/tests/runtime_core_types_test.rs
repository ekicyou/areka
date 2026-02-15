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
}

// =============================================================================
// Task 1.2: EvaluatedValue, RuntimeError テスト
// =============================================================================

mod evaluated_value_tests {
    use super::*;
    use dola::DynamicValue;

    #[test]
    fn float_variant() {
        let v = EvaluatedValue::Float(3.14);
        assert_eq!(v, EvaluatedValue::Float(3.14));
    }

    #[test]
    fn integer_variant() {
        let v = EvaluatedValue::Integer(42);
        assert_eq!(v, EvaluatedValue::Integer(42));
    }

    #[test]
    fn object_variant() {
        let v = EvaluatedValue::Object(DynamicValue::String("hello".to_string()));
        assert_eq!(
            v,
            EvaluatedValue::Object(DynamicValue::String("hello".to_string()))
        );
    }

    #[test]
    fn display_float() {
        let v = EvaluatedValue::Float(3.14159);
        let s = format!("{v}");
        assert_eq!(s, "3.141590");
    }

    #[test]
    fn display_integer() {
        let v = EvaluatedValue::Integer(-42);
        let s = format!("{v}");
        assert_eq!(s, "-42");
    }

    #[test]
    fn display_object() {
        let v = EvaluatedValue::Object(DynamicValue::String("test".to_string()));
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
    fn document_parse_error_display() {
        let err = RuntimeError::DocumentParseError("invalid toml".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("invalid toml"));
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

    #[test]
    fn runtime_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(RuntimeError::InvalidGroupId(1));
        assert!(err.to_string().contains("1"));
    }
}

mod start_result_tests {
    use super::*;

    #[test]
    fn start_result_fields() {
        let r = StartResult {
            group_id: 1,
            end_time: 5.0,
        };
        assert_eq!(r.group_id, 1);
        assert_eq!(r.end_time, 5.0);
    }

    #[test]
    fn start_result_infinity() {
        let r = StartResult {
            group_id: 2,
            end_time: f64::INFINITY,
        };
        assert!(r.end_time.is_infinite());
    }
}
