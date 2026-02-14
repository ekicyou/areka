//! # Storyboard Compiler
//!
//! ストーリーボードの宣言的定義を、ランタイムが直接消費可能な
//! 「コンパイル済みトランジション」データ構造にコンパイルする。

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::document::DolaDocument;
use crate::easing::EasingFunction;
use crate::error::DolaError;
use crate::storyboard::{InterruptionPolicy, KeyframeNames, KeyframeRef, StoryboardEntry};
use crate::transition::{TransitionRef, TransitionValue};
use crate::validate::{Validate, collect_keyframe_names_from_ref};
use crate::variable::AnimationVariableDef;

// ============================================================
// Compiled Data Structures (Task 1)
// ============================================================

/// コンパイル済みストーリーボードのルート構造体
///
/// 変数名をキーとしたタイムラインマップと、ランタイム用メタ情報を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledStoryboard {
    /// 元のストーリーボード名
    pub storyboard_name: String,
    /// コンパイル起点の開始時刻（f64秒）
    pub start_time: f64,
    /// 変数名 → コンパイル済みタイムライン
    pub timelines: BTreeMap<String, CompiledVariableTimeline>,
    /// 再生速度倍率（ランタイム適用、事前適用なし）
    pub time_scale: f64,
    /// ループ回数 None=なし, Some(0)=無限, Some(n)=n回
    pub loop_count: Option<u32>,
    /// 割り込み終了戦略
    pub interruption_policy: InterruptionPolicy,
    /// ベース合計再生時間 time_scale未適用 全タイムラインの最大値
    pub total_base_duration: f64,
}

/// 変数ごとのセグメント列とランタイムヒント
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledVariableTimeline {
    /// 変数型情報
    pub variable_type: VariableTypeHint,
    /// セグメント配列（時刻順ソート済み、重複なし）
    pub segments: Vec<CompiledSegment>,
    /// このタイムラインのベース再生時間（最終セグメント end_time - start_time）
    pub base_duration: f64,
    /// 値域下限 f64/i64のみ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    /// 値域上限 f64/i64のみ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
}

/// 単一トランジションセグメントの全情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledSegment {
    /// セグメント開始時刻（絶対時刻、f64秒）
    pub start_time: f64,
    /// セグメント終了時刻（絶対時刻、f64秒）
    /// 即時遷移の場合は start_time と等しい
    pub end_time: f64,
    /// 開始値
    pub from_value: TransitionValue,
    /// 終了値
    pub to_value: TransitionValue,
    /// イージング関数
    /// None = 線形補間 または Object型即時切り替え
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easing: Option<EasingFunction>,
}

/// ランタイムに変数型固有の処理方法を伝達する enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VariableTypeHint {
    /// f64連続値（補間対応）
    Float,
    /// i64離散値（補間後の丸め処理が必要）
    Integer {
        /// タイプライター文字列
        #[serde(default, skip_serializing_if = "Option::is_none")]
        typewriter: Option<String>,
    },
    /// Object型（即時切り替えのみ）
    Object,
}

// ============================================================
// Compiler Implementation (Tasks 3-7)
// ============================================================

