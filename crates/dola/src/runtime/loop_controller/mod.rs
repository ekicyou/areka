//! ループ再生の周回判定・進行・タイムテーブル再利用のためのオフセット調整。
//!
//! 純粋関数群として実装。全状態は `StoryboardInstance` に保持される。

use interpolation::Ease;
use rand::Rng;
use rand::RngExt;

use crate::easing::{EasingFunction, EasingName, ParametricEasing};

use super::instance_manager::StoryboardInstance;

/// ループ処理の結果を示す判別 enum。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopAction {
    /// ループ継続（または周回内で変化なし）
    Continue,
    /// ループ完了 — Conclude すべき
    Conclude,
}

/// ループ継続の可否を判定する純粋関数。
///
/// `loop_count == -1` の場合は常に `true`（無限ループ）。
/// それ以外は `loops_completed < loop_count as u64` で判定。
pub(crate) fn should_continue_loop(instance: &StoryboardInstance) -> bool {
    if instance.loop_count == -1 {
        return true;
    }
    // 整数変換の安全性(D1a-V): loop_count は facade::compile_and_validate が
    // 「-1 または正値」のみ許可する（loop_count <= 0 かつ != -1 は InvalidLoopCount）。
    // -1 は上の早期 return で処理済みのため、ここでの `as u64` は常に正値の
    // 無損失変換であり、負値の wrap（例: -2 → 約 1.8e19 ≒ 実質無限ループ化）は
    // 発火しない。
    debug_assert!(
        instance.loop_count >= 1,
        "loop_count invariant violated: {}",
        instance.loop_count
    );
    instance.loops_completed < instance.loop_count as u64
}

/// イージング関数を `[0,1]` 入力に適用し `[0,1]` 出力を返す。
fn apply_easing(easing: &EasingFunction, t: f64) -> f64 {
    match easing {
        EasingFunction::Named(name) => match name {
            EasingName::Linear => t,
            EasingName::QuadraticIn => t.calc(interpolation::EaseFunction::QuadraticIn),
            EasingName::QuadraticOut => t.calc(interpolation::EaseFunction::QuadraticOut),
            EasingName::QuadraticInOut => t.calc(interpolation::EaseFunction::QuadraticInOut),
            EasingName::CubicIn => t.calc(interpolation::EaseFunction::CubicIn),
            EasingName::CubicOut => t.calc(interpolation::EaseFunction::CubicOut),
            EasingName::CubicInOut => t.calc(interpolation::EaseFunction::CubicInOut),
            EasingName::QuarticIn => t.calc(interpolation::EaseFunction::QuarticIn),
            EasingName::QuarticOut => t.calc(interpolation::EaseFunction::QuarticOut),
            EasingName::QuarticInOut => t.calc(interpolation::EaseFunction::QuarticInOut),
            EasingName::QuinticIn => t.calc(interpolation::EaseFunction::QuinticIn),
            EasingName::QuinticOut => t.calc(interpolation::EaseFunction::QuinticOut),
            EasingName::QuinticInOut => t.calc(interpolation::EaseFunction::QuinticInOut),
            EasingName::SineIn => t.calc(interpolation::EaseFunction::SineIn),
            EasingName::SineOut => t.calc(interpolation::EaseFunction::SineOut),
            EasingName::SineInOut => t.calc(interpolation::EaseFunction::SineInOut),
            EasingName::CircularIn => t.calc(interpolation::EaseFunction::CircularIn),
            EasingName::CircularOut => t.calc(interpolation::EaseFunction::CircularOut),
            EasingName::CircularInOut => t.calc(interpolation::EaseFunction::CircularInOut),
            EasingName::ExponentialIn => t.calc(interpolation::EaseFunction::ExponentialIn),
            EasingName::ExponentialOut => t.calc(interpolation::EaseFunction::ExponentialOut),
            EasingName::ExponentialInOut => t.calc(interpolation::EaseFunction::ExponentialInOut),
            EasingName::ElasticIn => t.calc(interpolation::EaseFunction::ElasticIn),
            EasingName::ElasticOut => t.calc(interpolation::EaseFunction::ElasticOut),
            EasingName::ElasticInOut => t.calc(interpolation::EaseFunction::ElasticInOut),
            EasingName::BackIn => t.calc(interpolation::EaseFunction::BackIn),
            EasingName::BackOut => t.calc(interpolation::EaseFunction::BackOut),
            EasingName::BackInOut => t.calc(interpolation::EaseFunction::BackInOut),
            EasingName::BounceIn => t.calc(interpolation::EaseFunction::BounceIn),
            EasingName::BounceOut => t.calc(interpolation::EaseFunction::BounceOut),
            EasingName::BounceInOut => t.calc(interpolation::EaseFunction::BounceInOut),
        },
        EasingFunction::Parametric(p) => match p {
            ParametricEasing::QuadraticBezier { x0, x1, x2 } => {
                interpolation::quad_bez(x0, x1, x2, &t)
            }
            ParametricEasing::CubicBezier { x0, x1, x2, x3 } => {
                interpolation::cub_bez(x0, x1, x2, x3, &t)
            }
        },
    }
}

