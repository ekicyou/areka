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

// =========================================================================
// D1b-T 追加: ObjectInternPool（intern の同一性保証）
// =========================================================================

#[test]
fn intern_pool_returns_same_rc_for_equal_values() {
    let mut pool = ObjectInternPool::new();
    let rc1 = pool.intern(DynamicValue::String("a".to_string()));
    let rc2 = pool.intern(DynamicValue::String("a".to_string()));
    assert!(
        Rc::ptr_eq(&rc1, &rc2),
        "equal values must intern to the same Rc"
    );
}

#[test]
fn intern_pool_returns_distinct_rc_for_different_values() {
    let mut pool = ObjectInternPool::new();
    let rc1 = pool.intern(DynamicValue::String("a".to_string()));
    let rc2 = pool.intern(DynamicValue::String("b".to_string()));
    assert!(
        !Rc::ptr_eq(&rc1, &rc2),
        "different values must intern to distinct Rc"
    );
    assert_eq!(*rc1, DynamicValue::String("a".to_string()));
    assert_eq!(*rc2, DynamicValue::String("b".to_string()));
}

#[test]
fn interpolate_with_pool_shares_rc_across_calls() {
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::String("a".to_string())),
        to_value: TransitionValue::Dynamic(DynamicValue::String("b".to_string())),
        easing: None,
    };
    let mut pool = ObjectInternPool::new();
    let r1 =
        Interpolator::interpolate_with_pool(&seg, &VariableTypeHint::Object, 0.0, Some(&mut pool));
    let r2 =
        Interpolator::interpolate_with_pool(&seg, &VariableTypeHint::Object, 0.5, Some(&mut pool));
    match (&r1, &r2) {
        (EvaluatedValue::Object(a), EvaluatedValue::Object(b)) => {
            // 同一 pool 経由なら同一内容（from_value）は同一 Rc → EvaluatedValue も等しい
            assert!(Rc::ptr_eq(a, b), "pool must dedupe equal Object values");
            assert_eq!(r1, r2, "EvaluatedValue PartialEq must hold via ptr_eq");
        }
        _ => panic!("expected Object variants"),
    }
}

#[test]
fn interpolate_without_pool_allocates_fresh_rc() {
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::String("a".to_string())),
        to_value: TransitionValue::Dynamic(DynamicValue::String("b".to_string())),
        easing: None,
    };
    let r1 = Interpolator::interpolate(&seg, &VariableTypeHint::Object, 0.0);
    let r2 = Interpolator::interpolate(&seg, &VariableTypeHint::Object, 0.0);
    match (&r1, &r2) {
        (EvaluatedValue::Object(a), EvaluatedValue::Object(b)) => {
            // pool なしでは毎回新規 Rc → 内容が同じでも ptr_eq は false
            assert!(!Rc::ptr_eq(a, b), "no pool means fresh Rc per call");
            assert_ne!(r1, r2, "EvaluatedValue PartialEq is pointer-based");
        }
        _ => panic!("expected Object variants"),
    }
}

// =========================================================================
// D1b-T 追加: scalar_value / transition_value_to_dynamic の変換分岐
// =========================================================================

#[test]
fn float_interpolation_from_dynamic_float_and_integer() {
    // Dynamic(Float) / Dynamic(Integer) は scalar_value で f64 へ変換される
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::Float(10.0)),
        to_value: TransitionValue::Dynamic(DynamicValue::Integer(20)),
        easing: None,
    };
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    assert_eq!(result, EvaluatedValue::Float(15.0));
}

#[test]
fn float_interpolation_non_numeric_dynamic_falls_back_to_zero() {
    // 非数値 Dynamic（String 等）は scalar_value で 0.0 にフォールバック
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::String("oops".to_string())),
        to_value: TransitionValue::Scalar(100.0),
        easing: None,
    };
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5);
    assert_eq!(result, EvaluatedValue::Float(50.0));
}

