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
use crate::transition::{TransitionDef, TransitionValue};
use crate::validate::Validate;
use crate::variable::AnimationVariableDef;

use resolve::*;

/// 有限の数どうしの和があふれて inf になった時刻・値を `InvalidNumber` として積む
/// （validate の数値の規則は数を 1 つずつしか見ない）。
fn push_if_overflowed(
    errors: &mut Vec<DolaError>,
    storyboard: &str,
    entry_idx: usize,
    field: &str,
    value: f64,
) {
    if !value.is_finite() {
        errors.push(DolaError::InvalidNumber {
            location: format!("storyboard '{storyboard}' entry {entry_idx}"),
            field: field.to_string(),
            value,
        });
    }
}

/// 時刻の決まった、値を埋める前のセグメント
struct PendingSegment<'d> {
    entry_idx: usize,
    start_time: f64,
    end_time: f64,
    trans_def: &'d TransitionDef,
    /// at も between も無い連結の遷移か
    sequential: bool,
    /// 同じ変数の配列上の直前のエントリ
    prev_entry: Option<usize>,
}

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
    // doc は外部入力（JSON/TOML/YAML）か Builder の由来だが、どの経路でも compile_validated の前に
    // ここか compile_all で必ず validate() を通る（数値の規則を含む。tests/compile/boundary_test.rs が縛る）。
    // ネスト深度の制限は各フォーマットのパーサの責務。本モジュールも validate() の
    // トリガー循環の検出も再帰を持たない（どちらも明示のスタックか反復）。
    doc.validate()?;
    compile_validated(doc, storyboard_name, start_time)
}

