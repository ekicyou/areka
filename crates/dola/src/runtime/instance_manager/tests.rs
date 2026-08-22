use super::*;
use crate::easing::EasingName;

fn create_test_instance(mgr: &mut InstanceManager, group_id: u64) {
    mgr.create_instance(
        group_id,
        "test_sb",
        InterruptionPolicy::Conclude,
        0.0,  // start_time
        1.0,  // time_scale
        2.0,  // base_duration
        1,    // loop_count
        2.0,  // end_time
        0.0,  // loop_start_time
        2.0,  // loop_duration
        None, // loop_offset_min
        0.0,  // loop_offset_max
        EasingFunction::Named(EasingName::Linear),
        0, // trigger_count
    );
}

#[test]
fn create_instance_initial_state() {
    let mut mgr = InstanceManager::new();
    let inst = mgr.create_instance(
        1,
        "fade",
        InterruptionPolicy::Conclude,
        10.0,
        1.0,
        5.0,
        1,
        15.0,
        10.0, // loop_start_time
        5.0,  // loop_duration
        None, // loop_offset_min
        0.0,  // loop_offset_max
        EasingFunction::Named(EasingName::Linear),
        0, // trigger_count
    );
    assert_eq!(inst.group_id, 1);
    assert_eq!(inst.storyboard_name, "fade");
    assert_eq!(inst.state, InstanceState::Created);
    assert_eq!(inst.start_time, 10.0);
    assert_eq!(inst.base_duration, 5.0);
    assert_eq!(inst.pause_accumulated, 0.0);
    assert!(inst.pause_start.is_none());
    assert!(inst.finish_deadline.is_none());
}

#[test]
fn transition_created_to_playing() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    assert!(mgr.transition(1, InstanceState::Playing).is_ok());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

#[test]
fn transition_playing_to_paused() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    assert!(mgr.transition(1, InstanceState::Paused).is_ok());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
}

#[test]
fn transition_paused_to_playing() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(1, InstanceState::Paused).unwrap();
    assert!(mgr.transition(1, InstanceState::Playing).is_ok());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

#[test]
fn transition_to_concluded_removes_instance() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(1, InstanceState::Concluded).unwrap();
    // Concluded 遷移後はインスタンス削除
    assert!(mgr.get(1).is_err());
}

#[test]
fn invalid_transition_rejected() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    // Created → Paused は不正
    assert!(mgr.transition(1, InstanceState::Paused).is_err());
}

#[test]
fn nonexistent_group_id_error() {
    let mgr = InstanceManager::new();
    assert!(mgr.get(999).is_err());
}

#[test]
fn pause_and_resume() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();

    // Pause at t=1.0
    mgr.pause(1).unwrap();
    mgr.set_pause_start(1, 1.0).unwrap();
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
    assert_eq!(mgr.get(1).unwrap().pause_start, Some(1.0));

    // Resume at t=3.0 (2秒間 pause)
    let new_end = mgr.resume(1, 3.0).unwrap();
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
    assert_eq!(mgr.get(1).unwrap().pause_accumulated, 2.0);
    assert!(mgr.get(1).unwrap().pause_start.is_none());
    // end_time = 2.0 + 2.0(pause) = 4.0
    assert_eq!(new_end, 4.0);
}

#[test]
fn finish_deadline_check() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, 5.0).unwrap();

    // t=4.0: まだ deadline に達していない
    assert!(mgr.check_finish_deadlines(4.0).is_empty());

    // t=5.0: deadline に達した
    let expired = mgr.check_finish_deadlines(5.0);
    assert_eq!(expired, vec![1]);
}

#[test]
fn finish_deadline_on_terminal_state_rejected() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(1, InstanceState::Cancelled).unwrap();
    // Cancelled は terminal → 自動削除される → deadline 設定不可
    assert!(mgr.get(1).is_err());
}

#[test]
fn multiple_independent_instances() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    create_test_instance(&mut mgr, 2);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(2, InstanceState::Playing).unwrap();
    mgr.pause(1).unwrap();
    // Instance 1 is Paused, Instance 2 is still Playing
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
    assert_eq!(mgr.get(2).unwrap().state, InstanceState::Playing);
}

// =========================================================================
// D1b-T 追加: 無効 group_id への各操作（InvalidGroupId エラーパス）
// =========================================================================

#[test]
fn operations_on_nonexistent_group_id_return_invalid_group_id() {
    let mut mgr = InstanceManager::new();
    assert_eq!(
        mgr.get_mut(999).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.transition(999, InstanceState::Playing).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.pause(999).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.set_pause_start(999, 1.0).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.resume(999, 1.0).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.set_finish_deadline(999, 1.0).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
}

// =========================================================================
// D1b-T 追加: 不正状態遷移の特性化（既存 ID でも InvalidGroupId を返す現行仕様）
// =========================================================================

#[test]
fn invalid_transition_on_existing_instance_reports_invalid_group_id() {
    // 特性化: transition は「不正遷移」も InvalidGroupId(gid) として報告する
    // （エラー型が ID 不在と遷移不正を区別しない現行挙動を固定）
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    let err = mgr.transition(1, InstanceState::Paused).unwrap_err();
    assert_eq!(err, RuntimeError::InvalidGroupId(1));
    // 失敗してもインスタンスと状態は保持される
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Created);
}

#[test]
fn pause_on_created_rejected_keeps_state() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    // Created → Paused は不正（pause() 経由でも拒否）
    assert!(mgr.pause(1).is_err());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Created);
}

