//! # Storyboard Compiler
//!
//! ストーリーボードの宣言的定義を、ランタイムが直接消費可能な
//! 「コンパイル済みトランジション」データ構造にコンパイルする。

mod resolve;
pub mod types;

pub use types::*;

use std::collections::{BTreeMap, HashMap};

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::transition::TransitionValue;
use crate::validate::Validate;
use crate::variable::AnimationVariableDef;

use resolve::*;

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
    // NOTE(D2-V): デシリアライズ境界に関する整理 —
    // doc は外部入力（JSON/TOML/YAML）または Builder API 由来だが、どの取り込み経路でも
    // 本関数冒頭で必ず validate() を通過する（Builder はバリデーションを迂回できない。
    // tests/compile/boundary_test.rs で特性化済み）。ネスト深度の制限（再帰的
    // デシリアライズによるスタック枯渇防御）は各フォーマットパーサ側の責務であり、
    // 本モジュールは再帰を持たない（トポロジカルソートは反復実装）。
    // ただし validate() 内のトリガー循環検出（validate/rules.rs::dfs_detect_cycle）は
    // 再帰 DFS のため、ストーリーボード連鎖長に比例したスタックを消費する（D3 境界、
    // D3-V への申し送り）。数値フィールドの有限性・符号は検証されない（P8/P14/P20 参照）。
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
    let graph = build_dependency_graph(sb);

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
    // compiled triggers
    let mut triggers: Vec<CompiledTrigger> = Vec::new();

    // SAFETY: topological_sort の成功時 result は 0..entry_count（= sb.entry.len()）の
    // 置換であり、全要素が添字範囲内であることを保証する（下の添字アクセスは panic 不能）。
    debug_assert!(sorted_indices.len() == sb.entry.len());
    debug_assert!(sorted_indices.iter().all(|&i| i < sb.entry.len()));

    for &entry_idx in &sorted_indices {
        let entry = &sb.entry[entry_idx];
        let kf_name = entry_keyframe_name(entry, entry_idx);

        if let Some(ref target_storyboard) = entry.trigger_storyboard {
            // Trigger entry: resolve fire_time using same logic as pure KF
            let fire_time = resolve_pure_keyframe_time(
                storyboard_name,
                entry_idx,
                entry,
                &keyframe_times,
                &entry_keyframe_time,
                &mut errors,
            );
            if let Some(t) = fire_time {
                // Register keyframe at fire_time (0-second completion)
                keyframe_times.insert(kf_name, t);
                entry_keyframe_time.insert(entry_idx, t);

                // Create CompiledTrigger
                triggers.push(CompiledTrigger {
                    fire_time: t,
                    target_storyboard: target_storyboard.clone(),
                    start_offset: entry.trigger_start_offset,
                    source_entry_index: entry_idx,
                });
            }
            continue;
        }

        // Pure keyframe entry (no variable/transition; trigger は上で処理済み)
        if entry.variable.is_none() && entry.transition.is_none() {
            // Pure Keyframe (Task 5.4)
            let kf_time = resolve_pure_keyframe_time(
                storyboard_name,
                entry_idx,
                entry,
                &keyframe_times,
                &entry_keyframe_time,
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
        // NOTE(D2-V): NaN 時刻は partial_cmp が None → Equal 扱いとなり panic しないが、
        // ソート順は不定となる。後続の重複検査も NaN 比較（常に false）で素通りするため、
        // NaN 時刻のセグメントは重複検出されずに出力へ伝播する（有限性検証は P14 参照。
        // tests/compile/boundary_test.rs で特性化済み）。
        segments.sort_by(|a, b| {
            a.start_time
                .partial_cmp(&b.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Overlap check (Task 6.2)
        // NOTE(D2-V): 負の duration による反転セグメント（end < start）は、単独または
        // 近接しない配置では本検査に掛からず出力へ伝播する（P20 参照）。
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
        // SAFETY: is_empty() ガードにより last()/first() の unwrap は panic 不能。
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
    // NOTE(D2-V): f64::max は NaN を無視するため、NaN の base_duration（NaN 時刻
    // セグメント由来）は total_base_duration へ伝播せず黙って脱落する。
    // 負の base_duration（反転セグメント由来）も初期値 0.0 でマスクされる（P14/P20 参照）。
    let total_base_duration = timelines
        .values()
        .map(|tl| tl.base_duration)
        .fold(0.0_f64, f64::max);

    // Sort triggers by fire_time (stable sort preserves entry index order for same fire_time)
    triggers.sort_by(|a, b| {
        a.fire_time
            .partial_cmp(&b.fire_time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.source_entry_index.cmp(&b.source_entry_index))
    });

    Ok(CompiledStoryboard {
        storyboard_name: storyboard_name.to_string(),
        start_time,
        timelines,
        time_scale: sb.time_scale,
        loop_count: sb.loop_count,
        interruption_policy: sb.interruption_policy,
        loop_offset: sb.loop_offset.clone(),
        total_base_duration,
        triggers,
    })
}
