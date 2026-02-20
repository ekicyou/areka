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
    while current_time >= instance.end_time {
        advance_loop(instance, rng);

        if !should_continue_loop(instance) {
            return LoopAction::Conclude;
        }
    }

    LoopAction::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::instance_state::InstanceState;
    use crate::storyboard::InterruptionPolicy;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    /// テスト用インスタンスヘルパー（loop_offset なし）
    fn make_instance(loop_count: i32, loop_duration: f64) -> StoryboardInstance {
        StoryboardInstance {
            group_id: 1,
            storyboard_name: "test".to_string(),
            state: InstanceState::Playing,
            interruption_policy: InterruptionPolicy::Conclude,
            start_time: 0.0,
            time_scale: 1.0,
            base_duration: loop_duration,
            pause_accumulated: 0.0,
            pause_start: None,
            loop_count,
            loops_completed: 0,
            finish_deadline: None,
            end_time: loop_duration,
            loop_start_time: 0.0,
            loop_duration,
            loop_offset_min: None,
            loop_offset_max: 0.0,
            loop_offset_easing: EasingFunction::Named(EasingName::Linear),
            trigger_states: Vec::new(),
        }
    }

    /// テスト用インスタンスヘルパー（loop_offset あり）
    fn make_instance_with_offset(
        loop_count: i32,
        loop_duration: f64,
        min: f64,
        max: f64,
        easing: EasingFunction,
    ) -> StoryboardInstance {
        let mut inst = make_instance(loop_count, loop_duration);
        inst.loop_offset_min = Some(min);
        inst.loop_offset_max = max;
        inst.loop_offset_easing = easing;
        inst
    }

    /// 決定的テスト用 RNG
    fn test_rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    /// テスト用ダミー RNG（loop_offset なしのテストで使用）
    fn dummy_rng() -> SmallRng {
        SmallRng::seed_from_u64(0)
    }

    // =======================================================================
    // should_continue_loop
    // =======================================================================

    #[test]
    fn should_continue_loop_basic() {
        let mut inst = make_instance(3, 1.0);
        // loops_completed=0, loop_count=3 → true
        assert!(should_continue_loop(&inst));

        inst.loops_completed = 2;
        assert!(should_continue_loop(&inst));

        inst.loops_completed = 3;
        assert!(!should_continue_loop(&inst));
    }

    #[test]
    fn should_continue_loop_infinite() {
        let mut inst = make_instance(-1, 1.0);
        assert!(should_continue_loop(&inst));

        inst.loops_completed = u64::MAX - 1;
        assert!(should_continue_loop(&inst));
    }

    #[test]
    fn should_continue_loop_single() {
        let mut inst = make_instance(1, 1.0);
        // loops_completed=0 → true (まだ1周目未完了)
        assert!(should_continue_loop(&inst));

        inst.loops_completed = 1;
        // loops_completed=1 >= loop_count=1 → false
        assert!(!should_continue_loop(&inst));
    }

    // =======================================================================
    // advance_loop
    // =======================================================================

    #[test]
    fn advance_loop_updates_fields() {
        let mut inst = make_instance(3, 2.0);
        let mut rng = dummy_rng();
        // 初期: loops_completed=0, loop_start_time=0.0, end_time=2.0
        assert_eq!(inst.loops_completed, 0);
        assert_eq!(inst.loop_start_time, 0.0);
        assert_eq!(inst.end_time, 2.0);

        advance_loop(&mut inst, &mut rng);

        assert_eq!(inst.loops_completed, 1);
        assert_eq!(inst.loop_start_time, 2.0);
        assert_eq!(inst.end_time, 4.0);

        advance_loop(&mut inst, &mut rng);

        assert_eq!(inst.loops_completed, 2);
        assert_eq!(inst.loop_start_time, 4.0);
        assert_eq!(inst.end_time, 6.0);
    }

    // =======================================================================
    // process_loops
    // =======================================================================

    #[test]
    fn process_loops_within_loop() {
        // current_time < end_time → Continue, フィールド変化なし
        let mut inst = make_instance(3, 2.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 1.0, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 0);
        assert_eq!(inst.loop_start_time, 0.0);
        assert_eq!(inst.end_time, 2.0);
    }

    #[test]
    fn process_loops_one_loop_completed_continue() {
        // loop_count=3, 1周終了 → Continue, loops_completed=1
        let mut inst = make_instance(3, 2.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 2.5, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 1);
        assert_eq!(inst.loop_start_time, 2.0);
        assert_eq!(inst.end_time, 4.0);
    }

    #[test]
    fn process_loops_all_loops_completed() {
        // loop_count=3, 3周終了 → Conclude
        let mut inst = make_instance(3, 2.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 6.5, &mut rng);
        assert_eq!(action, LoopAction::Conclude);
        assert_eq!(inst.loops_completed, 3);
    }

    #[test]
    fn process_loops_multi_loops_at_once() {
        // loop_count=5, 3周分一度に終了 → Continue, loops_completed=3
        let mut inst = make_instance(5, 1.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 3.5, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 3);
        assert_eq!(inst.loop_start_time, 3.0);
        assert_eq!(inst.end_time, 4.0);
    }

    #[test]
    fn process_loops_all_loops_completed_overshoot() {
        // loop_count=3, 5周分超過 → Conclude, loops_completed=3
        let mut inst = make_instance(3, 1.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 5.5, &mut rng);
        assert_eq!(action, LoopAction::Conclude);
        assert_eq!(inst.loops_completed, 3);
    }

    #[test]
    fn process_loops_infinite_multi_loops() {
        // loop_count=-1, 5周分 → Continue, loops_completed=5
        let mut inst = make_instance(-1, 1.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 5.5, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 5);
        assert_eq!(inst.loop_start_time, 5.0);
        assert_eq!(inst.end_time, 6.0);
    }

    #[test]
    fn process_loops_single_loop_conclude() {
        // loop_count=1, 終了 → Conclude（whileループに入らず即Conclude）
        let mut inst = make_instance(1, 2.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 2.5, &mut rng);
        assert_eq!(action, LoopAction::Conclude);
        // loop_count=1 の場合は advance_loop を呼ばない
        assert_eq!(inst.loops_completed, 0);
    }

    #[test]
    fn process_loops_exact_boundary() {
        // current_time == end_time → 周回処理が発生する
        let mut inst = make_instance(3, 2.0);
        let mut rng = dummy_rng();
        let action = process_loops(&mut inst, 2.0, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 1);
    }

    // =======================================================================
    // generate_delay (Task 3.5)
    // =======================================================================

    #[test]
    fn generate_delay_min_eq_max_returns_fixed() {
        let mut rng = test_rng();
        // min == max → 常に固定値を返す（rng 不使用）
        let d1 = generate_delay(
            3.0,
            3.0,
            &EasingFunction::Named(EasingName::Linear),
            &mut rng,
        );
        assert_eq!(d1, 3.0);
        let d2 = generate_delay(
            0.0,
            0.0,
            &EasingFunction::Named(EasingName::QuadraticIn),
            &mut rng,
        );
        assert_eq!(d2, 0.0);
    }

    #[test]
    fn generate_delay_within_range() {
        let mut rng = test_rng();
        let easing = EasingFunction::Named(EasingName::Linear);
        for _ in 0..100 {
            let d = generate_delay(1.0, 5.0, &easing, &mut rng);
            assert!(d >= 1.0, "delay {d} < min 1.0");
            assert!(d <= 5.0, "delay {d} > max 5.0");
        }
    }

    #[test]
    fn generate_delay_linear_distribution() {
        let mut rng = SmallRng::seed_from_u64(123);
        let easing = EasingFunction::Named(EasingName::Linear);
        let mut sum = 0.0;
        let n = 10000;
        for _ in 0..n {
            sum += generate_delay(0.0, 10.0, &easing, &mut rng);
        }
        let mean = sum / n as f64;
        // Linear distribution: mean ≈ 5.0 (±0.5 tolerance)
        assert!((mean - 5.0).abs() < 0.5, "mean {mean} too far from 5.0");
    }

    #[test]
    fn generate_delay_quadratic_in_skews_low() {
        let mut rng = SmallRng::seed_from_u64(456);
        let lin = EasingFunction::Named(EasingName::Linear);
        let qin = EasingFunction::Named(EasingName::QuadraticIn);
        let n = 5000;
        let mut sum_lin = 0.0;
        let mut sum_qin = 0.0;
        let mut rng2 = SmallRng::seed_from_u64(456);
        for _ in 0..n {
            sum_lin += generate_delay(0.0, 10.0, &lin, &mut rng);
            sum_qin += generate_delay(0.0, 10.0, &qin, &mut rng2);
        }
        let mean_lin = sum_lin / n as f64;
        let mean_qin = sum_qin / n as f64;
        // QuadraticIn skews low → mean should be lower than Linear
        assert!(
            mean_qin < mean_lin,
            "QuadraticIn mean {mean_qin} should < Linear mean {mean_lin}"
        );
    }

    #[test]
    fn generate_delay_quadratic_out_skews_high() {
        let mut rng = SmallRng::seed_from_u64(789);
        let lin = EasingFunction::Named(EasingName::Linear);
        let qout = EasingFunction::Named(EasingName::QuadraticOut);
        let n = 5000;
        let mut sum_lin = 0.0;
        let mut sum_qout = 0.0;
        let mut rng2 = SmallRng::seed_from_u64(789);
        for _ in 0..n {
            sum_lin += generate_delay(0.0, 10.0, &lin, &mut rng);
            sum_qout += generate_delay(0.0, 10.0, &qout, &mut rng2);
        }
        let mean_lin = sum_lin / n as f64;
        let mean_qout = sum_qout / n as f64;
        // QuadraticOut skews high → mean should be higher than Linear
        assert!(
            mean_qout > mean_lin,
            "QuadraticOut mean {mean_qout} should > Linear mean {mean_lin}"
        );
    }

    #[test]
    fn generate_delay_deterministic_with_same_seed() {
        let easing = EasingFunction::Named(EasingName::SineOut);
        let mut rng1 = SmallRng::seed_from_u64(42);
        let mut rng2 = SmallRng::seed_from_u64(42);
        for _ in 0..20 {
            let d1 = generate_delay(1.0, 8.0, &easing, &mut rng1);
            let d2 = generate_delay(1.0, 8.0, &easing, &mut rng2);
            assert_eq!(d1, d2);
        }
    }

    #[test]
    fn apply_easing_linear_identity() {
        let easing = EasingFunction::Named(EasingName::Linear);
        assert_eq!(apply_easing(&easing, 0.0), 0.0);
        assert_eq!(apply_easing(&easing, 0.5), 0.5);
        assert_eq!(apply_easing(&easing, 1.0), 1.0);
    }

    #[test]
    fn apply_easing_parametric_cubic_bezier() {
        let easing = EasingFunction::Parametric(crate::easing::ParametricEasing::CubicBezier {
            x0: 0.0,
            x1: 0.5,
            x2: 0.5,
            x3: 1.0,
        });
        let result = apply_easing(&easing, 0.5);
        assert!(
            result >= 0.0 && result <= 1.0,
            "result {result} out of range"
        );
    }

    // =======================================================================
    // advance_loop with delay (Task 3.6)
    // =======================================================================

    #[test]
    fn advance_loop_with_offset_adds_delay() {
        let mut inst =
            make_instance_with_offset(-1, 2.0, 1.0, 5.0, EasingFunction::Named(EasingName::Linear));
        let mut rng = test_rng();
        let end_before = inst.end_time; // 2.0

        advance_loop(&mut inst, &mut rng);

        // end_time should be > 2.0 + 2.0 (loop_duration) due to delay
        assert!(
            inst.end_time > end_before + inst.loop_duration,
            "end_time {} should be > {} (loop_duration + old end_time)",
            inst.end_time,
            end_before + inst.loop_duration
        );
        // delay should be in [1.0, 5.0] → end_time in [5.0, 9.0]
        let delay = inst.end_time - (end_before + inst.loop_duration);
        assert!(
            delay >= 1.0 && delay <= 5.0,
            "delay {delay} out of [1.0, 5.0]"
        );
    }

    #[test]
    fn advance_loop_without_offset_no_delay() {
        let mut inst = make_instance(-1, 2.0);
        let mut rng = test_rng();
        advance_loop(&mut inst, &mut rng);
        // No offset → end_time = 2.0 + 2.0 = 4.0 exactly
        assert_eq!(inst.end_time, 4.0);
    }

    #[test]
    fn process_loops_with_delay_prevents_multi_skip() {
        // With loop_offset, delay prevents multiple loops from completing at once
        let mut inst = make_instance_with_offset(
            -1,
            1.0,
            3.0,
            3.0, // fixed 3.0s delay
            EasingFunction::Named(EasingName::Linear),
        );
        let mut rng = test_rng();

        // current_time=1.5 >= end_time=1.0 → advance once
        // After advance: end_time = 1.0 + 1.0 + 3.0 = 5.0
        // current_time=1.5 < 5.0 → stop
        let action = process_loops(&mut inst, 1.5, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 1);
        assert_eq!(inst.end_time, 5.0); // 1.0 + 1.0(loop) + 3.0(delay)
    }

    #[test]
    fn process_loops_loop_count_1_ignores_offset() {
        // loop_count=1 → Conclude immediately, offset is never applied
        let mut inst =
            make_instance_with_offset(1, 2.0, 5.0, 5.0, EasingFunction::Named(EasingName::Linear));
        let mut rng = test_rng();
        let action = process_loops(&mut inst, 2.5, &mut rng);
        assert_eq!(action, LoopAction::Conclude);
        assert_eq!(inst.loops_completed, 0); // Never advanced
    }

    #[test]
    fn process_loops_infinite_with_offset_applies_each_loop() {
        let mut inst = make_instance_with_offset(
            -1,
            1.0,
            2.0,
            2.0, // fixed 2.0s delay
            EasingFunction::Named(EasingName::Linear),
        );
        let mut rng = test_rng();

        // Loop 1: end_time=1.0; current_time=4.0 → advance → end_time=1.0+1.0+2.0=4.0
        // current_time=4.0 >= 4.0 → advance loop 2 → end_time=4.0+1.0+2.0=7.0
        // current_time=4.0 < 7.0 → stop
        let action = process_loops(&mut inst, 4.0, &mut rng);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 2);
        assert_eq!(inst.end_time, 7.0);
    }
}
