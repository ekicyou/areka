//! 購読変数ごとのタイムテーブル管理と時刻ベース評価。

use std::collections::HashMap;

use crate::compile::{CompiledSegment, CompiledStoryboard, VariableTypeHint};

use super::instance_manager::StoryboardInstance;
use super::instance_state::InstanceState;
use super::interpolator::{Interpolator, ObjectInternPool};
use super::types::EvaluatedValue;

/// 変数ごとのタイムライン。
pub(crate) struct VariableTimeline {
    pub entries: Vec<TimelineEntry>,
}

/// 1 つの group_id に属するセグメント群。
pub(crate) struct TimelineEntry {
    pub group_id: u64,
    pub segments: Vec<CompiledSegment>,
    pub variable_type: VariableTypeHint,
}

/// 購読変数ごとのタイムテーブル管理と時刻ベース評価を行う内部コンポーネント。
pub(crate) struct TimelineManager {
    timelines: HashMap<String, VariableTimeline>,
    intern_pool: ObjectInternPool,
}

impl TimelineManager {
    pub fn new() -> Self {
        Self {
            timelines: HashMap::new(),
            intern_pool: ObjectInternPool::new(),
        }
    }

    /// コンパイル結果をタイムテーブルに追加。
    pub fn insert_entries(&mut self, group_id: u64, compiled: &CompiledStoryboard) {
        for (var_name, timeline) in &compiled.timelines {
            let entry = TimelineEntry {
                group_id,
                segments: timeline.segments.clone(),
                variable_type: timeline.variable_type.clone(),
            };

            self.timelines
                .entry(var_name.clone())
                .or_insert_with(|| VariableTimeline {
                    entries: Vec::new(),
                })
                .entries
                .push(entry);
        }
    }

    /// 指定変数の現在値を評価。
    ///
    /// - 最新（最大）group_id 優先（Tier 2 暫定動作）
    /// - 終了済みエントリは自動破棄
    /// - Pause 中のインスタンスは pause 開始時点の effective_time で固定
    pub fn evaluate(
        &mut self,
        variable_name: &str,
        current_time: f64,
        instances: &HashMap<u64, StoryboardInstance>,
    ) -> Option<EvaluatedValue> {
        let timeline = self.timelines.get_mut(variable_name)?;

        let mut best_value: Option<(u64, EvaluatedValue)> = None;
        let mut expired_indices: Vec<usize> = Vec::new();

        for (idx, entry) in timeline.entries.iter().enumerate() {
            let instance = match instances.get(&entry.group_id) {
                Some(inst) => inst,
                None => {
                    // インスタンスが存在しない(Concluded で削除済み等) → expired
                    expired_indices.push(idx);
                    continue;
                }
            };

            // effective_time 計算
            let effective_time = calculate_effective_time(current_time, instance);

            // アクティブなセグメントを探す
            let eval = evaluate_segments(
                &entry.segments,
                &entry.variable_type,
                effective_time,
                &mut self.intern_pool,
            );

            match eval {
                Some(val) => {
                    // 最新（最大）group_id 優先
                    match &best_value {
                        Some((best_gid, _)) if entry.group_id <= *best_gid => {}
                        _ => {
                            best_value = Some((entry.group_id, val));
                        }
                    }
                }
                None => {
                    // 全セグメント終了済み → expired
                    expired_indices.push(idx);
                }
            }
        }

        // 終了済みエントリを逆順で削除（インデックスがずれないように）
        for idx in expired_indices.into_iter().rev() {
            timeline.entries.remove(idx);
        }

        // 空になったタイムラインはそのまま残す（cleanup は呼び出し側）
        best_value.map(|(_, val)| val)
    }

    /// Conclude 用: group_id の全変数の最終値を取得。
    pub fn collect_final_values(&self, group_id: u64) -> HashMap<String, EvaluatedValue> {
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id {
                    if let Some(last_seg) = entry.segments.last() {
                        // 最終セグメントの to_value を最終値とする
                        let val = Interpolator::interpolate(
                            last_seg,
                            &entry.variable_type,
                            1.0, // progress_t = 1.0 で最終値
                        );
                        result.insert(var_name.clone(), val);
                    }
                }
            }
        }

        result
    }

    /// group_id の全エントリを削除。
    pub fn remove_entries(&mut self, group_id: u64) {
        for timeline in self.timelines.values_mut() {
            timeline.entries.retain(|e| e.group_id != group_id);
        }
    }

    /// group_id に関連するタイムテーブルエントリが存在するか確認。
    #[allow(dead_code)]
    pub fn has_entries(&self, group_id: u64) -> bool {
        self.timelines
            .values()
            .any(|tl| tl.entries.iter().any(|e| e.group_id == group_id))
    }

    /// 指定変数のタイムラインを取得（読み取り専用）。
    /// 競合検出（detect_overlaps）で使用。
    pub(crate) fn get_timeline(&self, variable_name: &str) -> Option<&VariableTimeline> {
        self.timelines.get(variable_name)
    }

    /// group_id に属する全変数の値を start_time 時点で評価する。
    /// Cancel/Trim 戦略で使用。
    pub(crate) fn evaluate_all_for_group(
        &mut self,
        group_id: u64,
        current_time: f64,
        instances: &HashMap<u64, StoryboardInstance>,
    ) -> HashMap<String, EvaluatedValue> {
        let instance = match instances.get(&group_id) {
            Some(inst) => inst,
            None => return HashMap::new(),
        };

        let effective_time = calculate_effective_time(current_time, instance);
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id {
                    if let Some(val) = evaluate_segments(
                        &entry.segments,
                        &entry.variable_type,
                        effective_time,
                        &mut self.intern_pool,
                    ) {
                        result.insert(var_name.clone(), val);
                    }
                }
            }
        }

        result
    }

    /// 現在再生中セグメントの最終値を group_id 単位で取得する。
    /// Conclude 戦略専用。各変数について、start_time 時点でアクティブな
    /// セグメントの to_value (progress_t=1.0) を返す。
    /// 未開始のセグメントはスキップする。
    pub(crate) fn collect_current_segment_final_values(
        &self,
        group_id: u64,
        current_time: f64,
        instances: &HashMap<u64, StoryboardInstance>,
    ) -> HashMap<String, EvaluatedValue> {
        let instance = match instances.get(&group_id) {
            Some(inst) => inst,
            None => return HashMap::new(),
        };

        let effective_time = calculate_effective_time(current_time, instance);
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id {
                    // アクティブなセグメントを特定
                    if let Some(active_seg) =
                        find_active_segment(&entry.segments, effective_time)
                    {
                        // アクティブセグメントの最終値 (progress_t=1.0) を取得
                        let val = Interpolator::interpolate(
                            active_seg,
                            &entry.variable_type,
                            1.0,
                        );
                        result.insert(var_name.clone(), val);
                    }
                }
            }
        }

        result
    }
}

