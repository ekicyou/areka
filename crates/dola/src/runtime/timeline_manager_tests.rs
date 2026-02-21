use super::*;
use std::collections::BTreeMap;

use crate::compile::{CompiledVariableTimeline, VariableTypeHint};
use crate::easing::{EasingFunction, EasingName};
use crate::storyboard::InterruptionPolicy;
use crate::transition::TransitionValue;

fn make_float_compiled_storyboard(
    var_name: &str,
    from: f64,
    to: f64,
    start: f64,
    end: f64,
) -> CompiledStoryboard {
    let mut timelines = BTreeMap::new();
    timelines.insert(
        var_name.to_string(),
        CompiledVariableTimeline {
            variable_type: VariableTypeHint::Float,
            segments: vec![CompiledSegment {
                start_time: start,
                end_time: end,
                from_value: TransitionValue::Scalar(from),
                to_value: TransitionValue::Scalar(to),
                easing: None,
            }],
            base_duration: end - start,
            min_value: None,
            max_value: None,
        },
    );

    CompiledStoryboard {
        storyboard_name: "test".to_string(),
        start_time: 0.0,
        timelines,
        time_scale: 1.0,
        loop_count: 1,
        interruption_policy: InterruptionPolicy::Conclude,
        loop_offset: None,
        total_base_duration: end - start,
        triggers: Vec::new(),
    }
}

fn make_instance(group_id: u64, start_time: f64) -> StoryboardInstance {
    StoryboardInstance {
        group_id,
        storyboard_name: "test".to_string(),
        state: InstanceState::Playing,
        interruption_policy: InterruptionPolicy::Conclude,
        start_time,
        time_scale: 1.0,
        base_duration: 1.0,
        pause_accumulated: 0.0,
        pause_start: None,
        loop_count: 1,
        loops_completed: 0,
        finish_deadline: None,
        end_time: start_time + 1.0,
        loop_start_time: start_time,
        loop_duration: 1.0,
        loop_offset_min: None,
        loop_offset_max: 0.0,
        loop_offset_easing: EasingFunction::Named(EasingName::Linear),
        trigger_states: Vec::new(),
    }
}

#[test]
fn insert_and_evaluate_float() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);

    let mut instances = HashMap::new();
    instances.insert(1, make_instance(1, 0.0));

    // t=0.5 → progress 0.5 → value 50.0
    let val = mgr.evaluate("x", 0.5, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(50.0)));
}

#[test]
fn evaluate_nonexistent_variable() {
    let mut mgr = TimelineManager::new();
    let instances = HashMap::new();
    assert!(mgr.evaluate("nonexistent", 0.0, &instances).is_none());
}

#[test]
fn expired_entry_auto_removal() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);

    let mut instances = HashMap::new();
    instances.insert(1, make_instance(1, 0.0));

    // t=2.0 → 全セグメント終了 → None + エントリ破棄
    let val = mgr.evaluate("x", 2.0, &instances);
    assert!(val.is_none());
    assert!(!mgr.has_entries(1));
}

#[test]
fn latest_group_id_priority() {
    let mut mgr = TimelineManager::new();

    let compiled1 = make_float_compiled_storyboard("x", 0.0, 50.0, 0.0, 2.0);
    let compiled2 = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 2.0);
    mgr.insert_entries(1, &compiled1);
    mgr.insert_entries(2, &compiled2);

    let mut instances = HashMap::new();
    instances.insert(1, make_instance(1, 0.0));
    instances.insert(2, make_instance(2, 0.0));

    // group_id=2 が最新 → 100.0 系列が採用
    let val = mgr.evaluate("x", 1.0, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(50.0))); // progress=0.5 of 0→100
}

#[test]
fn pause_freezes_value() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 2.0);
    mgr.insert_entries(1, &compiled);

    let mut instances = HashMap::new();
    let mut inst = make_instance(1, 0.0);
    inst.base_duration = 2.0;
    inst.end_time = 2.0;
    inst.state = InstanceState::Paused;
    inst.pause_start = Some(0.5); // t=0.5 で Pause
    instances.insert(1, inst);

    // t=10.0 でも Pause 中は t=0.5 の値を返す
    let val = mgr.evaluate("x", 10.0, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(25.0))); // effective=0.5, progress=0.25
}

#[test]
fn collect_final_values() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);

    let finals = mgr.collect_final_values(1);
    assert_eq!(finals.get("x"), Some(&EvaluatedValue::Float(100.0)));
}

#[test]
fn remove_entries() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);
    assert!(mgr.has_entries(1));

    mgr.remove_entries(1);
    assert!(!mgr.has_entries(1));
}

#[test]
fn effective_time_with_time_scale() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 2.0);
    mgr.insert_entries(1, &compiled);

    let mut instances = HashMap::new();
    let mut inst = make_instance(1, 0.0);
    inst.time_scale = 2.0; // 2倍速
    inst.base_duration = 2.0;
    instances.insert(1, inst);

    // t=0.5, time_scale=2.0 → effective_time=1.0, progress=0.5
    let val = mgr.evaluate("x", 0.5, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(50.0)));
}
