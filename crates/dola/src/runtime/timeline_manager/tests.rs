use super::*;
use std::collections::BTreeMap;

use crate::compile::{CompiledVariableTimeline, VariableTypeHint};
use crate::easing::{EasingFunction, EasingName};
use crate::runtime::instance_state::InstanceState;
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

/// 任意のセグメント列を持つ 1 変数 CompiledStoryboard を生成する。
fn make_compiled_with_segments(
    var_name: &str,
    segments: Vec<CompiledSegment>,
) -> CompiledStoryboard {
    let total = segments.last().map(|s| s.end_time).unwrap_or(0.0);
    let mut timelines = BTreeMap::new();
    timelines.insert(
        var_name.to_string(),
        CompiledVariableTimeline {
            variable_type: VariableTypeHint::Float,
            segments,
            base_duration: total,
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
        total_base_duration: total,
        triggers: Vec::new(),
    }
}

fn float_segment(start: f64, end: f64, from: f64, to: f64) -> CompiledSegment {
    CompiledSegment {
        start_time: start,
        end_time: end,
        from_value: TransitionValue::Scalar(from),
        to_value: TransitionValue::Scalar(to),
        easing: None,
    }
}

fn make_instance(start_time: f64) -> StoryboardInstance {
    StoryboardInstance {
        storyboard_name: "test".to_string(),
        state: InstanceState::Playing,
        interruption_policy: InterruptionPolicy::Conclude,
        time_scale: 1.0,
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

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    // t=0.5 → progress 0.5 → value 50.0
    let val = mgr.evaluate("x", 0.5, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(50.0)));
}

#[test]
fn evaluate_nonexistent_variable() {
    let mut mgr = TimelineManager::new();
    let instances = BTreeMap::new();
    assert!(mgr.evaluate("nonexistent", 0.0, &instances).is_none());
}

#[test]
fn expired_entry_auto_removal() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    // t=2.0 → 全セグメント終了でもインスタンスが生きている間は最後の値・項目は残る
    assert_eq!(
        mgr.evaluate("x", 2.0, &instances),
        Some(EvaluatedValue::Float(100.0))
    );
    assert!(mgr.has_entries(1));

    // 項目を消すのは remove_entries だけ
    mgr.remove_entries(1);
    assert!(mgr.evaluate("x", 2.0, &instances).is_none());
}

#[test]
fn latest_group_id_priority() {
    let mut mgr = TimelineManager::new();

    let compiled1 = make_float_compiled_storyboard("x", 0.0, 50.0, 0.0, 2.0);
    let compiled2 = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 2.0);
    mgr.insert_entries(1, &compiled1);
    mgr.insert_entries(2, &compiled2);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));
    instances.insert(2, make_instance(0.0));

    // group_id=2 が最新 → 100.0 系列が採用
    let val = mgr.evaluate("x", 1.0, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(50.0))); // progress=0.5 of 0→100
}

#[test]
fn pause_freezes_value() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 2.0);
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    let mut inst = make_instance(0.0);
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

    let mut instances = BTreeMap::new();
    let mut inst = make_instance(0.0);
    inst.time_scale = 2.0; // 2倍速
    instances.insert(1, inst);

    // t=0.5, time_scale=2.0 → effective_time=1.0, progress=0.5
    let val = mgr.evaluate("x", 0.5, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(50.0)));
}

// =====================================================================
// evaluate のエッジケース（インスタンス欠損・未到達・即時遷移）
// =====================================================================

#[test]
fn evaluate_removes_entry_when_instance_missing() {
    // Concluded 等でインスタンスが削除済みのエントリは expired として破棄される
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);

    let instances = BTreeMap::new(); // インスタンスなし
    let val = mgr.evaluate("x", 0.5, &instances);
    assert!(val.is_none());
    assert!(!mgr.has_entries(1), "entry should be removed");
}

#[test]
fn evaluate_before_first_segment_returns_from_value() {
    // delay 付きセグメント（1.0 開始）に未到達 → from_value を返す
    let mut mgr = TimelineManager::new();
    let compiled = make_compiled_with_segments("x", vec![float_segment(1.0, 2.0, 10.0, 20.0)]);
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    let val = mgr.evaluate("x", 0.5, &instances);
    assert_eq!(val, Some(EvaluatedValue::Float(10.0)));
}

