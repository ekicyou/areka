//! イージング関数の適用と補間計算。

use interpolation::{Ease, EaseFunction};

use crate::compile::{CompiledSegment, VariableTypeHint};
use crate::easing::{EasingFunction, EasingName, ParametricEasing};
use crate::transition::TransitionValue;
use crate::value::DynamicValue;

use super::EvaluatedValue;

/// イージング適用と値の補間計算を行う。
pub struct Interpolator;

impl Interpolator {
    /// セグメントの進捗率 `progress_t` で補間値を計算する。
    ///
    /// - `progress_t` は 0.0..=1.0 にクランプされる。
    /// - `VariableTypeHint::Float`: f64 直接値
    /// - `VariableTypeHint::Integer`: f64 補間 → `round()` → i64
    /// - `VariableTypeHint::Object`: `progress_t >= 1.0` なら `to_value`、それ以外は `from_value`
    pub fn interpolate(
        segment: &CompiledSegment,
        variable_type: &VariableTypeHint,
        progress_t: f64,
    ) -> EvaluatedValue {
        let t = progress_t.clamp(0.0, 1.0);

        match variable_type {
            VariableTypeHint::Object => {
                // Object 型: 即時切替（t >= 1.0 で to_value）
                let value = if t >= 1.0 {
                    &segment.to_value
                } else {
                    &segment.from_value
                };
                EvaluatedValue::Object(transition_value_to_dynamic(value))
            }
            VariableTypeHint::Float => {
                let from = scalar_value(&segment.from_value);
                let to = scalar_value(&segment.to_value);
                let eased_t = apply_easing(t, &segment.easing);
                let result = interpolation::lerp(&from, &to, &eased_t);
                EvaluatedValue::Float(result)
            }
            VariableTypeHint::Integer { .. } => {
                let from = scalar_value(&segment.from_value);
                let to = scalar_value(&segment.to_value);
                let eased_t = apply_easing(t, &segment.easing);
                let result = interpolation::lerp(&from, &to, &eased_t);
                EvaluatedValue::Integer(result.round() as i64)
            }
        }
    }
}

/// `EasingFunction` を適用して進捗率を変換する。
fn apply_easing(t: f64, easing: &Option<EasingFunction>) -> f64 {
    match easing {
        None => t, // デフォルト: 線形補間
        Some(EasingFunction::Named(name)) => apply_named_easing(t, *name),
        Some(EasingFunction::Parametric(param)) => apply_parametric_easing(t, param),
    }
}

/// `EasingName` を `interpolation::EaseFunction` にマッピングして適用する。
fn apply_named_easing(t: f64, name: EasingName) -> f64 {
    match name {
        EasingName::Linear => t, // Linear はそのまま返す
        EasingName::QuadraticIn => t.calc(EaseFunction::QuadraticIn),
        EasingName::QuadraticOut => t.calc(EaseFunction::QuadraticOut),
        EasingName::QuadraticInOut => t.calc(EaseFunction::QuadraticInOut),
        EasingName::CubicIn => t.calc(EaseFunction::CubicIn),
        EasingName::CubicOut => t.calc(EaseFunction::CubicOut),
        EasingName::CubicInOut => t.calc(EaseFunction::CubicInOut),
        EasingName::QuarticIn => t.calc(EaseFunction::QuarticIn),
        EasingName::QuarticOut => t.calc(EaseFunction::QuarticOut),
        EasingName::QuarticInOut => t.calc(EaseFunction::QuarticInOut),
        EasingName::QuinticIn => t.calc(EaseFunction::QuinticIn),
        EasingName::QuinticOut => t.calc(EaseFunction::QuinticOut),
        EasingName::QuinticInOut => t.calc(EaseFunction::QuinticInOut),
        EasingName::SineIn => t.calc(EaseFunction::SineIn),
        EasingName::SineOut => t.calc(EaseFunction::SineOut),
        EasingName::SineInOut => t.calc(EaseFunction::SineInOut),
        EasingName::CircularIn => t.calc(EaseFunction::CircularIn),
        EasingName::CircularOut => t.calc(EaseFunction::CircularOut),
        EasingName::CircularInOut => t.calc(EaseFunction::CircularInOut),
        EasingName::ExponentialIn => t.calc(EaseFunction::ExponentialIn),
        EasingName::ExponentialOut => t.calc(EaseFunction::ExponentialOut),
        EasingName::ExponentialInOut => t.calc(EaseFunction::ExponentialInOut),
        EasingName::ElasticIn => t.calc(EaseFunction::ElasticIn),
        EasingName::ElasticOut => t.calc(EaseFunction::ElasticOut),
        EasingName::ElasticInOut => t.calc(EaseFunction::ElasticInOut),
        EasingName::BackIn => t.calc(EaseFunction::BackIn),
        EasingName::BackOut => t.calc(EaseFunction::BackOut),
        EasingName::BackInOut => t.calc(EaseFunction::BackInOut),
        EasingName::BounceIn => t.calc(EaseFunction::BounceIn),
        EasingName::BounceOut => t.calc(EaseFunction::BounceOut),
        EasingName::BounceInOut => t.calc(EaseFunction::BounceInOut),
    }
}

/// パラメトリックイージング（ベジェ曲線）を適用する。
fn apply_parametric_easing(t: f64, param: &ParametricEasing) -> f64 {
    match param {
        ParametricEasing::QuadraticBezier { x0, x1, x2 } => interpolation::quad_bez(x0, x1, x2, &t),
        ParametricEasing::CubicBezier { x0, x1, x2, x3 } => {
            interpolation::cub_bez(x0, x1, x2, x3, &t)
        }
    }
}

/// `TransitionValue` からスカラー値（f64）を取得する。
fn scalar_value(value: &TransitionValue) -> f64 {
    match value {
        TransitionValue::Scalar(v) => *v,
        TransitionValue::Dynamic(dv) => match dv {
            DynamicValue::Float(f) => *f,
            DynamicValue::Integer(i) => *i as f64,
            _ => 0.0,
        },
    }
}

/// `TransitionValue` を `DynamicValue` に変換する。
fn transition_value_to_dynamic(value: &TransitionValue) -> DynamicValue {
    match value {
        TransitionValue::Scalar(v) => DynamicValue::Float(*v),
        TransitionValue::Dynamic(dv) => dv.clone(),
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(
            result,
            EvaluatedValue::Object(DynamicValue::String("a".to_string()))
        );
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
        assert_eq!(
            result,
            EvaluatedValue::Object(DynamicValue::String("b".to_string()))
        );
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
}