/// ストーリーボードをコンパイルする
///
/// # Preconditions
/// - doc は整形式の DolaDocument（内部で validate() を実行するため、
///   呼び出し側の事前バリデーションは不要）
/// - storyboard_name は doc.storyboard に存在する名前
/// - start_time >= 0.0
///
/// # Postconditions
/// - 成功時: CompiledStoryboard 内の全セグメントは絶対時刻を持ち、
///   各タイムラインのセグメントは時刻順ソート済みで重複なし
/// - 失敗時: Vec<DolaError> にすべてのエラーを収集して返却
///
/// # Invariants
/// - time_scale はセグメント時刻に事前適用されない
/// - Object型セグメントの easing は常に None
pub fn compile_storyboard(
    doc: &DolaDocument,
    storyboard_name: &str,
    start_time: f64,
) -> Result<CompiledStoryboard, Vec<DolaError>> {
    // Step 1: バリデーション (Task 3.1)
    doc.validate()?;

    // Step 2: ストーリーボード検索
    let sb = doc.storyboard.get(storyboard_name).ok_or_else(|| {
        vec![DolaError::CompileError {
            storyboard: storyboard_name.to_string(),
            entry_index: 0,
            reason: format!("Storyboard '{}' not found", storyboard_name),
        }]
    })?;

    let mut errors: Vec<DolaError> = Vec::new();

    // Step 3: 依存グラフ構築 & 循環検出 & トポソート (Task 4)
    let graph = build_dependency_graph(storyboard_name, sb, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    let sorted_indices = match topological_sort(&graph, sb.entry.len()) {
        Ok(order) => order,
        Err(cycle) => {
            errors.push(DolaError::KeyframeCycle {
                storyboard: storyboard_name.to_string(),
                cycle,
            });
            return Err(errors);
        }
    };

    // Step 4: エントリ処理 (Task 5)
    // keyframe_name → resolved time
    let mut keyframe_times: HashMap<String, f64> = HashMap::new();
    keyframe_times.insert("start".to_string(), start_time);

    // variable_name → last segment end_time
    let mut var_last_end_time: HashMap<String, f64> = HashMap::new();
    // variable_name → last segment to_value
    let mut var_last_value: HashMap<String, TransitionValue> = HashMap::new();
    // variable_name → Vec<CompiledSegment>
    let mut var_segments: HashMap<String, Vec<CompiledSegment>> = HashMap::new();
    // entry_index → keyframe_time (for pure KF "at なし" fallback)
    let mut entry_keyframe_time: HashMap<usize, f64> = HashMap::new();

    for &entry_idx in &sorted_indices {
        let entry = &sb.entry[entry_idx];
        let kf_name = entry
            .keyframe
            .clone()
            .unwrap_or_else(|| format!("__implicit_{}", entry_idx));

        // Determine if this is a pure keyframe entry (no variable/transition)
        let is_pure_kf = entry.variable.is_none() && entry.transition.is_none();

        if is_pure_kf {
            // Pure Keyframe (Task 5.4)
            let kf_time = resolve_pure_keyframe_time(
                storyboard_name,
                entry_idx,
                entry,
                &keyframe_times,
                &entry_keyframe_time,
                &sorted_indices,
                &mut errors,
            );
            if let Some(t) = kf_time {
                keyframe_times.insert(kf_name, t);
                entry_keyframe_time.insert(entry_idx, t);
            }
            continue;
        }

        // Transition entry: resolve transition def
        let var_name = match &entry.variable {
            Some(v) => v.clone(),
            None => continue, // validated already: transition requires variable
        };

        let trans_def =
            match resolve_transition(storyboard_name, entry_idx, entry, doc, &mut errors) {
                Some(td) => td,
                None => continue,
            };

        let var_def = match doc.variable.get(&var_name) {
            Some(vd) => vd,
            None => continue, // already caught by validation
        };

        // Resolve timing (Task 5.1-5.3)
        let timing = resolve_entry_timing(
            storyboard_name,
            entry_idx,
            entry,
            &trans_def,
            start_time,
            &keyframe_times,
            var_last_end_time.get(&var_name).copied(),
            &mut errors,
        );

        let (segment_start, segment_end, kf_time) = match timing {
            Some(t) => t,
            None => continue,
        };

        // Register keyframe time
        keyframe_times.insert(kf_name, kf_time);
        entry_keyframe_time.insert(entry_idx, kf_time);

        // Resolve from value (Task 5.7)
        let from_value = resolve_from_value(&trans_def, var_last_value.get(&var_name), var_def);

        // Resolve to value (Task 5.8)
        let to_value = resolve_to_value(&trans_def, &from_value);

        // Build segment (Task 5.9)
        let is_object = matches!(var_def, AnimationVariableDef::Object { .. });
        let easing = if is_object {
            None
        } else {
            trans_def.easing.clone()
        };

        let segment = CompiledSegment {
            start_time: segment_start,
            end_time: segment_end,
            from_value: from_value.clone(),
            to_value: to_value.clone(),
            easing,
        };

        var_segments
            .entry(var_name.clone())
            .or_default()
            .push(segment);
        var_last_end_time.insert(var_name.clone(), segment_end);
        var_last_value.insert(var_name, to_value);
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Step 5: Finalization (Task 6)
    let mut timelines: BTreeMap<String, CompiledVariableTimeline> = BTreeMap::new();

    for (var_name, mut segments) in var_segments {
        // Sort by start_time (Task 6.1)
        segments.sort_by(|a, b| {
            a.start_time
                .partial_cmp(&b.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Overlap check (Task 6.2)
        for i in 1..segments.len() {
            if segments[i - 1].end_time > segments[i].start_time {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: 0, // approximate
                    reason: format!(
                        "Segment overlap for variable '{}': previous end_time {} > next start_time {}",
                        var_name, segments[i - 1].end_time, segments[i].start_time
                    ),
                });
            }
        }

        let var_def = doc.variable.get(&var_name);

        // Build CompiledVariableTimeline (Task 6.3)
        let base_duration = if segments.is_empty() {
            0.0
        } else {
            segments.last().unwrap().end_time - segments.first().unwrap().start_time
        };

        let variable_type = build_variable_type_hint(var_def);
        let (min_value, max_value) = extract_min_max(var_def);

        timelines.insert(
            var_name,
            CompiledVariableTimeline {
                variable_type,
                segments,
                base_duration,
                min_value,
                max_value,
            },
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Build CompiledStoryboard (Task 6.4)
    let total_base_duration = timelines
        .values()
        .map(|tl| tl.base_duration)
        .fold(0.0_f64, f64::max);

    Ok(CompiledStoryboard {
        storyboard_name: storyboard_name.to_string(),
        start_time,
        timelines,
        time_scale: sb.time_scale,
        loop_count: sb.loop_count,
        interruption_policy: sb.interruption_policy,
        total_base_duration,
    })
}

// ============================================================
// Internal helpers
// ============================================================

/// キーフレーム依存グラフ
/// adjacency[i] = set of entry indices that entry i depends on
struct DependencyGraph {
    /// entry_index → set of dependency entry indices
    deps: HashMap<usize, HashSet<usize>>,
    /// keyframe name → entry index that defines it
    kf_to_entry: HashMap<String, usize>,
}

/// Build dependency graph from storyboard entries (Task 4.1)
fn build_dependency_graph(
    _storyboard_name: &str,
    sb: &crate::storyboard::Storyboard,
    _errors: &mut Vec<DolaError>,
) -> DependencyGraph {
    let mut kf_to_entry: HashMap<String, usize> = HashMap::new();

    // First pass: map keyframe names to entry indices
    for (idx, entry) in sb.entry.iter().enumerate() {
        let kf_name = entry
            .keyframe
            .clone()
            .unwrap_or_else(|| format!("__implicit_{}", idx));
        kf_to_entry.insert(kf_name, idx);
    }

    let mut deps: HashMap<usize, HashSet<usize>> = HashMap::new();

    // Second pass: build dependency edges
    for (idx, entry) in sb.entry.iter().enumerate() {
        let entry_deps = deps.entry(idx).or_default();

        if let Some(ref kf_ref) = entry.at {
            // at-based: depends on referenced keyframes
            let names = collect_keyframe_names_from_ref(kf_ref);
            for name in names {
                if name == "start" {
                    continue; // pseudo-keyframe, always available
                }
                if let Some(&dep_idx) = kf_to_entry.get(&name) {
                    entry_deps.insert(dep_idx);
                }
            }
        } else if let Some(ref between) = entry.between {
            // between: depends on from and to keyframes
            if between.from != "start" {
                if let Some(&dep_idx) = kf_to_entry.get(&between.from) {
                    entry_deps.insert(dep_idx);
                }
            }
            if between.to != "start" {
                if let Some(&dep_idx) = kf_to_entry.get(&between.to) {
                    entry_deps.insert(dep_idx);
                }
            }
        }
        // Sequential entries: no explicit dependency in graph
        // (handled by variable-specific last_end_time tracking)
    }

    DependencyGraph { deps, kf_to_entry }
}

/// Topological sort using Kahn's algorithm (Task 4.2, 4.3)
/// Returns sorted entry indices or a cycle (Vec of keyframe names)
fn topological_sort(
    graph: &DependencyGraph,
    entry_count: usize,
) -> Result<Vec<usize>, Vec<String>> {
    let mut in_degree: HashMap<usize, usize> = HashMap::new();
    let mut reverse_deps: HashMap<usize, Vec<usize>> = HashMap::new();

    for idx in 0..entry_count {
        in_degree.entry(idx).or_insert(0);
    }

    for (idx, dep_set) in &graph.deps {
        in_degree.entry(*idx).or_insert(0);
        for &dep in dep_set {
            *in_degree.entry(*idx).or_insert(0) += 0; // ensure entry exists
            reverse_deps.entry(dep).or_default().push(*idx);
        }
    }

    // Recompute in_degree properly
    for idx in 0..entry_count {
        let dep_count = graph.deps.get(&idx).map_or(0, |s| s.len());
        in_degree.insert(idx, dep_count);
    }

    let mut queue: Vec<usize> = Vec::new();
    for idx in 0..entry_count {
        if in_degree[&idx] == 0 {
            queue.push(idx);
        }
    }
    // Sort descending so pop() gives smallest index first
    queue.sort_by(|a, b| b.cmp(a));

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node);
        if let Some(dependents) = reverse_deps.get(&node) {
            for &dep in dependents {
                let deg = in_degree.get_mut(&dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(dep);
                    queue.sort_by(|a, b| b.cmp(a));
                }
            }
        }
    }

    if result.len() != entry_count {
        // Cycle detected: find cycle members
        let cycle_members: Vec<String> = (0..entry_count)
            .filter(|idx| in_degree.get(idx).copied().unwrap_or(0) > 0)
            .map(|idx| {
                // Try to find keyframe name for this entry
                graph
                    .kf_to_entry
                    .iter()
                    .find(|&(_, &v)| v == idx)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| format!("entry_{}", idx))
            })
            .collect();
        Err(cycle_members)
    } else {
        Ok(result)
    }
}

/// Resolve a pure keyframe entry's time (Task 5.4)
fn resolve_pure_keyframe_time(
    storyboard_name: &str,
    entry_idx: usize,
    entry: &StoryboardEntry,
    keyframe_times: &HashMap<String, f64>,
    entry_keyframe_time: &HashMap<usize, f64>,
    sorted_indices: &[usize],
    errors: &mut Vec<DolaError>,
) -> Option<f64> {
    if let Some(ref kf_ref) = entry.at {
        // at ベースの純粋KF
        let time = resolve_keyframe_ref_time(kf_ref, keyframe_times);
        match time {
            Some(t) => Some(t),
            None => {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Cannot resolve keyframe reference time".to_string(),
                });
                None
            }
        }
    } else {
        // at なし: 配列直前エントリの keyframe_time を継承
        let prev_entry_idx = find_previous_entry_in_sort_order(entry_idx, sorted_indices);
        match prev_entry_idx {
            Some(prev_idx) => {
                if let Some(&t) = entry_keyframe_time.get(&prev_idx) {
                    Some(t)
                } else {
                    errors.push(DolaError::CompileError {
                        storyboard: storyboard_name.to_string(),
                        entry_index: entry_idx,
                        reason:
                            "Pure keyframe without 'at': no previous entry keyframe time available"
                                .to_string(),
                    });
                    None
                }
            }
            None => {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Pure keyframe without 'at': no previous entry in array".to_string(),
                });
                None
            }
        }
    }
}

