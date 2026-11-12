//! 依存グラフ構築・トポロジカルソート・タイミング解決ヘルパー
//!
//! compile_storyboard() から呼び出される内部関数群。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::storyboard::{KeyframeNames, KeyframeRef, StoryboardEntry};
use crate::transition::{TransitionDef, TransitionRef, TransitionValue};
use crate::validate::{IMPLICIT_KEYFRAME_PREFIX, collect_keyframe_names_from_ref};
use crate::variable::AnimationVariableDef;

use super::types::VariableTypeHint;

/// エントリのキーフレーム名を返す（未指定時は内部用の暗黙名 `__implicit_{idx}`）
///
/// 明示名が暗黙名とぶつからないよう、検証（V3）が `__implicit_` で始まる名を拒む。
pub(super) fn entry_keyframe_name(entry: &StoryboardEntry, idx: usize) -> String {
    entry
        .keyframe
        .clone()
        .unwrap_or_else(|| format!("{IMPLICIT_KEYFRAME_PREFIX}{idx}"))
}

/// キーフレーム依存グラフ
/// adjacency[i] = set of entry indices that entry i depends on
pub(super) struct DependencyGraph {
    /// entry_index → set of dependency entry indices
    pub(super) deps: HashMap<usize, HashSet<usize>>,
    /// keyframe name → entry index that defines it
    pub(super) kf_to_entry: HashMap<String, usize>,
    /// 連結の遷移の entry index → 同じ変数の配列上の直前のエントリの index
    pub(super) prev_same_var: HashMap<usize, usize>,
}

/// Build dependency graph from storyboard entries (Task 4.1)
pub(super) fn build_dependency_graph(sb: &crate::storyboard::Storyboard) -> DependencyGraph {
    let mut kf_to_entry: HashMap<String, usize> = HashMap::new();

    // First pass: map keyframe names to entry indices
    for (idx, entry) in sb.entry.iter().enumerate() {
        kf_to_entry.insert(entry_keyframe_name(entry, idx), idx);
    }

    let mut deps: HashMap<usize, HashSet<usize>> = HashMap::new();
    let mut prev_same_var: HashMap<usize, usize> = HashMap::new();

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
            for name in [&between.from, &between.to] {
                if name == "start" {
                    continue; // pseudo-keyframe, always available
                }
                if let Some(&dep_idx) = kf_to_entry.get(name) {
                    entry_deps.insert(dep_idx);
                }
            }
        } else if let Some(ref var) = entry.variable {
            // 連結の遷移: 同じ変数の配列上の直前のエントリに続く（要件 11.7）
            if let Some(prev) = (0..idx)
                .rev()
                .find(|&i| sb.entry[i].variable.as_ref() == Some(var))
            {
                entry_deps.insert(prev);
                prev_same_var.insert(idx, prev);
            }
        } else if let Some(prev) = idx.checked_sub(1) {
            // at の無い純粋なキーフレーム・トリガー: 配列の 1 つ前のエントリに続く（要件 11.7）
            entry_deps.insert(prev);
        }
    }

    DependencyGraph {
        deps,
        kf_to_entry,
        prev_same_var,
    }
}

