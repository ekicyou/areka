//! LoopOffset serde ラウンドトリップ
use super::*;

// =============================================================
// LoopOffset serde ラウンドトリップ
// =============================================================

mod serde_tests {
    use super::*;

    #[test]
    fn scalar_shorthand_deserialize() {
        // 数値 3.0 → LoopOffset::Scalar(3 秒)
        let json = "3.0";
        let lo: LoopOffset = serde_json::from_str(json).unwrap();
        assert_eq!(lo, LoopOffset::Scalar(TimeSpan::from_millis(3000)));
    }

    #[test]
    fn scalar_shorthand_round_trip() {
        let lo = LoopOffset::Scalar(TimeSpan::from_millis(5000));
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
                assert_eq!(r.min, TimeSpan::from_millis(1000));
                assert_eq!(r.max, TimeSpan::from_millis(5000));
                assert_eq!(r.easing, EasingFunction::Named(EasingName::QuadraticOut));
            }
            _ => panic!("Expected LoopOffset::Range"),
        }
    }

    #[test]
    fn object_form_round_trip() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: TimeSpan::from_millis(1000),
            max: TimeSpan::from_millis(5000),
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
                assert_eq!(r.min, TimeSpan::ZERO);
                assert_eq!(r.max, TimeSpan::from_millis(5000));
            }
            _ => panic!("Expected LoopOffset::Range"),
        }
    }

    #[test]
    fn parametric_easing_round_trip() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: TimeSpan::from_millis(500),
            max: TimeSpan::from_millis(3000),
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
        assert_eq!(
            sb.loop_offset,
            Some(LoopOffset::Scalar(TimeSpan::from_millis(5000)))
        );
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
                assert_eq!(r.min, TimeSpan::from_millis(1000));
                assert_eq!(r.max, TimeSpan::from_millis(5000));
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

    #[test]
    fn negative_or_non_finite_offsets_fail_to_load() {
        // 失敗した事だけを見る（untagged の文は「どの形にも合わない」になる）
        for json in [
            "-1.0",
            r#"{ "min": -1.0, "max": 5.0 }"#,
            r#"{ "min": 0.0, "max": -3.0 }"#,
            r#"{ "max": 1e400 }"#,
            r#"{ "min": 1e400, "max": 1.0 }"#,
        ] {
            assert!(serde_json::from_str::<LoopOffset>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn offsets_beyond_the_document_range_fail_to_load() {
        for json in [
            "225179981.3685248",
            r#"{ "max": 225179981.3685248 }"#,
            r#"{ "min": 225179981.3685248, "max": 225179981.3685248 }"#,
        ] {
            assert!(serde_json::from_str::<LoopOffset>(json).is_err(), "{json}");
        }
        assert!(serde_json::from_str::<LoopOffset>("225179981.3685247").is_ok());
    }

    #[test]
    fn offsets_at_the_limit_edge_round_trip() {
        let edge = TimeSpan::try_from_ticks((1 << 51) - 1).unwrap();
        let lo = LoopOffset::Scalar(edge);
        let json = serde_json::to_string(&lo).unwrap();
        assert_eq!(serde_json::from_str::<LoopOffset>(&json).unwrap(), lo);
    }

    #[test]
    fn offsets_beyond_the_document_range_fail_to_write() {
        let over = TimeSpan::try_from_ticks(1 << 51).unwrap();
        assert!(serde_json::to_string(&LoopOffset::Scalar(over)).is_err());
        let range = LoopOffset::Range(LoopOffsetRange {
            min: TimeSpan::ZERO,
            max: over,
            easing: EasingFunction::Named(EasingName::Linear),
        });
        assert!(serde_json::to_string(&range).is_err());
    }
}
