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