/// Topological sort using Kahn's algorithm (Task 4.2, 4.3)
/// Returns sorted entry indices or a cycle (Vec of keyframe names)
pub(super) fn topological_sort(
    graph: &DependencyGraph,
    entry_count: usize,
) -> Result<Vec<usize>, Vec<String>> {
    let mut in_degree: Vec<usize> = (0..entry_count)
        .map(|idx| graph.deps.get(&idx).map_or(0, |s| s.len()))
        .collect();

    // SAFETY: graph は build_dependency_graph で同一 storyboard から構築され、
    // deps のキー・値はいずれも enumerate 由来のエントリ index（< entry_count）。
    // したがって reverse_deps[dep] の添字アクセスは範囲内（呼び出し契約を表明）。
    let mut reverse_deps: Vec<Vec<usize>> = vec![Vec::new(); entry_count];
    for (&idx, dep_set) in &graph.deps {
        for &dep in dep_set {
            debug_assert!(
                dep < entry_count && idx < entry_count,
                "dependency graph indices must be < entry_count"
            );
            reverse_deps[dep].push(idx);
        }
    }

    // Min-heap so pop() gives smallest index first (deterministic order)
    let mut queue: BinaryHeap<Reverse<usize>> = (0..entry_count)
        .filter(|&idx| in_degree[idx] == 0)
        .map(Reverse)
        .collect();

    let mut result = Vec::new();

    // NOTE(D2-V): Kahn 法による反復実装（再帰なし）。エントリ数に比例した
    // ヒープ確保のみで、巨大文書でもスタック枯渇しない。
    while let Some(Reverse(node)) = queue.pop() {
        result.push(node);
        for &dependent in &reverse_deps[node] {
            // SAFETY: in_degree[dependent] は dependent への入次数（deps は HashSet で
            // 重複エッジなし）に初期化され、各エッジにつき本行で1回だけ減算されるため
            // 0 を下回らない（usize アンダーフローは発生しない）。
            debug_assert!(in_degree[dependent] > 0, "in_degree underflow");
            in_degree[dependent] -= 1;
            if in_degree[dependent] == 0 {
                queue.push(Reverse(dependent));
            }
        }
    }

    if result.len() != entry_count {
        // 残った節はどれも残った節に依存するので、依存をたどれば閉路に入る。
        // 閉路の先にあるだけの節を出さないよう、1 本たどって閉路の分だけを残す
        let mut path: Vec<usize> = Vec::new();
        let mut node = (0..entry_count).find(|&idx| in_degree[idx] > 0);
        while let Some(n) = node {
            if let Some(pos) = path.iter().position(|&p| p == n) {
                path.drain(..pos);
                break;
            }
            path.push(n);
            node = graph
                .deps
                .get(&n)
                .and_then(|d| d.iter().copied().filter(|&i| in_degree[i] > 0).min());
        }
        let cycle_members: Vec<String> = path
            .into_iter()
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
pub(super) fn resolve_pure_keyframe_time(
    storyboard_name: &str,
    entry_idx: usize,
    entry: &StoryboardEntry,
    start_time: f64,
    keyframe_times: &HashMap<String, f64>,
    entry_keyframe_time: &HashMap<usize, f64>,
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
    } else if let Some(ref between) = entry.between {
        // between の純粋KF・トリガーは between の始まりの時刻に置く（要件 11.10）
        let time = keyframe_times.get(&between.from).copied();
        if time.is_none() {
            errors.push(DolaError::CompileError {
                storyboard: storyboard_name.to_string(),
                entry_index: entry_idx,
                reason: "Cannot resolve between keyframe times".to_string(),
            });
        }
        time
    } else {
        // at なし: 配列直前エントリ（元配列 index - 1）の keyframe_time を継承。先頭は start_time
        let Some(prev_idx) = entry_idx.checked_sub(1) else {
            return Some(start_time);
        };
        let time = entry_keyframe_time.get(&prev_idx).copied();
        if time.is_none() {
            // 直前のエントリが解けなかったときの連鎖
            errors.push(DolaError::CompileError {
                storyboard: storyboard_name.to_string(),
                entry_index: entry_idx,
                reason: "Pure keyframe without 'at': no previous entry keyframe time available"
                    .to_string(),
            });
        }
        time
    }
}

/// All KFs must be resolved; take the latest
/// （1つでも未解決、または names が空なら None）
fn latest_keyframe_time(names: &[String], keyframe_times: &HashMap<String, f64>) -> Option<f64> {
    let mut max_time: Option<f64> = None;
    for name in names {
        let t = *keyframe_times.get(name)?;
        max_time = Some(max_time.map_or(t, |m: f64| m.max(t)));
    }
    max_time
}

/// Resolve a KeyframeRef to a time value (Task 5.5)
pub(super) fn resolve_keyframe_ref_time(
    kf_ref: &KeyframeRef,
    keyframe_times: &HashMap<String, f64>,
) -> Option<f64> {
    match kf_ref {
        KeyframeRef::Single(name) => keyframe_times.get(name).copied(),
        KeyframeRef::Multiple(names) => latest_keyframe_time(names, keyframe_times),
        KeyframeRef::WithOffset { keyframes, offset } => {
            let base_time = match keyframes {
                KeyframeNames::Single(name) => keyframe_times.get(name).copied(),
                KeyframeNames::Multiple(names) => latest_keyframe_time(names, keyframe_times),
            };
            base_time.map(|t| t + offset)
        }
    }
}

/// エントリの遷移の定義を借りる（Task 5.6）。遷移の有無は V7、名前の参照は V5 が先に確かめている
pub(super) fn resolve_transition<'a>(
    entry: &'a StoryboardEntry,
    doc: &'a DolaDocument,
) -> Option<&'a TransitionDef> {
    match entry.transition.as_ref()? {
        TransitionRef::Inline(def) => Some(def),
        TransitionRef::Named(name) => doc.transition.get(name),
    }
}

/// Resolve entry timing (Tasks 5.1-5.3)
/// Returns (segment_start, segment_end, keyframe_time)
///
/// delay / duration / offset の有限・符号は validate() の数値の規則が先に見ている。
#[expect(
    clippy::too_many_arguments,
    reason = "解決の途中の表を 1 か所で受ける私的な関数で、束の型を作ると同じ欄の写しが増える"
)]
pub(super) fn resolve_entry_timing(
    storyboard_name: &str,
    entry_idx: usize,
    entry: &StoryboardEntry,
    trans_def: &TransitionDef,
    start_time: f64,
    keyframe_times: &HashMap<String, f64>,
    prev_end_time: Option<f64>,
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
        let base_time = prev_end_time.unwrap_or(start_time);
        let segment_start = base_time + delay;
        let segment_end = segment_start + duration;
        let kf_time = segment_end;
        Some((segment_start, segment_end, kf_time))
    }
}

/// Resolve from value (Task 5.7)
pub(super) fn resolve_from_value(
    trans_def: &TransitionDef,
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
pub(super) fn resolve_to_value(
    trans_def: &TransitionDef,
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
pub(super) fn build_variable_type_hint(var_def: &AnimationVariableDef) -> VariableTypeHint {
    match var_def {
        AnimationVariableDef::Float { .. } => VariableTypeHint::Float,
        AnimationVariableDef::Integer { .. } => VariableTypeHint::Integer,
        AnimationVariableDef::Object { .. } => VariableTypeHint::Object,
    }
}

/// Extract min/max from variable definition (Task 6.3)
pub(super) fn extract_min_max(var_def: &AnimationVariableDef) -> (Option<f64>, Option<f64>) {
    match var_def {
        AnimationVariableDef::Float { min, max, .. } => (*min, *max),
        AnimationVariableDef::Integer { min, max, .. } => {
            (min.map(|v| v as f64), max.map(|v| v as f64))
        }
        AnimationVariableDef::Object { .. } => (None, None),
    }
}
