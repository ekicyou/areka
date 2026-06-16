//! 依存グラフ構築・トポロジカルソート・タイミング解決ヘルパー
//!
//! compile_storyboard() から呼び出される内部関数群。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::storyboard::{KeyframeNames, KeyframeRef, StoryboardEntry};
use crate::transition::{TransitionRef, TransitionValue};
use crate::validate::collect_keyframe_names_from_ref;
use crate::variable::AnimationVariableDef;

use super::types::VariableTypeHint;

/// エントリのキーフレーム名を返す（未指定時は内部用の暗黙名 `__implicit_{idx}`）
///
/// NOTE(D2-V): バリデーションは `start` のみ予約し（V3）、`__implicit_` プレフィックスを
/// 予約していないため、ユーザーが明示的に `keyframe = "__implicit_{n}"` を指定すると
/// 別エントリの暗黙名と衝突し得る。衝突時は `kf_to_entry` / `keyframe_times` の
/// HashMap 上書き（後勝ち）により明示名が黙ってシャドウされ、誤った時刻解決となる
/// （panic はしない。tests/compile/boundary_test.rs で特性化、プレフィックス予約は
/// バリデーション追加＝挙動変更のため P21 提案を参照）。
pub(super) fn entry_keyframe_name(entry: &StoryboardEntry, idx: usize) -> String {
    entry
        .keyframe
        .clone()
        .unwrap_or_else(|| format!("__implicit_{}", idx))
}

/// キーフレーム依存グラフ
/// adjacency[i] = set of entry indices that entry i depends on
pub(super) struct DependencyGraph {
    /// entry_index → set of dependency entry indices
    pub(super) deps: HashMap<usize, HashSet<usize>>,
    /// keyframe name → entry index that defines it
    pub(super) kf_to_entry: HashMap<String, usize>,
}

/// Build dependency graph from storyboard entries (Task 4.1)
pub(super) fn build_dependency_graph(sb: &crate::storyboard::Storyboard) -> DependencyGraph {
    let mut kf_to_entry: HashMap<String, usize> = HashMap::new();

    // First pass: map keyframe names to entry indices
    for (idx, entry) in sb.entry.iter().enumerate() {
        kf_to_entry.insert(entry_keyframe_name(entry, idx), idx);
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
            for name in [&between.from, &between.to] {
                if name == "start" {
                    continue; // pseudo-keyframe, always available
                }
                if let Some(&dep_idx) = kf_to_entry.get(name) {
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
        // Cycle detected: find cycle members
        let cycle_members: Vec<String> = (0..entry_count)
            .filter(|&idx| in_degree[idx] > 0)
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
    } else {
        // at なし: 配列直前エントリ（元配列 index - 1）の keyframe_time を継承
        match entry_idx.checked_sub(1) {
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
                // 先頭エントリ: "start" キーフレーム（= start_time）をフォールバック
                if let Some(&t) = keyframe_times.get("start") {
                    Some(t)
                } else {
                    errors.push(DolaError::CompileError {
                        storyboard: storyboard_name.to_string(),
                        entry_index: entry_idx,
                        reason: "Pure keyframe without 'at': no previous entry in array"
                            .to_string(),
                    });
                    None
                }
            }
        }
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

/// Resolve transition definition from entry (Task 5.6)
pub(super) fn resolve_transition(
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
///
/// NOTE(D2-V): delay / duration / offset は指示書由来の任意 f64 で、有限性・符号の
/// 検証がない（P14/P20 参照）。f64 加算は panic しないが以下の静かな縮退がある:
/// - 負の duration → segment_end < segment_start の反転セグメントが生成される（P20）
/// - NaN の delay/duration → between の反転検査 `segment_start >= segment_end` が
///   NaN 比較 false で素通りし、NaN 時刻のセグメントが出力へ伝播する（P14）
/// - ±inf は inf 時刻として伝播する
///
/// 現行挙動は tests/compile/boundary_test.rs で特性化済み。
pub(super) fn resolve_entry_timing(
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
pub(super) fn resolve_from_value(
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
pub(super) fn resolve_to_value(
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
pub(super) fn build_variable_type_hint(var_def: Option<&AnimationVariableDef>) -> VariableTypeHint {
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
pub(super) fn extract_min_max(
    var_def: Option<&AnimationVariableDef>,
) -> (Option<f64>, Option<f64>) {
    match var_def {
        Some(AnimationVariableDef::Float { min, max, .. }) => (*min, *max),
        Some(AnimationVariableDef::Integer { min, max, .. }) => {
            (min.map(|v| v as f64), max.map(|v| v as f64))
        }
        _ => (None, None),
    }
}
