//! core_types: Easing / Transition serde round-trip
//! Tasks 3.4, 4.4

use super::*;

// =============================================================
// Task 3.4: EasingFunction/EasingName/ParametricEasing serde round-trip
// =============================================================

mod easing_tests {
    use super::*;

    #[test]
    fn all_31_easing_names_json_roundtrip() {
        let names = vec![
            (EasingName::Linear, "linear"),
            (EasingName::QuadraticIn, "quadratic_in"),
            (EasingName::QuadraticOut, "quadratic_out"),
            (EasingName::QuadraticInOut, "quadratic_in_out"),
            (EasingName::CubicIn, "cubic_in"),
            (EasingName::CubicOut, "cubic_out"),
            (EasingName::CubicInOut, "cubic_in_out"),
            (EasingName::QuarticIn, "quartic_in"),
            (EasingName::QuarticOut, "quartic_out"),
            (EasingName::QuarticInOut, "quartic_in_out"),
            (EasingName::QuinticIn, "quintic_in"),
            (EasingName::QuinticOut, "quintic_out"),
            (EasingName::QuinticInOut, "quintic_in_out"),
            (EasingName::SineIn, "sine_in"),
            (EasingName::SineOut, "sine_out"),
            (EasingName::SineInOut, "sine_in_out"),
            (EasingName::CircularIn, "circular_in"),
            (EasingName::CircularOut, "circular_out"),
            (EasingName::CircularInOut, "circular_in_out"),
            (EasingName::ExponentialIn, "exponential_in"),
            (EasingName::ExponentialOut, "exponential_out"),
            (EasingName::ExponentialInOut, "exponential_in_out"),
            (EasingName::ElasticIn, "elastic_in"),
            (EasingName::ElasticOut, "elastic_out"),
            (EasingName::ElasticInOut, "elastic_in_out"),
            (EasingName::BackIn, "back_in"),
            (EasingName::BackOut, "back_out"),
            (EasingName::BackInOut, "back_in_out"),
            (EasingName::BounceIn, "bounce_in"),
            (EasingName::BounceOut, "bounce_out"),
            (EasingName::BounceInOut, "bounce_in_out"),
        ];

        assert_eq!(names.len(), 31, "Must have exactly 31 easing names");

        for (variant, expected_str) in &names {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(
                json,
                format!("\"{}\"", expected_str),
                "Failed for {:?}",
                variant
            );

            let deserialized: EasingName = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, deserialized);
        }
    }

    #[test]
    fn parametric_quadratic_bezier_json_roundtrip() {
        let easing = ParametricEasing::QuadraticBezier {
            x0: 0.0,
            x1: 0.5,
            x2: 1.0,
        };
        let json = serde_json::to_string(&easing).unwrap();
        assert!(json.contains(r#""type":"quadratic_bezier""#));
        let deserialized: ParametricEasing = serde_json::from_str(&json).unwrap();
        assert_eq!(easing, deserialized);
    }

    #[test]
    fn parametric_cubic_bezier_json_roundtrip() {
        let easing = ParametricEasing::CubicBezier {
            x0: 0.0,
            x1: 0.42,
            x2: 0.58,
            x3: 1.0,
        };
        let json = serde_json::to_string(&easing).unwrap();
        assert!(json.contains(r#""type":"cubic_bezier""#));
        let deserialized: ParametricEasing = serde_json::from_str(&json).unwrap();
        assert_eq!(easing, deserialized);
    }

    #[test]
    fn easing_function_named_untagged_deserialize() {
        // 文字列 → Named
        let json = r#""linear""#;
        let ef: EasingFunction = serde_json::from_str(json).unwrap();
        assert_eq!(ef, EasingFunction::Named(EasingName::Linear));
    }

    #[test]
    fn easing_function_parametric_untagged_deserialize() {
        // オブジェクト → Parametric
        let json = r#"{"type":"cubic_bezier","x0":0.0,"x1":0.42,"x2":0.58,"x3":1.0}"#;
        let ef: EasingFunction = serde_json::from_str(json).unwrap();
        assert_eq!(
            ef,
            EasingFunction::Parametric(ParametricEasing::CubicBezier {
                x0: 0.0,
                x1: 0.42,
                x2: 0.58,
                x3: 1.0,
            })
        );
    }
}

// =============================================================
// Task 4.4: TransitionValue/TransitionDef/TransitionRef serde round-trip
// =============================================================

mod transition_tests {
    use super::*;

    #[test]
    fn transition_value_scalar_json_roundtrip() {
        let val = TransitionValue::Scalar(5.0);
        let json = serde_json::to_string(&val).unwrap();
        let deserialized: TransitionValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn transition_value_dynamic_json_roundtrip() {
        let val = TransitionValue::Dynamic(DynamicValue::Map({
            let mut m = BTreeMap::new();
            m.insert(
                "path".to_string(),
                DynamicValue::String("img.png".to_string()),
            );
            m
        }));
        let json = serde_json::to_string(&val).unwrap();
        let deserialized: TransitionValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn transition_def_full_fields_json_roundtrip() {
        let def = TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            relative_to: None,
            easing: Some(EasingFunction::Named(EasingName::QuadraticInOut)),
            delay: 0.5,
            duration: Some(2.0),
        };
        let json = serde_json::to_string(&def).unwrap();
        let deserialized: TransitionDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def, deserialized);
    }

    #[test]
    fn transition_def_relative_to_json_roundtrip() {
        let def = TransitionDef {
            from: None,
            to: None,
            relative_to: Some(50.0),
            easing: Some(EasingFunction::Named(EasingName::Linear)),
            delay: 0.0,
            duration: Some(1.0),
        };
        let json = serde_json::to_string(&def).unwrap();
        let deserialized: TransitionDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def, deserialized);
    }

    #[test]
    fn transition_def_delay_default_json() {
        // delay 省略時はデフォルト 0.0
        let json = r#"{"to":1.0,"duration":1.0}"#;
        let def: TransitionDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.delay, 0.0);
    }

    #[test]
    fn transition_ref_named_json_roundtrip() {
        let tref = TransitionRef::Named("fade_in".to_string());
        let json = serde_json::to_string(&tref).unwrap();
        assert_eq!(json, r#""fade_in""#);
        let deserialized: TransitionRef = serde_json::from_str(&json).unwrap();
        assert_eq!(tref, deserialized);
    }

    #[test]
    fn transition_ref_inline_json_roundtrip() {
        let tref = TransitionRef::Inline(TransitionDef {
            from: None,
            to: Some(TransitionValue::Scalar(1.0)),
            relative_to: None,
            easing: None,
            delay: 0.0,
            duration: Some(1.5),
        });
        let json = serde_json::to_string(&tref).unwrap();
        let deserialized: TransitionRef = serde_json::from_str(&json).unwrap();
        assert_eq!(tref, deserialized);
    }
}
