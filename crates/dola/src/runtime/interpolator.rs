//! イージング関数の適用と補間計算。

use std::collections::HashMap;
use std::rc::Rc;

use interpolation::{Ease, EaseFunction};

use crate::compile::{CompiledSegment, VariableTypeHint};
use crate::easing::{EasingFunction, EasingName, ParametricEasing};
use crate::transition::TransitionValue;
use crate::value::DynamicValue;

use super::EvaluatedValue;

/// Object 値の intern pool（同一内容の DynamicValue に同一 Rc を返す）。
///
/// compile 時または evaluate 時に使用し、Object 型変数の
/// 差分比較を `Rc::ptr_eq()` による O(1) で実行可能にする。
pub(crate) struct ObjectInternPool {
    pool: HashMap<DynamicValue, Rc<DynamicValue>>,
}

impl ObjectInternPool {
    pub fn new() -> Self {
        Self {
            pool: HashMap::new(),
        }
    }

    /// 同一内容の DynamicValue に対して同一の Rc を返す。
    pub fn intern(&mut self, value: DynamicValue) -> Rc<DynamicValue> {
        self.pool
            .entry(value.clone())
            .or_insert_with(|| Rc::new(value))
            .clone()
    }
}

/// イージング適用と値の補間計算を行う。
pub struct Interpolator;

impl Interpolator {
    /// セグメントの進捗率 `progress_t` で補間値を計算する。
    ///
    /// - `progress_t` は 0.0..=1.0 にクランプされる。
    /// - `VariableTypeHint::Float`: f64 直接値
    /// - `VariableTypeHint::Integer`: f64 補間 → `round()` → i64
    /// - `VariableTypeHint::Object`: `progress_t >= 1.0` なら `to_value`、それ以外は `from_value`
    ///
    /// `intern_pool` が Some の場合、Object 値を intern して同一内容で同一 Rc を共有する。
    pub fn interpolate(
        segment: &CompiledSegment,
        variable_type: &VariableTypeHint,
        progress_t: f64,
    ) -> EvaluatedValue {
        Self::interpolate_with_pool(segment, variable_type, progress_t, None)
    }

    /// intern pool 付き補間。TimelineManager から呼ばれる。
    pub(crate) fn interpolate_with_pool(
        segment: &CompiledSegment,
        variable_type: &VariableTypeHint,
        progress_t: f64,
        intern_pool: Option<&mut ObjectInternPool>,
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
                let dv = transition_value_to_dynamic(value);
                let rc = match intern_pool {
                    Some(pool) => pool.intern(dv),
                    None => Rc::new(dv),
                };
                EvaluatedValue::Object(rc)
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
#[path = "interpolator_tests.rs"]
mod tests;