/// effective_time を計算する。
///
/// `effective_time = (current_time - loop_start_time - pause_accumulated) * time_scale`
/// Pause 中は pause 開始時点で固定。
/// `loop_start_time` の初期値は `start_time` と同一のため、ループなし時は既存動作と互換。
pub(crate) fn calculate_effective_time(current_time: f64, instance: &StoryboardInstance) -> f64 {
    let raw_time = if instance.state == InstanceState::Paused {
        // Pause 中は pause_start 時点で固定
        match instance.pause_start {
            Some(pause_start) => {
                pause_start - instance.loop_start_time - instance.pause_accumulated
            }
            None => current_time - instance.loop_start_time - instance.pause_accumulated,
        }
    } else {
        current_time - instance.loop_start_time - instance.pause_accumulated
    };
    // time_scale は再生速度倍率（1.0 倍速が標準、2.0 で 2 倍速）
    // time_scale が大きいほどタイムラインが速く進む
    raw_time * instance.time_scale
}

/// セグメント列から現在値を評価する。
///
/// 全セグメント終了済みの場合は `None` を返す（エントリ破棄のシグナル）。
fn evaluate_segments(
    segments: &[CompiledSegment],
    variable_type: &VariableTypeHint,
    effective_time: f64,
    intern_pool: &mut ObjectInternPool,
) -> Option<EvaluatedValue> {
    if segments.is_empty() {
        return None;
    }

    // 最終セグメントの end_time を超えている → 全終了
    let last_seg = segments.last().unwrap();
    if effective_time >= last_seg.end_time && last_seg.end_time > last_seg.start_time {
        return None;
    }

    // アクティブなセグメントを探す
    for seg in segments {
        if effective_time < seg.start_time {
            // まだこのセグメントに到達していない → from_value を返す
            let val =
                Interpolator::interpolate_with_pool(seg, variable_type, 0.0, Some(intern_pool));
            return Some(val);
        }

        let duration = seg.end_time - seg.start_time;
        if duration <= 0.0 {
            // 即時遷移（duration=0）
            if effective_time >= seg.start_time {
                let val =
                    Interpolator::interpolate_with_pool(seg, variable_type, 1.0, Some(intern_pool));
                return Some(val);
            }
        } else if effective_time < seg.end_time {
            // このセグメント内
            let progress_t = (effective_time - seg.start_time) / duration;
            let val = Interpolator::interpolate_with_pool(
                seg,
                variable_type,
                progress_t,
                Some(intern_pool),
            );
            return Some(val);
        }
        // seg.end_time <= effective_time → 次のセグメントへ
    }

    // 最終セグメントが即時遷移の場合、end_time == start_time で上のループを通過後ここに来る
    // 最終セグメントが完了したが、全体終了とはみなさない（最後の値を保持）
    let val = Interpolator::interpolate_with_pool(last_seg, variable_type, 1.0, Some(intern_pool));
    Some(val)
}

/// effective_time 時点でアクティブなセグメントを検索する。
/// Conclude 戦略の collect_current_segment_final_values で使用。
/// 未開始セグメントはスキップし、アクティブなセグメントを返す。
fn find_active_segment(segments: &[CompiledSegment], effective_time: f64) -> Option<&CompiledSegment> {
    if segments.is_empty() {
        return None;
    }

    let mut active: Option<&CompiledSegment> = None;

    for seg in segments {
        if effective_time < seg.start_time {
            // まだこのセグメントに到達していない
            break;
        }

        let duration = seg.end_time - seg.start_time;
        if duration <= 0.0 {
            // 即時遷移
            if effective_time >= seg.start_time {
                active = Some(seg);
            }
        } else if effective_time < seg.end_time {
            // このセグメント内
            return Some(seg);
        } else {
            // このセグメントは終了済み → 次へ
            active = Some(seg);
        }
    }

    active
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::compile::{CompiledVariableTimeline, VariableTypeHint};
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
            total_base_duration: end - start,
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
}