#[test]
fn evaluate_zero_duration_segment_returns_to_value() {
    // 即時遷移（duration=0）セグメントは start_time 以降 to_value を返し、期限切れにならない
    let mut mgr = TimelineManager::new();
    let compiled = make_compiled_with_segments("x", vec![float_segment(1.0, 1.0, 0.0, 100.0)]);
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    // 到達前: from_value
    assert_eq!(
        mgr.evaluate("x", 0.5, &instances),
        Some(EvaluatedValue::Float(0.0))
    );
    // 到達後: to_value（end_time == start_time のため expired 判定されない）
    assert_eq!(
        mgr.evaluate("x", 1.0, &instances),
        Some(EvaluatedValue::Float(100.0))
    );
    assert_eq!(
        mgr.evaluate("x", 100.0, &instances),
        Some(EvaluatedValue::Float(100.0))
    );
    assert!(mgr.has_entries(1), "zero-duration entry should persist");
}

#[test]
fn evaluate_multi_segment_progression() {
    // 連続 2 セグメント: [0,1]: 0→10, [1,2]: 10→50
    let mut mgr = TimelineManager::new();
    let compiled = make_compiled_with_segments(
        "x",
        vec![
            float_segment(0.0, 1.0, 0.0, 10.0),
            float_segment(1.0, 2.0, 10.0, 50.0),
        ],
    );
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    // セグメント1内 (t=0.5): progress=0.5 → 5.0
    assert_eq!(
        mgr.evaluate("x", 0.5, &instances),
        Some(EvaluatedValue::Float(5.0))
    );
    // セグメント2内 (t=1.5): progress=0.5 → 30.0
    assert_eq!(
        mgr.evaluate("x", 1.5, &instances),
        Some(EvaluatedValue::Float(30.0))
    );
    // 全セグメント終了 (t=2.5): 最後の値を保ち、項目は残る
    assert_eq!(
        mgr.evaluate("x", 2.5, &instances),
        Some(EvaluatedValue::Float(50.0))
    );
    assert!(mgr.has_entries(1));
}

#[test]
fn instant_segment_does_not_hide_later_segments() {
    // 即時のセグメントで返して抜けると、後ろのセグメントの値が出ず即時の値で止まっていた
    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));
    for (segments, t, expected) in [
        (
            vec![
                float_segment(0.0, 0.0, 0.0, 5.0),
                float_segment(0.0, 1.0, 5.0, 10.0),
            ],
            0.5,
            7.5,
        ),
        (
            vec![
                float_segment(0.0, 1.0, 0.0, 1.0),
                float_segment(1.0, 1.0, 1.0, 5.0),
                float_segment(1.0, 2.0, 5.0, 0.0),
            ],
            1.5,
            2.5,
        ),
    ] {
        let mut mgr = TimelineManager::new();
        mgr.insert_entries(1, &make_compiled_with_segments("x", segments));
        assert_eq!(
            mgr.evaluate("x", t, &instances),
            Some(EvaluatedValue::Float(expected))
        );
    }
}

#[test]
fn not_yet_started_instance_does_not_override_started_one() {
    // 親（gid 1）が x を 0→1 に 2 秒・子（gid 2）は 1.5 秒に始まって x を 5→6。
    // 始まる前の子が group_id の大きさで勝つと、t=1.0 で x が子の from の 5 に跳んでいた
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(1, &make_float_compiled_storyboard("x", 0.0, 1.0, 0.0, 2.0));
    mgr.insert_entries(2, &make_float_compiled_storyboard("x", 5.0, 6.0, 0.0, 1.0));
    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));
    instances.insert(2, make_instance(1.5));

    assert_eq!(
        mgr.evaluate("x", 1.0, &instances),
        Some(EvaluatedValue::Float(0.5))
    );
    // 子が始まれば子が勝つ
    assert_eq!(
        mgr.evaluate("x", 2.0, &instances),
        Some(EvaluatedValue::Float(5.5))
    );
    // 全部が始まる前なら、今までどおり group_id の大きい方の from
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(1, &make_float_compiled_storyboard("x", 0.0, 1.0, 0.0, 2.0));
    mgr.insert_entries(2, &make_float_compiled_storyboard("x", 5.0, 6.0, 0.0, 1.0));
    instances.insert(1, make_instance(1.2));
    assert_eq!(
        mgr.evaluate("x", 1.0, &instances),
        Some(EvaluatedValue::Float(5.0))
    );
}