/// Find the previous entry (by original array index) in sorted order
fn find_previous_entry_in_sort_order(entry_idx: usize, _sorted_indices: &[usize]) -> Option<usize> {
    // "配列直前エントリ" = original array index - 1
    if entry_idx > 0 {
        Some(entry_idx - 1)
    } else {
        None
    }
}

/// Resolve a KeyframeRef to a time value (Task 5.5)
fn resolve_keyframe_ref_time(
    kf_ref: &KeyframeRef,
    keyframe_times: &HashMap<String, f64>,
) -> Option<f64> {
    match kf_ref {
        KeyframeRef::Single(name) => keyframe_times.get(name).copied(),
        KeyframeRef::Multiple(names) => {
            // All KFs must be resolved; take the latest
            let mut max_time: Option<f64> = None;
            for name in names {
                match keyframe_times.get(name) {
                    Some(&t) => {
                        max_time = Some(max_time.map_or(t, |m: f64| m.max(t)));
                    }
                    None => return None,
                }
            }
            max_time
        }
        KeyframeRef::WithOffset { keyframes, offset } => {
            let base_time = match keyframes {
                KeyframeNames::Single(name) => keyframe_times.get(name).copied(),
                KeyframeNames::Multiple(names) => {
                    let mut max_time: Option<f64> = None;
                    for name in names {
                        match keyframe_times.get(name) {
                            Some(&t) => {
                                max_time = Some(max_time.map_or(t, |m: f64| m.max(t)));
                            }
                            None => return None,
                        }
                    }
                    max_time
                }
            };
            base_time.map(|t| t + offset)
        }
    }
}

