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
    pub fn interpolate(
        segment: &CompiledSegment,
        variable_type: &VariableTypeHint,
        progress_t: f64,
    ) -> EvaluatedValue {
        Self::interpolate_with_pool(segment, variable_type, progress_t, None)
    }

    /// intern pool 付き補間。TimelineManager から呼ばれる。
    ///
    /// `intern_pool` が Some の場合、Object 値を intern して同一内容で同一 Rc を共有する。
    pub(crate) fn interpolate_with_pool(
        segment: &CompiledSegment,
        variable_type: &VariableTypeHint,
        progress_t: f64,
        intern_pool: Option<&mut ObjectInternPool>,
    ) -> EvaluatedValue {
        // NOTE(数値境界): f64::clamp は min <= max（定数 0.0 <= 1.0）のため panic せず、
        // progress_t = NaN の場合は NaN をそのまま返す（クランプされない）。
        // NaN の伝播先: Object は `t >= 1.0` が false → from_value 側、
        // Float は NaN が結果へ伝播、Integer は `NaN.round() as i64` の飽和キャストで 0。
        // +inf / -inf は 1.0 / 0.0 にクランプされる。いずれの経路も panic しない
        // （tests.rs の D1b-V 数値境界テストで固定）。
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
                // NOTE(数値境界): `as i64` は飽和キャスト（Rust 1.45+）であり panic しない:
                // NaN → 0、i64::MAX 超 / +inf → i64::MAX、i64::MIN 未満 / -inf → i64::MIN。
                // round() も全 f64 入力で panic しない。
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
///
/// NOTE(数値境界): interpolation クレートの各 ease 関数は入力を内部で [0,1] に
/// クランプする（NaN は比較が false となりそのまま素通り）。実装は多項式・
/// sqrt・sin・powf・定数除算のみで、入力依存の除算を含まず全 f64 入力で
/// panic しない。NaN 入力は NaN として伝播する。
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
///
/// NOTE(数値境界): quad_bez / cub_bez は lerp（乗算・加減算のみ）の合成で
/// 除算を含まず、全 f64 入力で panic しない。制御点 x0..x3 は指示書由来の
/// 任意 f64 であり、NaN/inf は結果へそのまま伝播する（指示書数値の有限性
/// 検証は未実装: proposals.md P14 参照）。出力は [0,1] にクランプされない
/// ため、制御点次第で from/to を超える外挿（オーバーシュート）が起きる。
/// これは Back/Elastic 系の overshoot と同様に許容された挙動。
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
            // NOTE(防御的フォールバック): f64/i64 変数の from/to はバリデーション
            // （V13）で数値に制限されるため、非数値 Dynamic はコンパイル済み
            // データが不変条件を満たす限り到達しない。到達時も panic せず 0.0 に
            // 縮退する（tests.rs::float_interpolation_non_numeric_dynamic_falls_back_to_zero
            // で特性化済み）。
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
mod tests;
