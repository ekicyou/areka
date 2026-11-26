use super::*;
use crate::easing::EasingName;
use crate::time::testing::{at_ms, delta_ms, ms};

fn create_test_instance(mgr: &mut InstanceManager, group_id: u64) {
    mgr.instances_mut().insert(
        group_id,
        StoryboardInstance {
            storyboard_name: "test_sb".to_string(),
            state: InstanceState::Created,
            interruption_policy: InterruptionPolicy::Conclude,
            time_scale: TimeScale::ONE,
            pause_accumulated: TimeDelta::ZERO,
            pause_start: None,
            loop_count: 1,
            loops_completed: 0,
            finish_deadline: None,
            loop_start: at_ms(0),
            lap_length: ms(2000),
            loop_offset_min: None,
            loop_offset_max: TimeSpan::ZERO,
            loop_offset_easing: EasingFunction::Named(EasingName::Linear),
            trigger_states: Vec::new(),
        },
    );
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

    mgr.pause(1, at_ms(1000)).unwrap();
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
    assert_eq!(mgr.get(1).unwrap().pause_start, Some(at_ms(1000)));

    // 2秒間 pause
    let new_end = mgr.resume(1, at_ms(3000)).unwrap();
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
    assert_eq!(mgr.get(1).unwrap().pause_accumulated, delta_ms(2000));
    assert!(mgr.get(1).unwrap().pause_start.is_none());
    // 終わりの予定 = 2.0 + 2.0(pause)
    assert_eq!(new_end, Some(at_ms(4000)));
}

#[test]
fn finish_deadline_check() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, at_ms(5000)).unwrap();

    // まだ deadline に達していない
    assert!(mgr.check_finish_deadlines(at_ms(4000)).is_empty());

    let expired = mgr.check_finish_deadlines(at_ms(5000));
    assert_eq!(expired, vec![1]);
}

#[test]
fn multiple_independent_instances() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    create_test_instance(&mut mgr, 2);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(2, InstanceState::Playing).unwrap();
    mgr.pause(1, at_ms(0)).unwrap();
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
    assert_eq!(mgr.get(2).unwrap().state, InstanceState::Playing);
}

// =========================================================================
// 無効 group_id への各操作
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
        mgr.pause(999, at_ms(1000)).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.resume(999, at_ms(1000)).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
    assert_eq!(
        mgr.set_finish_deadline(999, at_ms(1000)).err(),
        Some(RuntimeError::InvalidGroupId(999))
    );
}

// =========================================================================
// 不正状態遷移（既存 ID でも InvalidGroupId を返す）
// =========================================================================

#[test]
fn invalid_transition_on_existing_instance_reports_invalid_group_id() {
    // エラー型が ID 不在と遷移不正を区別しないので、不正遷移も InvalidGroupId(gid) になる
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
    // pause() 経由でも Created → Paused は拒否される
    assert!(mgr.pause(1, at_ms(0)).is_err());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Created);
}

#[test]
fn resume_on_playing_rejected() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    // Resume は Paused からのみ
    assert!(mgr.resume(1, at_ms(1000)).is_err());
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

// =========================================================================
// get_mut / remove の境界
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

// =========================================================================
// check_finish_deadlines の複合条件
// =========================================================================

#[test]
fn check_finish_deadlines_ignores_instances_without_deadline() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    create_test_instance(&mut mgr, 2);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.transition(2, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, at_ms(5000)).unwrap();
    // gid=2 は deadline 未設定なので対象外
    let expired = mgr.check_finish_deadlines(at_ms(10_000));
    assert_eq!(expired, vec![1]);
}

// =========================================================================
// 数値境界（非単調時刻入力・最大の時刻の deadline）
// =========================================================================

/// now < pause_start の resume は止めていた長さが負となり
/// pause_accumulated と終わりの予定が過去方向へ補正される（時刻の単調性は検証しない）
#[test]
fn resume_with_time_before_pause_start_shrinks_end_time() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1); // 周回の長さ 2 秒
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.pause(1, at_ms(5000)).unwrap();

    // pause_start より過去の時刻で resume すると止めていた長さ = -2.0 秒
    let new_end = mgr.resume(1, at_ms(3000)).unwrap();
    assert_eq!(mgr.get(1).unwrap().pause_accumulated, delta_ms(-2000));
    assert_eq!(new_end, Some(at_ms(0))); // 2.0 + (-2.0)
    assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
}

/// 型の最大の時刻の deadline は、最大の時刻でだけ発火する
#[test]
fn max_finish_deadline_fires_only_at_max() {
    let mut mgr = InstanceManager::new();
    create_test_instance(&mut mgr, 1);
    mgr.transition(1, InstanceState::Playing).unwrap();
    mgr.set_finish_deadline(1, TimePoint::MAX).unwrap();
    assert!(
        mgr.check_finish_deadlines(TimePoint::MAX - ms(1))
            .is_empty()
    );
    assert_eq!(mgr.check_finish_deadlines(TimePoint::MAX), vec![1]);
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
