//! 購読変数ごとのタイムテーブル管理と時刻ベース評価。

use std::collections::{BTreeMap, HashMap};

use crate::compile::{CompiledSegment, CompiledStoryboard, VariableTypeHint};
use crate::time::{TimeDelta, TimePoint};

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
    /// - 始まった項目（実効時刻が最初のセグメントの始まり以降）のうち最新（最大）group_id を優先する。
    ///   始まっていない項目（未来の start・start_offset の子）は、全部が始まっていないときだけ from を出す
    /// - 最後のセグメントを過ぎた変数は、インスタンスが生きている間は最後の値を保つ
    ///   （項目を消すのは `remove_entries` だけ）
    /// - Pause 中のインスタンスは pause 開始時点の台本の時刻で固定
    pub fn evaluate(
        &mut self,
        variable_name: &str,
        now: TimePoint,
        instances: &BTreeMap<u64, StoryboardInstance>,
    ) -> Option<EvaluatedValue> {
        let timeline = self.timelines.get_mut(variable_name)?;
        // インスタンスの無い項目（取り残し）だけを捨てる。生きた項目は remove_entries だけが消す
        timeline
            .entries
            .retain(|e| instances.contains_key(&e.group_id));

        // (始まったか, group_id) の大きい項目を取る
        let mut best_value: Option<((bool, u64), EvaluatedValue)> = None;

        for entry in &timeline.entries {
            let Some(instance) = instances.get(&entry.group_id) else {
                continue;
            };

            let effective_time = value_time(now, instance);
            // 周回を 1 回でも終えたインスタンスは始まった扱い（2 周目の先頭の遅れの中でも）
            let started = instance.loops_completed > 0
                || entry
                    .segments
                    .first()
                    .is_some_and(|s| match effective_time {
                        ValueTime::At(t) => t >= s.start_time,
                        ValueTime::AfterLastSegment => true,
                    });
            let key = (started, entry.group_id);

            if best_value.as_ref().is_some_and(|(best, _)| key <= *best) {
                continue;
            }
            if let Some(val) = evaluate_segments(entry, effective_time, &mut self.intern_pool) {
                best_value = Some((key, val));
            }
        }

        best_value.map(|(_, val)| val)
    }

    /// Conclude 用: group_id の全変数の最終値を取得。
    pub fn collect_final_values(&mut self, group_id: u64) -> HashMap<String, EvaluatedValue> {
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id
                    && let Some(last_seg) = entry.segments.last()
                {
                    let val = entry.value_at(last_seg, 1.0, &mut self.intern_pool);
                    result.insert(var_name.clone(), val);
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
    #[cfg(test)]
    pub fn has_entries(&self, group_id: u64) -> bool {
        self.timelines
            .values()
            .any(|tl| tl.entries.iter().any(|e| e.group_id == group_id))
    }

    /// 指定変数のタイムラインを取得（読み取り専用）。
    pub(crate) fn get_timeline(&self, variable_name: &str) -> Option<&VariableTimeline> {
        self.timelines.get(variable_name)
    }

    /// group_id に属する全変数の値を now 時点で評価する（Cancel/Trim 戦略用）。
    pub(crate) fn evaluate_all_for_group(
        &mut self,
        group_id: u64,
        now: TimePoint,
        instances: &BTreeMap<u64, StoryboardInstance>,
    ) -> HashMap<String, EvaluatedValue> {
        let instance = match instances.get(&group_id) {
            Some(inst) => inst,
            None => return HashMap::new(),
        };

        let effective_time = value_time(now, instance);
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id
                    && let Some(val) =
                        evaluate_segments(entry, effective_time, &mut self.intern_pool)
                {
                    result.insert(var_name.clone(), val);
                }
            }
        }

        result
    }

    /// 現在再生中セグメントの最終値（to_value）を group_id 単位で取得する（Conclude 戦略専用）。
    /// 未開始のセグメントはスキップする。
    pub(crate) fn collect_current_segment_final_values(
        &mut self,
        group_id: u64,
        now: TimePoint,
        instances: &BTreeMap<u64, StoryboardInstance>,
    ) -> HashMap<String, EvaluatedValue> {
        let instance = match instances.get(&group_id) {
            Some(inst) => inst,
            None => return HashMap::new(),
        };

        let effective_time = value_time(now, instance);
        let mut result = HashMap::new();

        for (var_name, timeline) in &self.timelines {
            for entry in &timeline.entries {
                if entry.group_id == group_id
                    && let Some(active_seg) = find_active_segment(&entry.segments, effective_time)
                {
                    let val = entry.value_at(active_seg, 1.0, &mut self.intern_pool);
                    result.insert(var_name.clone(), val);
                }
            }
        }

        result
    }
}

/// 台本の時刻（経った長さの前向きの写し）。Pause 中は pause 開始時点で固定。
pub(crate) fn calculate_effective_time(now: TimePoint, instance: &StoryboardInstance) -> TimeDelta {
    instance.time_scale.forward(instance.elapsed(now))
}

/// 値を出す台本の時刻。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueTime {
    At(TimeDelta),
    /// 周回の後の待ち（次の周回の始まりの前）。前の周回の最後の値を保つ
    AfterLastSegment,
}

fn value_time(now: TimePoint, instance: &StoryboardInstance) -> ValueTime {
    let t = calculate_effective_time(now, instance);
    if instance.loops_completed > 0 && t.is_negative() {
        ValueTime::AfterLastSegment
    } else {
        ValueTime::At(t)
    }
}

/// セグメント列から現在値を評価する。
///
/// セグメントが無いときだけ `None`。最後のセグメントを過ぎたら最後の値を返す。
fn evaluate_segments(
    entry: &TimelineEntry,
    effective_time: ValueTime,
    intern_pool: &mut ObjectInternPool,
) -> Option<EvaluatedValue> {
    let segments = &entry.segments;
    let last_seg = segments.last()?;
    let ValueTime::At(effective_time) = effective_time else {
        return Some(entry.value_at(last_seg, 1.0, intern_pool));
    };

    let mut prev: Option<&CompiledSegment> = None;
    for seg in segments {
        if effective_time < seg.start_time {
            // まだこのセグメントに到達していない。隙間なら直前の終わりの値を保ち、最初の前なら from
            return Some(match prev {
                Some(p) => entry.value_at(p, 1.0, intern_pool),
                None => entry.value_at(seg, 0.0, intern_pool),
            });
        }
        prev = Some(seg);

        if effective_time < seg.end_time
            && let Some(progress_t) =
                (effective_time - seg.start_time).ratio_of(seg.end_time - seg.start_time)
        {
            return Some(entry.value_at(seg, progress_t, intern_pool));
        }
    }

    Some(entry.value_at(last_seg, 1.0, intern_pool))
}

/// 台本の時刻でアクティブなセグメントを検索する。未開始セグメントはスキップする。
fn find_active_segment(
    segments: &[CompiledSegment],
    effective_time: ValueTime,
) -> Option<&CompiledSegment> {
    let ValueTime::At(effective_time) = effective_time else {
        return segments.last();
    };

    let mut active: Option<&CompiledSegment> = None;

    for seg in segments {
        if effective_time < seg.start_time {
            break;
        }

        if seg.end_time <= seg.start_time || effective_time >= seg.end_time {
            active = Some(seg);
        } else {
            return Some(seg);
        }
    }

    active
}

#[cfg(test)]
mod tests;
