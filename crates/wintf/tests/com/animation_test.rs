//! com/animation.rs — Windows Animation Manager ラッパーのヘッドレステスト
//!
//! UIAnimation は CPU のみで動作する COM コンポーネント（CoCreateInstance）
//! であり GPU デバイス不要。CoInitializeEx パターンは既存テストに従う。
//! 本モジュールは従来 examples/dcomp_demo.rs でのみ使用されテスト空白だった。

use wintf::com::animation::*;

use super::common::with_com_initialized;

// ============================================================
// IUIAnimationTimer
// ============================================================

#[test]
fn animation_timer_returns_non_negative_monotonic_time() {
    with_com_initialized(|| {
        let timer = create_animation_timer().expect("create_animation_timer");
        let t1 = timer.get_time().expect("get_time #1");
        let t2 = timer.get_time().expect("get_time #2");
        assert!(t1 >= 0.0, "time should be non-negative: {t1}");
        assert!(t2 >= t1, "time should be monotonic: {t1} → {t2}");
    });
}

// ============================================================
// IUIAnimationManager2
// ============================================================

#[test]
fn animation_variable_holds_initial_value() {
    with_com_initialized(|| {
        let manager = create_animation_manager().expect("create_animation_manager");
        let var = manager
            .create_animation_variable(42.5)
            .expect("create_animation_variable");
        assert_eq!(var.get_value().expect("get_value"), 42.5);
    });
}

#[test]
fn manager_update_without_storyboards_succeeds() {
    with_com_initialized(|| {
        let manager = create_animation_manager().expect("manager");
        manager.update(0.0).expect("update(0.0)");
        manager.update(1.0).expect("update(1.0)");
    });
}

#[test]
fn manager_creates_storyboard() {
    with_com_initialized(|| {
        let manager = create_animation_manager().expect("manager");
        manager.create_storyboard().expect("create_storyboard");
    });
}

// ============================================================
// IUIAnimationTransitionLibrary2
// ============================================================

#[test]
fn transition_library_creates_transitions() {
    with_com_initialized(|| {
        let library =
            create_animation_transition_library().expect("create_animation_transition_library");
        library
            .create_accelerate_decelerate_transition(0.5, 1.0, 0.3, 0.3)
            .expect("accelerate_decelerate");
        library
            .create_cubic_transition(0.5, 1.0, 0.0)
            .expect("cubic");
    });
}

// ============================================================
// ストーリーボード組み立て + スケジュール + 値の最終到達
// ============================================================

/// 変数 0.0 → 1.0 の遷移をスケジュールし、終了時刻を大きく過ぎた
/// Update 後に最終値へ到達することを検証する（決定的挙動）。
#[test]
fn scheduled_storyboard_drives_variable_to_final_value() {
    with_com_initialized(|| {
        let manager = create_animation_manager().expect("manager");
        let library = create_animation_transition_library().expect("library");
        let timer = create_animation_timer().expect("timer");

        let var = manager.create_animation_variable(0.0).expect("variable");
        let storyboard = manager.create_storyboard().expect("storyboard");
        let transition = library
            .create_accelerate_decelerate_transition(0.25, 1.0, 0.3, 0.3)
            .expect("transition");

        storyboard
            .add_transition(&var, &transition)
            .expect("add_transition");

        let t0 = timer.get_time().expect("get_time");
        storyboard.schedule(t0).expect("schedule");

        // 遷移時間 0.25 秒を大きく超えた時刻まで進める → 最終値で安定
        manager.update(t0 + 10.0).expect("update");
        let value = var.get_value().expect("get_value");
        assert!(
            (value - 1.0).abs() < 1e-6,
            "value should reach final 1.0, got {value}"
        );
    });
}

/// キーフレーム API（add_keyframe_after_transition /
/// add_transition_at_keyframe）の組み立てが成功することを検証する。
#[test]
fn keyframe_based_storyboard_assembly_succeeds() {
    with_com_initialized(|| {
        let manager = create_animation_manager().expect("manager");
        let library = create_animation_transition_library().expect("library");
        let timer = create_animation_timer().expect("timer");

        let var_a = manager.create_animation_variable(0.0).expect("var_a");
        let var_b = manager.create_animation_variable(0.0).expect("var_b");
        let storyboard = manager.create_storyboard().expect("storyboard");

        let first = library
            .create_cubic_transition(0.1, 1.0, 0.0)
            .expect("first transition");
        storyboard
            .add_transition(&var_a, &first)
            .expect("add_transition");

        // first 終了点にキーフレームを置き、そこから var_b の遷移を開始
        let keyframe = storyboard
            .add_keyframe_after_transition(&first)
            .expect("add_keyframe_after_transition");
        let second = library
            .create_cubic_transition(0.1, 2.0, 0.0)
            .expect("second transition");
        storyboard
            .add_transition_at_keyframe(&var_b, &second, keyframe)
            .expect("add_transition_at_keyframe");

        let t0 = timer.get_time().expect("get_time");
        storyboard.schedule(t0).expect("schedule");
        manager.update(t0 + 10.0).expect("update");

        assert!((var_a.get_value().expect("a") - 1.0).abs() < 1e-6);
        assert!((var_b.get_value().expect("b") - 2.0).abs() < 1e-6);
    });
}