#[test]
fn looped_instance_in_leading_delay_counts_as_started() {
    // 2 周目の先頭の遅れ（0 ≤ t < 最初のセグメントの始まり）でも、周回を終えたインスタンスは始まった扱い
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(1, &make_float_compiled_storyboard("x", 0.0, 1.0, 0.0, 10.0));
    mgr.insert_entries(2, &make_float_compiled_storyboard("x", 5.0, 6.0, 0.5, 1.5));
    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));
    let mut looped = make_instance(0.0);
    looped.loops_completed = 1;
    looped.loop_start_time = 2.0;
    instances.insert(2, looped);

    assert_eq!(
        mgr.evaluate("x", 2.2, &instances),
        Some(EvaluatedValue::Float(5.0))
    );
}

// =====================================================================
// calculate_effective_time（純粋関数の直接検証）
// =====================================================================

#[test]
fn effective_time_playing_basic() {
    let mut inst = make_instance(0.0);
    inst.loop_start_time = 2.0;
    inst.pause_accumulated = 1.0;
    inst.time_scale = 2.0;
    // (10.0 - 2.0 - 1.0) * 2.0 = 14.0
    assert_eq!(calculate_effective_time(10.0, &inst), 14.0);
}

#[test]
fn effective_time_paused_frozen_at_pause_start() {
    let mut inst = make_instance(0.0);
    inst.state = InstanceState::Paused;
    inst.pause_start = Some(3.0);
    // current_time に依らず pause_start 基準: (3.0 - 0.0 - 0.0) * 1.0 = 3.0
    assert_eq!(calculate_effective_time(100.0, &inst), 3.0);
}

// =====================================================================
// collect_final_values / evaluate_all_for_group /
// collect_current_segment_final_values / get_timeline
// =====================================================================

#[test]
fn collect_final_values_unknown_group_returns_empty() {
    let mut mgr = TimelineManager::new();
    let compiled = make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0);
    mgr.insert_entries(1, &compiled);

    let finals = mgr.collect_final_values(999);
    assert!(finals.is_empty());
}

#[test]
fn evaluate_all_for_group_returns_all_variables() {
    let mut mgr = TimelineManager::new();
    // 同一 group_id に 2 変数を登録
    mgr.insert_entries(
        1,
        &make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0),
    );
    mgr.insert_entries(1, &make_float_compiled_storyboard("y", 0.0, 10.0, 0.0, 1.0));

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    let values = mgr.evaluate_all_for_group(1, 0.5, &instances);
    assert_eq!(values.len(), 2);
    assert_eq!(values.get("x"), Some(&EvaluatedValue::Float(50.0)));
    assert_eq!(values.get("y"), Some(&EvaluatedValue::Float(5.0)));
}

#[test]
fn evaluate_all_for_group_missing_instance_returns_empty() {
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(
        1,
        &make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0),
    );

    let instances = BTreeMap::new();
    let values = mgr.evaluate_all_for_group(1, 0.5, &instances);
    assert!(values.is_empty());
}

#[test]
fn evaluate_all_for_group_ignores_other_groups() {
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(
        1,
        &make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0),
    );
    mgr.insert_entries(2, &make_float_compiled_storyboard("y", 0.0, 10.0, 0.0, 1.0));

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));
    instances.insert(2, make_instance(0.0));

    let values = mgr.evaluate_all_for_group(1, 0.5, &instances);
    assert_eq!(values.len(), 1);
    assert!(values.contains_key("x"));
}