/// Resolve transition definition from entry (Task 5.6)
fn resolve_transition(
    storyboard_name: &str,
    entry_idx: usize,
    entry: &StoryboardEntry,
    doc: &DolaDocument,
    errors: &mut Vec<DolaError>,
) -> Option<crate::transition::TransitionDef> {
    match &entry.transition {
        Some(TransitionRef::Inline(def)) => Some(def.clone()),
        Some(TransitionRef::Named(name)) => match doc.transition.get(name) {
            Some(def) => Some(def.clone()),
            None => {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: entry_idx,
                    reason: format!("Named transition '{}' not found", name),
                });
                None
            }
        },
        None => {
            // No transition, shouldn't happen for non-pure-KF entry
            errors.push(DolaError::CompileError {
                storyboard: storyboard_name.to_string(),
                entry_index: entry_idx,
                reason: "Entry has variable but no transition".to_string(),
            });
            None
        }
    }
}

/// Resolve entry timing (Tasks 5.1-5.3)
/// Returns (segment_start, segment_end, keyframe_time)
fn resolve_entry_timing(
    storyboard_name: &str,
    entry_idx: usize,
    entry: &StoryboardEntry,
    trans_def: &crate::transition::TransitionDef,
    start_time: f64,
    keyframe_times: &HashMap<String, f64>,
    var_last_end_time: Option<f64>,
    errors: &mut Vec<DolaError>,
) -> Option<(f64, f64, f64)> {
    let delay = trans_def.delay;
    let duration = trans_def.duration.unwrap_or(0.0);

    if let Some(ref between) = entry.between {
        // Between placement (Task 5.3)
        let from_time = keyframe_times.get(&between.from).copied();
        let to_time = keyframe_times.get(&between.to).copied();

        match (from_time, to_time) {
            (Some(from_t), Some(to_t)) => {
                let segment_start = from_t + delay;
                let segment_end = to_t;

                if segment_start >= segment_end {
                    errors.push(DolaError::CompileError {
                        storyboard: storyboard_name.to_string(),
                        entry_index: entry_idx,
                        reason: format!(
                            "Between delay {} exceeds or equals interval ({} to {})",
                            delay, from_t, to_t
                        ),
                    });
                    return None;
                }

                let kf_time = segment_end;
                Some((segment_start, segment_end, kf_time))
            }
            _ => {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Cannot resolve between keyframe times".to_string(),
                });
                None
            }
        }
    } else if let Some(ref kf_ref) = entry.at {
        // At placement (Task 5.2)
        match resolve_keyframe_ref_time(kf_ref, keyframe_times) {
            Some(base_time) => {
                let segment_start = base_time + delay;
                let segment_end = segment_start + duration;
                let kf_time = segment_end;
                Some((segment_start, segment_end, kf_time))
            }
            None => {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Cannot resolve 'at' keyframe reference time".to_string(),
                });
                None
            }
        }
    } else {
        // Sequential placement (Task 5.1)
        let base_time = var_last_end_time.unwrap_or(start_time);
        let segment_start = base_time + delay;
        let segment_end = segment_start + duration;
        let kf_time = segment_end;
        Some((segment_start, segment_end, kf_time))
    }
}

