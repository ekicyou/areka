use super::*;
use crate::easing::EasingName;
use crate::runtime::instance_state::InstanceState;
use crate::storyboard::InterruptionPolicy;
use rand::SeedableRng;
use rand::rngs::SmallRng;

/// テスト用インスタンスヘルパー（loop_offset なし）
fn make_instance(loop_count: i32, loop_duration: f64) -> StoryboardInstance {
    StoryboardInstance {
        storyboard_name: "test".to_string(),
        state: InstanceState::Playing,
        interruption_policy: InterruptionPolicy::Conclude,
        time_scale: 1.0,
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
    assert!(should_continue_loop(&inst));

    inst.loops_completed = 1;
    assert!(!should_continue_loop(&inst));
}

// =======================================================================
// advance_loop
// =======================================================================

#[test]
fn advance_loop_updates_fields() {
    let mut inst = make_instance(3, 2.0);
    let mut rng = dummy_rng();
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
    // 周回内ならフィールドは変わらない
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
    let mut inst = make_instance(3, 2.0);
    let mut rng = dummy_rng();
    let action = process_loops(&mut inst, 6.5, &mut rng);
    assert_eq!(action, LoopAction::Conclude);
    assert_eq!(inst.loops_completed, 3);
}

#[test]
fn process_loops_multi_loops_at_once() {
    // 3周分が一度に終わる
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
    // 5周分超過しても loops_completed は loop_count まで
    let mut inst = make_instance(3, 1.0);
    let mut rng = dummy_rng();
    let action = process_loops(&mut inst, 5.5, &mut rng);
    assert_eq!(action, LoopAction::Conclude);
    assert_eq!(inst.loops_completed, 3);
}

#[test]
fn process_loops_infinite_multi_loops() {
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
    // while ループに入らず即 Conclude
    let mut inst = make_instance(1, 2.0);
    let mut rng = dummy_rng();
    let action = process_loops(&mut inst, 2.5, &mut rng);
    assert_eq!(action, LoopAction::Conclude);
    // loop_count=1 の場合は advance_loop を呼ばない
    assert_eq!(inst.loops_completed, 0);
}

#[test]
fn process_loops_exact_boundary() {
    // current_time == end_time でも周回処理が起きる
    let mut inst = make_instance(3, 2.0);
    let mut rng = dummy_rng();
    let action = process_loops(&mut inst, 2.0, &mut rng);
    assert_eq!(action, LoopAction::Continue);
    assert_eq!(inst.loops_completed, 1);
}

#[test]
fn process_loops_zero_duration_concludes_after_one_advance() {
    // 長さ 0・待ちなしの周回は時刻が進まない → 残りの周回もこの瞬間に終わる（loop_count 回まわさない）
    let mut inst = make_instance(1_000_000, 0.0);
    let mut rng = dummy_rng();
    let action = process_loops(&mut inst, 0.0, &mut rng);
    assert_eq!(action, LoopAction::Conclude);
    assert_eq!(inst.loops_completed, 1);
}

#[test]
fn process_loops_zero_duration_with_zero_delay_draw_continues() {
    // 長さ 0 でも乱数の待ちがあれば、待ちがたまたま 0 を引いた周回で残りを切らない
    let mut inst =
        make_instance_with_offset(5, 0.0, 0.0, 1.0, EasingFunction::Named(EasingName::BackIn));
    let mut rng = test_rng();
    let action = process_loops(&mut inst, 100.0, &mut rng);
    assert_eq!(action, LoopAction::Conclude);
    assert_eq!(inst.loops_completed, 5);
}

#[test]
fn process_loops_huge_end_time_concludes() {
    // 足しても丸めで消える end_time では、無限のループも回り続けず終わる
    let mut inst = make_instance(-1, 0.1);
    inst.end_time = 1e17;
    let mut rng = dummy_rng();
    assert_eq!(
        process_loops(&mut inst, 1e17, &mut rng),
        LoopAction::Conclude
    );
    assert_eq!(inst.loops_completed, 1);
}

// =======================================================================
// generate_delay
// =======================================================================

#[test]
fn generate_delay_min_eq_max_returns_fixed() {
    let mut rng = test_rng();
    // min == max は rng を使わず固定値を返す
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

// =======================================================================
// advance_loop with delay
// =======================================================================

#[test]
fn advance_loop_with_offset_adds_delay() {
    let mut inst =
        make_instance_with_offset(-1, 2.0, 1.0, 5.0, EasingFunction::Named(EasingName::Linear));
    let mut rng = test_rng();
    let end_before = inst.end_time;

    advance_loop(&mut inst, &mut rng);

    assert!(
        inst.end_time > end_before + inst.loop_duration,
        "end_time {} should be > {} (loop_duration + old end_time)",
        inst.end_time,
        end_before + inst.loop_duration
    );
    let delay = inst.end_time - (end_before + inst.loop_duration);
    assert!(
        (1.0..=5.0).contains(&delay),
        "delay {delay} out of [1.0, 5.0]"
    );
}

#[test]
fn advance_loop_without_offset_no_delay() {
    let mut inst = make_instance(-1, 2.0);
    let mut rng = test_rng();
    advance_loop(&mut inst, &mut rng);
    assert_eq!(inst.end_time, 4.0);
}

#[test]
fn process_loops_with_delay_prevents_multi_skip() {
    // delay があると複数周回が一度に終わらない
    let mut inst = make_instance_with_offset(
        -1,
        1.0,
        3.0,
        3.0, // fixed 3.0s delay
        EasingFunction::Named(EasingName::Linear),
    );
    let mut rng = test_rng();

    // 1 回進めると end_time = 1.0 + 1.0 + 3.0 = 5.0 で current_time を超える
    let action = process_loops(&mut inst, 1.5, &mut rng);
    assert_eq!(action, LoopAction::Continue);
    assert_eq!(inst.loops_completed, 1);
    assert_eq!(inst.end_time, 5.0); // 1.0 + 1.0(loop) + 3.0(delay)
}

#[test]
fn process_loops_loop_count_1_ignores_offset() {
    // loop_count=1 は即 Conclude で、offset は適用されない
    let mut inst =
        make_instance_with_offset(1, 2.0, 5.0, 5.0, EasingFunction::Named(EasingName::Linear));
    let mut rng = test_rng();
    let action = process_loops(&mut inst, 2.5, &mut rng);
    assert_eq!(action, LoopAction::Conclude);
    assert_eq!(inst.loops_completed, 0);
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

    // 1 周目: end_time 1.0 → 4.0、current_time=4.0 >= 4.0 なので 2 周目: → 7.0
    let action = process_loops(&mut inst, 4.0, &mut rng);
    assert_eq!(action, LoopAction::Continue);
    assert_eq!(inst.loops_completed, 2);
    assert_eq!(inst.end_time, 7.0);
}

/// [0,1] の外へ出るイージングでも待ちは [min, max] に収まる（負の待ちで tick が戻らない）
#[test]
fn generate_delay_clamps_overshooting_easing_into_range() {
    let wild_bezier = EasingFunction::Parametric(crate::easing::ParametricEasing::CubicBezier {
        x0: -5.0,
        x1: -5.0,
        x2: -5.0,
        x3: -5.0,
    });
    let back_in = EasingFunction::Named(EasingName::BackIn);
    for easing in [wild_bezier, back_in] {
        for seed in 0..64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let d = generate_delay(0.5, 1.5, &easing, &mut rng);
            assert!(
                (0.5..=1.5).contains(&d),
                "{easing:?} seed {seed}: delay {d}"
            );
        }
    }
}
