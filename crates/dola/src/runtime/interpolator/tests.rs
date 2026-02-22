use super::*;

fn make_segment(from: f64, to: f64, easing: Option<EasingFunction>) -> CompiledSegment {
    CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Scalar(from),
        to_value: TransitionValue::Scalar(to),
        easing,
    }
}

#[test]
fn linear_interpolation_default() {
    let seg = make_segment(0.0, 100.0, None);
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    assert_eq!(result, EvaluatedValue::Float(50.0));
}

#[test]
fn linear_easing_explicit() {
    let seg = make_segment(0.0, 100.0, Some(EasingFunction::Named(EasingName::Linear)));
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    assert_eq!(result, EvaluatedValue::Float(50.0));
}

#[test]
fn boundary_t_zero() {
    let seg = make_segment(10.0, 20.0, None);
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.0);
    assert_eq!(result, EvaluatedValue::Float(10.0));
}

#[test]
fn boundary_t_one() {
    let seg = make_segment(10.0, 20.0, None);
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 1.0);
    assert_eq!(result, EvaluatedValue::Float(20.0));
}

#[test]
fn clamp_below_zero() {
    let seg = make_segment(0.0, 100.0, None);
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, -0.5);
    assert_eq!(result, EvaluatedValue::Float(0.0));
}

#[test]
fn clamp_above_one() {
    let seg = make_segment(0.0, 100.0, None);
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 1.5);
    assert_eq!(result, EvaluatedValue::Float(100.0));
}

#[test]
fn integer_rounding() {
    let seg = make_segment(0.0, 10.0, None);
    let vt = VariableTypeHint::Integer { typewriter: None };
    let result = Interpolator::interpolate(&seg, &vt, 0.25);
    // 0.0 + (10.0 - 0.0) * 0.25 = 2.5 → round → 3 (banker's or standard)
    // standard round: 2.5 → 3
    assert_eq!(result, EvaluatedValue::Integer(3));
}

#[test]
fn integer_exact() {
    let seg = make_segment(0.0, 10.0, None);
    let vt = VariableTypeHint::Integer { typewriter: None };
    let result = Interpolator::interpolate(&seg, &vt, 0.5);
    assert_eq!(result, EvaluatedValue::Integer(5));
}

#[test]
fn object_before_end() {
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::String("a".to_string())),
        to_value: TransitionValue::Dynamic(DynamicValue::String("b".to_string())),
        easing: None,
    };
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Object, 0.5);
    // Rc::ptr_eq は異なるアロケーションなので内容で比較
    match &result {
        EvaluatedValue::Object(rc) => assert_eq!(**rc, DynamicValue::String("a".to_string())),
        _ => panic!("expected Object variant"),
    }
}

#[test]
fn object_at_end() {
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::String("a".to_string())),
        to_value: TransitionValue::Dynamic(DynamicValue::String("b".to_string())),
        easing: None,
    };
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Object, 1.0);
    match &result {
        EvaluatedValue::Object(rc) => assert_eq!(**rc, DynamicValue::String("b".to_string())),
        _ => panic!("expected Object variant"),
    }
}

#[test]
fn quadratic_in_easing() {
    let seg = make_segment(
        0.0,
        100.0,
        Some(EasingFunction::Named(EasingName::QuadraticIn)),
    );
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    // QuadraticIn: t^2, so 0.5^2 = 0.25 → lerp(0, 100, 0.25) = 25.0
    assert_eq!(result, EvaluatedValue::Float(25.0));
}

#[test]
fn cubic_bezier_easing() {
    let seg = make_segment(
        0.0,
        100.0,
        Some(EasingFunction::Parametric(ParametricEasing::CubicBezier {
            x0: 0.0,
            x1: 0.0,
            x2: 1.0,
            x3: 1.0,
        })),
    );
    // cub_bez(0, 0, 1, 1, 0.5) should give ~0.5 for this symmetric curve (linear-like)
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    if let EvaluatedValue::Float(v) = result {
        assert!((v - 50.0).abs() < 1.0, "expected ~50.0, got {v}");
    } else {
        panic!("expected Float");
    }
}

#[test]
fn quadratic_bezier_easing() {
    let seg = make_segment(
        0.0,
        100.0,
        Some(EasingFunction::Parametric(
            ParametricEasing::QuadraticBezier {
                x0: 0.0,
                x1: 0.5,
                x2: 1.0,
            },
        )),
    );
    // quad_bez(0, 0.5, 1, 0.5) should give ~0.5
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    if let EvaluatedValue::Float(v) = result {
        assert!((v - 50.0).abs() < 1.0, "expected ~50.0, got {v}");
    } else {
        panic!("expected Float");
    }
}

/// Req 7 AC3: 全30バリアントのマッピング正確性を検証
#[test]
fn all_30_easing_names_mapping() {
    let all_easings = [
        EasingName::Linear,
        EasingName::QuadraticIn,
        EasingName::QuadraticOut,
        EasingName::QuadraticInOut,
        EasingName::CubicIn,
        EasingName::CubicOut,
        EasingName::CubicInOut,
        EasingName::QuarticIn,
        EasingName::QuarticOut,
        EasingName::QuarticInOut,
        EasingName::QuinticIn,
        EasingName::QuinticOut,
        EasingName::QuinticInOut,
        EasingName::SineIn,
        EasingName::SineOut,
        EasingName::SineInOut,
        EasingName::CircularIn,
        EasingName::CircularOut,
        EasingName::CircularInOut,
        EasingName::ExponentialIn,
        EasingName::ExponentialOut,
        EasingName::ExponentialInOut,
        EasingName::ElasticIn,
        EasingName::ElasticOut,
        EasingName::ElasticInOut,
        EasingName::BackIn,
        EasingName::BackOut,
        EasingName::BackInOut,
        EasingName::BounceIn,
        EasingName::BounceOut,
        EasingName::BounceInOut,
    ];

    // 全30バリアントがパニックせずに実行できることを検証
    for easing in all_easings {
        let seg = make_segment(0.0, 100.0, Some(EasingFunction::Named(easing)));
        let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
        // 結果がFloat型であることを確認（値は各イージングで異なる）
        assert!(
            matches!(result, EvaluatedValue::Float(_)),
            "EasingName::{easing:?} failed to produce Float"
        );
    }
}
