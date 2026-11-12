//! 購読変数ごとのタイムテーブル管理と時刻ベース評価。

use std::collections::{BTreeMap, HashMap};

use crate::compile::{CompiledSegment, CompiledStoryboard, VariableTypeHint};

use super::instance_manager::StoryboardInstance;
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
    /// 値域（コンパイル結果の min_value／max_value）
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

impl TimelineEntry {
    /// セグメントの進捗 `progress_t` の値。Object は intern を通し、Float と Integer は値域へ収める。
    fn value_at(
        &self,
        seg: &CompiledSegment,
        progress_t: f64,
        intern_pool: &mut ObjectInternPool,
    ) -> EvaluatedValue {
        let min = self.min_value.unwrap_or(f64::NEG_INFINITY);
        let max = self.max_value.unwrap_or(f64::INFINITY);
        // clamp は min > max で panic し、max／min は NaN を捨てるので比べて収める
        let fit = |v: f64| {
            if v < min {
                min
            } else if v > max {
                max
            } else {
                v
            }
        };
        match Interpolator::interpolate_with_pool(
            seg,
            &self.variable_type,
            progress_t,
            Some(intern_pool),
        ) {
            EvaluatedValue::Float(v) => EvaluatedValue::Float(fit(v)),
            // Integer の値域は i64 から来るので丸めた後に収める
            EvaluatedValue::Integer(v) => EvaluatedValue::Integer(fit(v as f64) as i64),
            other => other,
        }
    }
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

    /// Object の値の intern の表から使われていない値を手放す（文書の差し替え）。
    pub fn release_unused_intern_values(&mut self) {
        self.intern_pool.release_unused();
    }

    /// コンパイル結果をタイムテーブルに追加。
    pub fn insert_entries(&mut self, group_id: u64, compiled: &CompiledStoryboard) {
        for (var_name, timeline) in &compiled.timelines {
            let entry = TimelineEntry {
                group_id,
                segments: timeline.segments.clone(),
                variable_type: timeline.variable_type.clone(),
                min_value: timeline.min_value,
                max_value: timeline.max_value,
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
    /// - 最後のセグメントを過ぎた変数は、インスタンスが生きている間は最後の値を保つ
    ///   （項目を消すのは `remove_entries` だけ）
    /// - Pause 中のインスタンスは pause 開始時点の effective_time で固定
    pub fn evaluate(
        &mut self,
        variable_name: &str,
        current_time: f64,
        instances: &BTreeMap<u64, StoryboardInstance>,
    ) -> Option<EvaluatedValue> {
        let timeline = self.timelines.get_mut(variable_name)?;
        // インスタンスの無い項目（取り残し）だけを捨てる。生きた項目は remove_entries だけが消す
        timeline
            .entries
            .retain(|e| instances.contains_key(&e.group_id));

        let mut best_value: Option<(u64, EvaluatedValue)> = None;

        for entry in &timeline.entries {
            let Some(instance) = instances.get(&entry.group_id) else {
                continue;
            };

            let effective_time = value_time(current_time, instance);

            // アクティブなセグメントを探す
            let eval = evaluate_segments(entry, effective_time, &mut self.intern_pool);

            if let Some(val) = eval {
                // 最新（最大）group_id 優先
                match &best_value {
                    Some((best_gid, _)) if entry.group_id <= *best_gid => {}
                    _ => {
                        best_value = Some((entry.group_id, val));
                    }
                }
            }
        }

        best_value.map(|(_, val)| val)
    }

    /// Conclude 用: group_id の全変数の最終値を取得。
    pub fn collect_final_values(&mut self, group_id: u64) -> HashMap<String, EvaluatedValue> {
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id {
                    if let Some(last_seg) = entry.segments.last() {
                        // 最終セグメントの to_value を最終値とする
                        let val = entry.value_at(last_seg, 1.0, &mut self.intern_pool);
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
        instances: &BTreeMap<u64, StoryboardInstance>,
    ) -> HashMap<String, EvaluatedValue> {
        let instance = match instances.get(&group_id) {
            Some(inst) => inst,
            None => return HashMap::new(),
        };

        let effective_time = value_time(current_time, instance);
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id {
                    if let Some(val) =
                        evaluate_segments(entry, effective_time, &mut self.intern_pool)
                    {
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
        &mut self,
        group_id: u64,
        current_time: f64,
        instances: &BTreeMap<u64, StoryboardInstance>,
    ) -> HashMap<String, EvaluatedValue> {
        let instance = match instances.get(&group_id) {
            Some(inst) => inst,
            None => return HashMap::new(),
        };

        let effective_time = value_time(current_time, instance);
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id {
                    // アクティブなセグメントを特定
                    if let Some(active_seg) = find_active_segment(&entry.segments, effective_time) {
                        // アクティブセグメントの最終値 (progress_t=1.0) を取得
                        let val = entry.value_at(active_seg, 1.0, &mut self.intern_pool);
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
    // pause_start は Paused の間だけある（pause が遷移と同時に入れ、resume が取る）。あればそこで時計を止める
    let now = instance.pause_start.unwrap_or(current_time);
    let raw_time = now - instance.loop_start_time - instance.pause_accumulated;
    // time_scale は再生速度倍率（1.0 倍速が標準、2.0 で 2 倍速）
    // time_scale が大きいほどタイムラインが速く進む
    raw_time * instance.time_scale
}

/// 値を出す実効時刻。周回の後の待ち（次の起点の前＝負）は、前の周回の最後の値を保つ。
fn value_time(current_time: f64, instance: &StoryboardInstance) -> f64 {
    let t = calculate_effective_time(current_time, instance);
    if instance.loops_completed > 0 && t < 0.0 {
        f64::INFINITY
    } else {
        t
    }
}

/// セグメント列から現在値を評価する。
///
/// セグメントが無いときだけ `None`。最後のセグメントを過ぎたら最後の値を返す。
fn evaluate_segments(
    entry: &TimelineEntry,
    effective_time: f64,
    intern_pool: &mut ObjectInternPool,
) -> Option<EvaluatedValue> {
    let segments = &entry.segments;
    let last_seg = segments.last()?;

    // アクティブなセグメントを探す
    for seg in segments {
        if effective_time < seg.start_time {
            // まだこのセグメントに到達していない → from_value を返す
            return Some(entry.value_at(seg, 0.0, intern_pool));
        }

        let duration = seg.end_time - seg.start_time;
        if duration <= 0.0 {
            // 即時遷移（duration=0）
            if effective_time >= seg.start_time {
                return Some(entry.value_at(seg, 1.0, intern_pool));
            }
        } else if effective_time < seg.end_time {
            // このセグメント内
            let progress_t = (effective_time - seg.start_time) / duration;
            return Some(entry.value_at(seg, progress_t, intern_pool));
        }
        // seg.end_time <= effective_time → 次のセグメントへ
    }

    // 最後のセグメントを過ぎた → 最後の値を保つ
    Some(entry.value_at(last_seg, 1.0, intern_pool))
}

/// effective_time 時点でアクティブなセグメントを検索する。
/// Conclude 戦略の collect_current_segment_final_values で使用。
/// 未開始セグメントはスキップし、アクティブなセグメントを返す。
fn find_active_segment(
    segments: &[CompiledSegment],
    effective_time: f64,
) -> Option<&CompiledSegment> {
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
mod tests;