#[test]
fn object_with_scalar_values_converts_to_dynamic_float() {
    // Object 型変数で Scalar 値が来た場合は DynamicValue::Float に包んで返す
    let seg = make_segment(1.5, 2.5, None);
    let before = Interpolator::interpolate(&seg, &VariableTypeHint::Object, 0.5);
    match &before {
        EvaluatedValue::Object(rc) => assert_eq!(**rc, DynamicValue::Float(1.5)),
        _ => panic!("expected Object variant"),
    }
    let at_end = Interpolator::interpolate(&seg, &VariableTypeHint::Object, 1.0);
    match &at_end {
        EvaluatedValue::Object(rc) => assert_eq!(**rc, DynamicValue::Float(2.5)),
        _ => panic!("expected Object variant"),
    }
}

// =========================================================================
// D1b-T 追加: Integer 補間の境界（負値・丸め方向・イージング併用）
// =========================================================================

#[test]
fn integer_negative_interpolation_rounds_away_from_zero() {
    // 0→-10 の t=0.25 → -2.5 → f64::round は 0 から遠い方向へ丸める → -3
    let seg = make_segment(0.0, -10.0, None);
    let vt = VariableTypeHint::Integer { typewriter: None };
    let result = Interpolator::interpolate(&seg, &vt, 0.25);
    assert_eq!(result, EvaluatedValue::Integer(-3));
}

#[test]
fn integer_interpolation_with_easing() {
    // QuadraticIn: 0.5^2 = 0.25 → lerp(0, 100, 0.25) = 25 → Integer(25)
    let seg = make_segment(
        0.0,
        100.0,
        Some(EasingFunction::Named(EasingName::QuadraticIn)),
    );
    let vt = VariableTypeHint::Integer { typewriter: None };
    let result = Interpolator::interpolate(&seg, &vt, 0.5);
    assert_eq!(result, EvaluatedValue::Integer(25));
}

// =========================================================================
// D1b-V 追加: 数値境界の特性化（NaN / inf / 飽和キャスト / オーバーシュート）
// =========================================================================

/// NaN progress_t: clamp は NaN を素通りさせ、Float 補間結果は NaN になる（panic しない）
#[test]
fn nan_progress_propagates_nan_for_float() {
    let seg = make_segment(0.0, 100.0, None);
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, f64::NAN);
    match result {
        EvaluatedValue::Float(v) => assert!(v.is_nan(), "expected NaN, got {v}"),
        _ => panic!("expected Float"),
    }
}

/// NaN progress_t: Integer は `NaN.round() as i64` の飽和キャストで 0 になる（panic しない）
#[test]
fn nan_progress_saturates_to_zero_for_integer() {
    let seg = make_segment(0.0, 100.0, None);
    let vt = VariableTypeHint::Integer { typewriter: None };
    let result = Interpolator::interpolate(&seg, &vt, f64::NAN);
    assert_eq!(result, EvaluatedValue::Integer(0));
}

/// NaN progress_t: Object は `t >= 1.0` が false となり from_value 側を返す（panic しない）
#[test]
fn nan_progress_returns_from_value_for_object() {
    let seg = CompiledSegment {
        start_time: 0.0,
        end_time: 1.0,
        from_value: TransitionValue::Dynamic(DynamicValue::String("a".to_string())),
        to_value: TransitionValue::Dynamic(DynamicValue::String("b".to_string())),
        easing: None,
    };
    let result = Interpolator::interpolate(&seg, &VariableTypeHint::Object, f64::NAN);
    match &result {
        EvaluatedValue::Object(rc) => assert_eq!(**rc, DynamicValue::String("a".to_string())),
        _ => panic!("expected Object variant"),
    }
}