/// 検証の後に全ストーリーボードをコンパイルにかける（読み込みと `DolaDocumentBuilder::build`）。
pub(crate) fn compile_all(doc: &DolaDocument) -> Result<(), Vec<DolaError>> {
    doc.validate()?;
    let errors: Vec<DolaError> = doc
        .storyboard
        .keys()
        .filter_map(|name| compile_validated(doc, name, 0.0).err())
        .flatten()
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 検査を省いてコンパイルする。`doc` は必ず `validate()` を通ったものだけを渡す
/// （下の `let Some(..) = resolved else { continue; }` が検査の欠けを release で黙って飛ばすため）。
pub(crate) fn compile_validated(
    doc: &DolaDocument,
    storyboard_name: &str,
    start_time: f64,
) -> Result<CompiledStoryboard, Vec<DolaError>> {
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

    // variable_name → (定義, 時刻の決まったセグメント)。名前の順で重なりを報告する。
    // 値（from/to）は時刻の順に並べた後の Step 5 で埋める
    type VarSegments<'d> = (&'d AnimationVariableDef, Vec<PendingSegment<'d>>);
    let mut var_segments: BTreeMap<&str, VarSegments> = BTreeMap::new();
    // entry_index → segment end_time（連結の遷移が配列上の直前のエントリに続くため）
    let mut entry_segment_end: HashMap<usize, f64> = HashMap::new();
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
                start_time,
                &keyframe_times,
                &entry_keyframe_time,
                &mut errors,
            );
            if let Some(t) = fire_time {
                push_if_overflowed(&mut errors, storyboard_name, entry_idx, "fire_time", t);
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
                start_time,
                &keyframe_times,
                &entry_keyframe_time,
                &mut errors,
            );
            if let Some(t) = kf_time {
                push_if_overflowed(&mut errors, storyboard_name, entry_idx, "keyframe_time", t);
                keyframe_times.insert(kf_name, t);
                entry_keyframe_time.insert(entry_idx, t);
            }
            continue;
        }

        // 遷移のエントリは変数・その定義・遷移の定義を持つ（V4・V5・V7 が先に落とす）
        let resolved = entry
            .variable
            .as_deref()
            .and_then(|v| Some((v, doc.variable.get(v)?, resolve_transition(entry, doc)?)));
        debug_assert!(
            resolved.is_some(),
            "validate() を通らない entry {entry_idx}"
        );
        let Some((var_name, var_def, trans_def)) = resolved else {
            continue;
        };

        // 連結の遷移は処理の順で最後に解いた分でなく、同じ変数の配列上の直前のエントリに続く（要件 11.7）
        let sequential = entry.at.is_none() && entry.between.is_none();
        let prev_entry = graph.prev_same_var.get(&entry_idx).copied();
        let last_end_time = prev_entry.and_then(|p| entry_segment_end.get(&p)).copied();

        // Resolve timing (Task 5.1-5.3)
        let timing = resolve_entry_timing(
            storyboard_name,
            entry_idx,
            entry,
            trans_def,
            start_time,
            &keyframe_times,
            last_end_time,
            &mut errors,
        );

        let (segment_start, segment_end, kf_time) = match timing {
            Some(t) => t,
            None => continue,
        };
        // 始まりは終わり以下なので、終わりが有限なら始まりも有限
        push_if_overflowed(
            &mut errors,
            storyboard_name,
            entry_idx,
            "end_time",
            segment_end,
        );

        // Register keyframe time
        keyframe_times.insert(kf_name, kf_time);
        entry_keyframe_time.insert(entry_idx, kf_time);

        var_segments
            .entry(var_name)
            .or_insert_with(|| (var_def, Vec::new()))
            .1
            .push(PendingSegment {
                entry_idx,
                start_time: segment_start,
                end_time: segment_end,
                trans_def,
                sequential,
                prev_entry,
            });
        entry_segment_end.insert(entry_idx, segment_end);
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Step 5: Finalization (Task 6)
    let mut timelines: BTreeMap<String, CompiledVariableTimeline> = BTreeMap::new();

    for (var_name, (var_def, mut segments)) in var_segments {
        // Sort by start_time (Task 6.1)
        // 比べは −0.0 を +0.0 にそろえた全順序（NaN は符号で両端に分かれる）。
        // 同じ始まりは配列の順（連結の直前のエントリは配列で先）
        segments.sort_by(|a, b| {
            (a.start_time + 0.0)
                .total_cmp(&(b.start_time + 0.0))
                .then(a.entry_idx.cmp(&b.entry_idx))
        });

        // from/to を時刻の順に埋める（Task 5.7・5.8）。from を省いたとき、連結の遷移は配列上の
        // 直前のエントリ（要件 11.7）、at・between は時刻の上で直前のセグメント（完了
        // dola-compiled-transition 要件 4.2）の to を取る。どちらもこの並びで先に埋まっている
        let mut entry_to: HashMap<usize, TransitionValue> = HashMap::new();
        let mut compiled: Vec<(usize, CompiledSegment)> = Vec::with_capacity(segments.len());
        for pending in &segments {
            let trans_def = pending.trans_def;
            let last_value = if pending.sequential {
                pending.prev_entry.and_then(|p| entry_to.get(&p))
            } else {
                compiled.last().map(|(_, s)| &s.to_value)
            };
            let from_value = resolve_from_value(trans_def, last_value, var_def);
            let to_value = resolve_to_value(trans_def, &from_value);
            let entry_idx = pending.entry_idx;

            // 値域の検証は書かれた from/to だけを見るので、relative_to の結果はここで比べる（要件 11.11）
            if let TransitionValue::Scalar(v) = to_value {
                let (min, max) = extract_min_max(var_def);
                let (min, max) = (
                    min.unwrap_or(f64::NEG_INFINITY),
                    max.unwrap_or(f64::INFINITY),
                );
                if !v.is_finite() {
                    push_if_overflowed(&mut errors, storyboard_name, entry_idx, "relative_to", v);
                } else if v < min || v > max {
                    errors.push(DolaError::TransitionValueOutOfRange {
                        storyboard: storyboard_name.to_string(),
                        entry_index: entry_idx,
                        variable: var_name.to_string(),
                        field: if trans_def.relative_to.is_some() {
                            "relative_to"
                        } else {
                            "to"
                        }
                        .to_string(),
                        value: v,
                        min,
                        max,
                    });
                }
            }

            // Build segment (Task 5.9)
            let easing = if matches!(var_def, AnimationVariableDef::Object { .. }) {
                None
            } else {
                trans_def.easing.clone()
            };
            entry_to.insert(entry_idx, to_value.clone());
            compiled.push((
                entry_idx,
                CompiledSegment {
                    start_time: pending.start_time,
                    end_time: pending.end_time,
                    from_value,
                    to_value,
                    easing,
                },
            ));
        }

        // Overlap check (Task 6.2)。重なった後ろのセグメントのエントリを指す
        for pair in compiled.windows(2) {
            let ((_, prev), (entry_index, next)) = (&pair[0], &pair[1]);
            if prev.end_time > next.start_time {
                errors.push(DolaError::CompileError {
                    storyboard: storyboard_name.to_string(),
                    entry_index: *entry_index,
                    reason: format!(
                        "Segment overlap for variable '{}': previous end_time {} > next start_time {}",
                        var_name, prev.end_time, next.start_time
                    ),
                });
            }
        }
        let segments: Vec<CompiledSegment> = compiled.into_iter().map(|(_, s)| s).collect();

        // Build CompiledVariableTimeline (Task 6.3)
        // 長さはコンパイルの起点から数える（先頭の遅れを含む）。
        // 最後のセグメントより後のトリガーは長さに足さない（dola-nested-storyboard 3.3）。
        let base_duration = segments
            .last()
            .map_or(0.0, |last| last.end_time - start_time);

        let variable_type = build_variable_type_hint(var_def);
        let (min_value, max_value) = extract_min_max(var_def);

        timelines.insert(
            var_name.to_string(),
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
    // 有限の長さも小さい time_scale で割ると inf になり、インスタンスが終わらない
    // （ランタイムは同じ割り算で 1 周の実時間を出す。読み込みの compile_all でここを通る）
    let loop_duration = total_base_duration / sb.time_scale;
    if !loop_duration.is_finite() {
        return Err(vec![DolaError::InvalidNumber {
            location: format!("storyboard '{storyboard_name}'"),
            field: "loop_duration".to_string(),
            value: loop_duration,
        }]);
    }

    // Sort triggers by fire_time (stable sort preserves entry index order for same fire_time)
    triggers.sort_by(|a, b| {
        (a.fire_time + 0.0)
            .total_cmp(&(b.fire_time + 0.0))
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