#[test]
fn collect_current_segment_final_values_returns_active_segment_final() {
    // 2 セグメント中、アクティブな 1 番目のセグメントの to_value を返す
    let mut mgr = TimelineManager::new();
    let compiled = make_compiled_with_segments(
        "x",
        vec![
            float_segment(0.0, 1.0, 0.0, 10.0),
            float_segment(1.0, 2.0, 10.0, 50.0),
        ],
    );
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    // t=0.5: セグメント1がアクティブ → to_value=10.0（50.0 ではない）
    let vals = mgr.collect_current_segment_final_values(1, 0.5, &instances);
    assert_eq!(vals.get("x"), Some(&EvaluatedValue::Float(10.0)));

    // t=1.5: セグメント2がアクティブ → to_value=50.0
    let vals = mgr.collect_current_segment_final_values(1, 1.5, &instances);
    assert_eq!(vals.get("x"), Some(&EvaluatedValue::Float(50.0)));
}

#[test]
fn collect_current_segment_final_values_before_first_segment_is_empty() {
    // 未開始セグメントのみ（delay 中）→ アクティブなし → 空
    let mut mgr = TimelineManager::new();
    let compiled = make_compiled_with_segments("x", vec![float_segment(1.0, 2.0, 10.0, 20.0)]);
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    let vals = mgr.collect_current_segment_final_values(1, 0.5, &instances);
    assert!(vals.is_empty());
}

#[test]
fn collect_current_segment_final_values_missing_instance_returns_empty() {
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(
        1,
        &make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0),
    );

    let instances = BTreeMap::new();
    let vals = mgr.collect_current_segment_final_values(1, 0.5, &instances);
    assert!(vals.is_empty());
}

#[test]
fn collect_current_segment_final_values_after_all_segments_uses_last() {
    // 全セグメント終了後はラスト・セグメントがアクティブ扱い → 最終値
    let mut mgr = TimelineManager::new();
    let compiled = make_compiled_with_segments("x", vec![float_segment(0.0, 1.0, 0.0, 10.0)]);
    mgr.insert_entries(1, &compiled);

    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    let vals = mgr.collect_current_segment_final_values(1, 5.0, &instances);
    assert_eq!(vals.get("x"), Some(&EvaluatedValue::Float(10.0)));
}

#[test]
fn get_timeline_returns_some_for_known_variable() {
    let mut mgr = TimelineManager::new();
    mgr.insert_entries(
        1,
        &make_float_compiled_storyboard("x", 0.0, 100.0, 0.0, 1.0),
    );

    assert!(mgr.get_timeline("x").is_some());
    assert_eq!(mgr.get_timeline("x").unwrap().entries.len(), 1);
    assert!(mgr.get_timeline("nonexistent").is_none());
}

/// イージングの行き過ぎは変数の値域へ収める（#33・要件 11.11）
#[test]
fn easing_overshoot_is_clamped_to_variable_range() {
    let mut instances = BTreeMap::new();
    instances.insert(1, make_instance(0.0));

    // Float [0,1]・BackOut は t=0.6 で 1.0 を越える
    let mut seg = float_segment(0.0, 1.0, 0.0, 1.0);
    seg.easing = Some(EasingFunction::Named(EasingName::BackOut));
    let mut compiled = make_compiled_with_segments("x", vec![seg]);
    let tl = compiled.timelines.get_mut("x").unwrap();
    tl.min_value = Some(0.0);
    tl.max_value = Some(1.0);

    // Integer [0,100]・BackIn は t=0.3 で 0 を下回る（丸めると -22）
    let mut seg = float_segment(0.0, 1.0, 0.0, 100.0);
    seg.easing = Some(EasingFunction::Named(EasingName::BackIn));
    let mut int_compiled = make_compiled_with_segments("n", vec![seg]);
    let tl = int_compiled.timelines.get_mut("n").unwrap();
    tl.variable_type = VariableTypeHint::Integer;
    tl.min_value = Some(0.0);
    tl.max_value = Some(100.0);

    let mut mgr = TimelineManager::new();
    mgr.insert_entries(1, &compiled);
    mgr.insert_entries(1, &int_compiled);

    assert_eq!(
        mgr.evaluate("x", 0.6, &instances),
        Some(EvaluatedValue::Float(1.0))
    );
    assert_eq!(
        mgr.evaluate("n", 0.3, &instances),
        Some(EvaluatedValue::Integer(0))
    );
}