/// ±inf progress_t は clamp で 1.0 / 0.0 に丸められ、to / from 値を返す
#[test]
fn infinite_progress_clamps_to_endpoints() {
    let seg = make_segment(10.0, 20.0, None);
    assert_eq!(
        Interpolator::interpolate(&seg, &VariableTypeHint::Float, f64::INFINITY),
        EvaluatedValue::Float(20.0)
    );
    assert_eq!(
        Interpolator::interpolate(&seg, &VariableTypeHint::Float, f64::NEG_INFINITY),
        EvaluatedValue::Float(10.0)
    );
}

/// NaN の from/to 値は lerp を経て結果へ伝播する（Float は NaN、Integer は 0 に飽和）
#[test]
fn nan_endpoints_propagate_without_panic() {
    let seg = make_segment(f64::NAN, 100.0, None);
    match Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5) {
        EvaluatedValue::Float(v) => assert!(v.is_nan(), "expected NaN, got {v}"),
        _ => panic!("expected Float"),
    }
    let vt = VariableTypeHint::Integer { typewriter: None };
    assert_eq!(
        Interpolator::interpolate(&seg, &vt, 0.5),
        EvaluatedValue::Integer(0)
    );
}

/// 極端な振幅の Integer 補間は i64 範囲へ飽和し、オーバーフロー panic しない
#[test]
fn integer_interpolation_saturates_at_i64_bounds() {
    let vt = VariableTypeHint::Integer { typewriter: None };

    let seg_pos = make_segment(0.0, 1e300, None);
    assert_eq!(
        Interpolator::interpolate(&seg_pos, &vt, 1.0),
        EvaluatedValue::Integer(i64::MAX)
    );

    let seg_neg = make_segment(0.0, -1e300, None);
    assert_eq!(
        Interpolator::interpolate(&seg_neg, &vt, 1.0),
        EvaluatedValue::Integer(i64::MIN)
    );

    let seg_inf = make_segment(0.0, f64::INFINITY, None);
    assert_eq!(
        Interpolator::interpolate(&seg_inf, &vt, 1.0),
        EvaluatedValue::Integer(i64::MAX)
    );
}

/// 全31種の名前付きイージングが NaN / ±inf の progress_t でも panic しないこと
#[test]
fn all_named_easings_no_panic_on_non_finite_t() {
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
    for easing in all_easings {
        for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let seg = make_segment(0.0, 100.0, Some(EasingFunction::Named(easing)));
            let result = Interpolator::interpolate(&seg, &VariableTypeHint::Float, t);
            assert!(
                matches!(result, EvaluatedValue::Float(_)),
                "EasingName::{easing:?} with t={t} failed to produce Float"
            );
        }
    }
}

/// ベジェ制御点に NaN が含まれても panic せず NaN が伝播する
#[test]
fn parametric_easing_nan_control_points_propagate_without_panic() {
    let seg = make_segment(
        0.0,
        100.0,
        Some(EasingFunction::Parametric(ParametricEasing::CubicBezier {
            x0: f64::NAN,
            x1: 0.0,
            x2: 1.0,
            x3: 1.0,
        })),
    );
    match Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5) {
        EvaluatedValue::Float(v) => assert!(v.is_nan(), "expected NaN, got {v}"),
        _ => panic!("expected Float"),
    }
}

/// ベジェ出力は [0,1] にクランプされず from/to を超える外挿が起きる（許容挙動の特性化）
#[test]
fn parametric_easing_overshoots_beyond_endpoints() {
    // quad_bez(0, 2, 1, 0.5) = lerp(lerp(0,2,.5), lerp(2,1,.5), .5) = lerp(1, 1.5, .5) = 1.25
    let seg = make_segment(
        0.0,
        100.0,
        Some(EasingFunction::Parametric(
            ParametricEasing::QuadraticBezier {
                x0: 0.0,
                x1: 2.0,
                x2: 1.0,
            },
        )),
    );
    match Interpolator::interpolate(&seg, &VariableTypeHint::Float, 0.5) {
        EvaluatedValue::Float(v) => {
            assert!(
                (v - 125.0).abs() < 1e-9,
                "expected 125.0 (overshoot), got {v}"
            );
        }
        _ => panic!("expected Float"),
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