#[test]
fn resume_on_playing_rejected() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    // Playing → Playing は不正遷移（Paused からのみ Resume 可能）
    assert!(mgr.resume(1, 1.0).is_err());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

// =========================================================================
// D1b-T 追加: pause_start 未設定での resume（防御パス）
// =========================================================================

#[test]
fn resume_without_pause_start_keeps_end_time() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    // pause() のみで set_pause_start を呼ばない（facade 非経由の防御パス）
    mgr.pause(1).unwrap();
    assert!(mgr.get(1).unwrap().pause_start.is_none());

    let new_end = mgr.resume(1, 3.0).unwrap();
    // pause_start 不在なら pause_accumulated と end_time は変化しない
    assert_eq!(mgr.get(1).unwrap().pause_accumulated, 0.0);
    assert_eq!(new_end, 2.0);
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

// =========================================================================
// D1b-T 追加: get_mut / remove / create_instance の境界
// =========================================================================

#[test]
fn get_mut_allows_field_mutation() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.get_mut(1).unwrap().loops_completed = 5;
    assert_eq!(mgr.get(1).unwrap().loops_completed, 5);
}

#[test]
fn remove_deletes_instance_and_is_noop_for_unknown_id() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.remove(1);
    assert!(mgr.get(1).is_err());
    // 不在 ID の remove は panic せず no-op
    mgr.remove(999);
}

#[test]
fn create_instance_with_same_group_id_overwrites() {
    // 特性化: 同一 group_id での再作成は旧インスタンスを上書きする
    // （facade は group_id を単調増加で採番するため通常は発生しない）
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.create_instance(
        1,
        "other_sb",
        InterruptionPolicy::Cancel,
        5.0,
        1.0,
        3.0,
        1,
        8.0,
        5.0,
        3.0,
        None,
        0.0,
        EasingFunction::Named(EasingName::Linear),
        0,
    );
    let inst = mgr.get(1).unwrap();
    assert_eq!(inst.storyboard_name, "other_sb");
    assert_eq!(inst.state, InstanceState::Created);
}

#[test]
fn trigger_states_initialized_sequential_and_unfired() {
    let mut mgr = InstanceManager::new();
    mgr.create_instance(
        1,
        "sb",
        InterruptionPolicy::Conclude,
        0.0,
        1.0,
        2.0,
        1,
        2.0,
        0.0,
        2.0,
        None,
        0.0,
        EasingFunction::Named(EasingName::Linear),
        3,
    );
    let inst = mgr.get(1).unwrap();
    assert_eq!(inst.trigger_states.len(), 3);
    for (i, ts) in inst.trigger_states.iter().enumerate() {
        assert_eq!(ts.trigger_index, i);
        assert!(!ts.fired);
    }
}

// =========================================================================
// D1b-T 追加: check_finish_deadlines の複合条件
// =========================================================================

#[test]
fn check_finish_deadlines_ignores_instances_without_deadline() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    create_test_instance(&mut mgr, 2);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(2, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, 5.0).unwrap();
    // gid=2 は deadline 未設定 → 対象外
    let expired = mgr.check_finish_deadlines(10.0);
    assert_eq!(expired, vec![1]);
}

// =========================================================================
// D1b-V 追加: 数値境界の特性化（非単調時刻入力・NaN deadline）
// =========================================================================

/// 特性化: current_time < pause_start の resume は pause_duration が負となり
/// pause_accumulated / end_time が過去方向へ補正される（時刻単調性の未検証: proposals.md P15）
#[test]
fn resume_with_time_before_pause_start_shrinks_end_time() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1); // end_time = 2.0
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.pause(1).unwrap();
    mgr.set_pause_start(1, 5.0).unwrap();

    // pause_start=5.0 より過去の時刻で resume → pause_duration = -2.0
    let new_end = mgr.resume(1, 3.0).unwrap();
    assert_eq!(mgr.get(1).unwrap().pause_accumulated, -2.0);
    assert_eq!(new_end, 0.0); // 2.0 + (-2.0)
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

/// 特性化: NaN deadline は `current_time >= deadline` が常に false となり発火しない
/// （NaN 流入経路の有限性検証は未実装: proposals.md P8/P14）
#[test]
fn nan_finish_deadline_never_fires() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, f64::NAN).unwrap();
    assert!(mgr.check_finish_deadlines(f64::MAX).is_empty());
    assert!(mgr.check_finish_deadlines(f64::INFINITY).is_empty());
}

/// 特性化: +inf deadline は有限の current_time では発火せず、inf 同士の比較でのみ発火する
#[test]
fn infinite_finish_deadline_fires_only_at_infinity() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, f64::INFINITY).unwrap();
    assert!(mgr.check_finish_deadlines(f64::MAX).is_empty());
    assert_eq!(mgr.check_finish_deadlines(f64::INFINITY), vec![1]);
}

#[test]
fn transition_to_cancelled_removes_instance() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(1, InstanceState::Cancelled).unwrap();
    assert!(mgr.get(1).is_err());
}

#[test]
fn transition_to_trimmed_removes_instance() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(1, InstanceState::Trimmed).unwrap();
    assert!(mgr.get(1).is_err());
}

#[test]
fn transition_to_compressed_removes_instance() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(1, InstanceState::Compressed).unwrap();
    assert!(mgr.get(1).is_err());
}