/// Resolve from value (Task 5.7)
fn resolve_from_value(
    trans_def: &crate::transition::TransitionDef,
    last_value: Option<&TransitionValue>,
    var_def: &AnimationVariableDef,
) -> TransitionValue {
    if let Some(ref from) = trans_def.from {
        return from.clone();
    }

    // Infer from last segment end value or variable initial
    if let Some(last_val) = last_value {
        return last_val.clone();
    }

    // Use variable initial value
    match var_def {
        AnimationVariableDef::Float { initial, .. } => TransitionValue::Scalar(*initial),
        AnimationVariableDef::Integer { initial, .. } => TransitionValue::Scalar(*initial as f64),
        AnimationVariableDef::Object { initial } => TransitionValue::Dynamic(initial.clone()),
    }
}

/// Resolve to value (Task 5.8)
fn resolve_to_value(
    trans_def: &crate::transition::TransitionDef,
    from_value: &TransitionValue,
) -> TransitionValue {
    if let Some(ref relative_to) = trans_def.relative_to {
        // relative_to: from + offset
        if let TransitionValue::Scalar(from_val) = from_value {
            return TransitionValue::Scalar(from_val + relative_to);
        }
    }

    if let Some(ref to) = trans_def.to {
        return to.clone();
    }

    // If neither to nor relative_to specified, from value is kept (instant)
    from_value.clone()
}

/// Build VariableTypeHint from variable definition (Task 6.3)
fn build_variable_type_hint(var_def: Option<&AnimationVariableDef>) -> VariableTypeHint {
    match var_def {
        Some(AnimationVariableDef::Float { .. }) => VariableTypeHint::Float,
        Some(AnimationVariableDef::Integer { typewriter, .. }) => VariableTypeHint::Integer {
            typewriter: typewriter.clone(),
        },
        Some(AnimationVariableDef::Object { .. }) => VariableTypeHint::Object,
        None => VariableTypeHint::Float, // fallback
    }
}

/// Extract min/max from variable definition (Task 6.3)
fn extract_min_max(var_def: Option<&AnimationVariableDef>) -> (Option<f64>, Option<f64>) {
    match var_def {
        Some(AnimationVariableDef::Float { min, max, .. }) => (*min, *max),
        Some(AnimationVariableDef::Integer { min, max, .. }) => {
            (min.map(|v| v as f64), max.map(|v| v as f64))
        }
        _ => (None, None),
    }
}