/// ランダム遅延生成。
///
/// uniform `[0,1]` → easing → `[min, max]` mapping。
/// `min == max` の場合は early return で固定遅延を返す。
pub(crate) fn generate_delay(
    min: f64,
    max: f64,
    easing: &EasingFunction,
    rng: &mut impl Rng,
) -> f64 {
    if min == max {
        return min;
    }
    let t: f64 = rng.random_range(0.0..1.0);
    let eased = apply_easing(easing, t);
    min + eased * (max - min)
}

/// 周回進行: 1周回分のオフセット調整 + 遅延生成を実行する。
///
/// - `loops_completed += 1`
/// - `loop_start_time += loop_duration`
/// - `end_time += loop_duration`
/// - `loop_offset_min.is_some()` の場合: `end_time += generate_delay(...)`
/// - 全 `trigger_states` の `fired = false` にリセット
pub(crate) fn advance_loop(instance: &mut StoryboardInstance, rng: &mut impl Rng) {
    instance.loops_completed += 1;
    instance.loop_start_time += instance.loop_duration;
    instance.end_time += instance.loop_duration;

    // ループオフセット遅延の適用
    if let Some(min) = instance.loop_offset_min {
        let delay = generate_delay(
            min,
            instance.loop_offset_max,
            &instance.loop_offset_easing,
            rng,
        );
        instance.end_time += delay;
    }

    // トリガー状態リセット（周回ごとに再発火可能にする）
    for ts in &mut instance.trigger_states {
        ts.fired = false;
    }
}

/// 1つのインスタンスのループ処理を実行する。
///
/// `current_time >= end_time` の場合、while ループで全終了済み周回を処理し、
/// 各周回について `loops_completed` をインクリメントして継続可否を判定する。
/// 遅延が `end_time` に加算されるため、複数周回の一括スキップは自然に抑制される。
///
/// # Arguments
/// - `instance`: 対象インスタンスの可変参照
/// - `current_time`: 現在時刻
/// - `rng`: 乱数生成器
///
/// # Returns
/// - `LoopAction::Continue`: ループ継続またはループ対象外
/// - `LoopAction::Conclude`: ループ完了 — 呼び出し側で `conclude_internal()` を実行
pub(crate) fn process_loops(
    instance: &mut StoryboardInstance,
    current_time: f64,
    rng: &mut impl Rng,
) -> LoopAction {
    // 周回内であれば何もしない
    if current_time < instance.end_time {
        return LoopAction::Continue;
    }

    // loop_count=1 の場合はループ不要 → 即座に Conclude
    if instance.loop_count == 1 {
        return LoopAction::Conclude;
    }

    // while ループで全終了済み周回を処理
    // 時刻境界の注意(D1a-V): 反復回数は (current_time - end_time) / 周回長 に比例する。
    // 無限ループ（loop_count == -1）は MIN_LOOP_DURATION（0.1s）が周回長の下限のため
    // 停止性は保証されるが、巨大な時刻ジャンプ（壁時計補正等）では反復が膨大になり得る。
    // 反復上限キャップや剰余スキップの導入は外部観測可能な挙動
    // （loops_completed / loop_start_time / 乱数消費）を変えるため
    // report/proposals.md P9 に記録。
    while current_time >= instance.end_time {
        advance_loop(instance, rng);

        if !should_continue_loop(instance) {
            return LoopAction::Conclude;
        }
    }

    LoopAction::Continue
}

#[cfg(test)]
mod tests;
