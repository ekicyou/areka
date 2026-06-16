//! 競合検出と終了戦略の適用（Tier 3）。
//!
//! 同一変数の時間的重複を検出し、`InterruptionPolicy` に基づく
//! 5種の終了戦略を group_id 単位で適用する。

use std::collections::{HashMap, HashSet};

use crate::compile::CompiledStoryboard;
use crate::storyboard::InterruptionPolicy;

use super::instance_manager::InstanceManager;
use super::instance_state::InstanceState;
use super::subscription_manager::SubscriptionManager;
use super::timeline_manager::TimelineManager;
use super::types::{EvaluatedValue, RuntimeError};

/// 指定 group_id を除外して競合解決を行い、影響を受けた group_id のリストを返す。
/// Never 競合が検出された場合は Err(RuntimeError::Conflict) を返す。
///
/// トリガー起動時、親インスタンスを除外して子の変数競合を解決するために使用。
pub(crate) fn resolve_conflicts_excluding(
    new_group_id: u64,
    compiled: &CompiledStoryboard,
    start_time: f64,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
    skip_group_ids: &HashSet<u64>,
) -> Result<Vec<u64>, RuntimeError> {
    // 1. 競合検出
    let conflicting = detect_overlaps(compiled, timeline_manager, instance_manager, skip_group_ids);

    if conflicting.is_empty() {
        return Ok(vec![]);
    }

    // 2. 各 group_id に対して終了戦略をディスパッチ
    let mut affected = Vec::new();

    // Never チェック: 1つでも Never があれば全体を拒否
    for &gid in &conflicting {
        if let Ok(inst) = instance_manager.get(gid) {
            if inst.interruption_policy == InterruptionPolicy::Never {
                // 新規インスタンスを削除してエラーを返す
                instance_manager.remove(new_group_id);
                return Err(RuntimeError::Conflict {
                    conflicting_group_ids: conflicting.into_iter().collect(),
                });
            }
        }
    }

    // 3. 各戦略を適用
    for gid in conflicting {
        let policy = match instance_manager.get(gid) {
            Ok(inst) => inst.interruption_policy,
            Err(_) => continue, // 既に削除済み
        };

        // 戦略ごとに購読者へ伝播する確定値を収集する
        let values = match policy {
            // Cancel / Trim: start_time 時点の補間値で凍結・確定
            InterruptionPolicy::Cancel | InterruptionPolicy::Trim => timeline_manager
                .evaluate_all_for_group(gid, start_time, instance_manager.instances()),
            // Conclude: 現在再生中セグメントの最終値にジャンプ
            InterruptionPolicy::Conclude => timeline_manager
                .collect_current_segment_final_values(
                    gid,
                    start_time,
                    instance_manager.instances(),
                ),
            // Compress: ストーリーボード全体最終値にジャンプ
            InterruptionPolicy::Compress => timeline_manager.collect_final_values(gid),
            InterruptionPolicy::Never => {
                // SAFETY(panic 経路): 手前の Never チェックループが同一の
                // `instance_manager.get` で `conflicting` 全件を走査し、Never を
                // 1つでも検出した時点で早期 return している。チェックと本 match の
                // 間に instance_manager への変更はない（remove はエラー経路のみ）
                // ため、ここで Never は観測されない。
                unreachable!("Never policy should have been handled above");
            }
        };

        // Never 以外は終了状態に1対1対応する（上の match で Never は除外済み）
        let terminal_state = InstanceState::from_policy(policy)
            .expect("non-Never policy maps to a terminal state");

        terminate_instance(
            gid,
            values,
            terminal_state,
            timeline_manager,
            instance_manager,
            subscription_manager,
        );

        affected.push(gid);
    }

    Ok(affected)
}

/// 収集済みの確定値を購読者へ伝播し、終了状態へ遷移してエントリを除去する。
///
/// 全終了戦略共通の後処理: 値伝播（name→id 変換）→ 終了遷移
/// （`is_terminal()` → 自動削除）→ タイムテーブルエントリ削除。
fn terminate_instance(
    group_id: u64,
    values: HashMap<String, EvaluatedValue>,
    terminal_state: InstanceState,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
) {
    // 不変条件: 呼び出し元は from_policy 由来の終了状態のみを渡す
    // （from_policy の Some は全て is_terminal — instance_state.rs 参照）。
    debug_assert!(
        terminal_state.is_terminal(),
        "terminate_instance requires a terminal state, got {terminal_state:?}"
    );

    if !values.is_empty() {
        let id_values = subscription_manager.convert_names_to_ids(&values);
        subscription_manager.force_update_last_values(&id_values);
    }

    let _ = instance_manager.transition(group_id, terminal_state);

    timeline_manager.remove_entries(group_id);
}

/// 新セグメントと既存タイムテーブルの時間重複を検出し、
/// 競合する group_id のセットを返す。
/// Playing/Paused 状態のインスタンスのみ対象。
fn detect_overlaps(
    compiled: &CompiledStoryboard,
    timeline_manager: &TimelineManager,
    instance_manager: &InstanceManager,
    skip_group_ids: &HashSet<u64>,
) -> HashSet<u64> {
    let mut conflicting = HashSet::new();

    // Playing/Paused のインスタンスのみ対象（skip_group_ids を除外）
    let active_instances: HashSet<u64> = instance_manager
        .instances()
        .iter()
        .filter(|(gid, inst)| {
            !skip_group_ids.contains(gid)
                && (inst.state == InstanceState::Playing || inst.state == InstanceState::Paused)
        })
        .map(|(gid, _)| *gid)
        .collect();

    for (var_name, new_timeline) in &compiled.timelines {
        // 新セグメントの時間範囲（effective_time ベース）
        for new_seg in &new_timeline.segments {
            let new_start = new_seg.start_time;
            let new_end = new_seg.end_time;

            // 既存タイムテーブルの同名変数をチェック
            if let Some(existing_timeline) = timeline_manager.get_timeline(var_name) {
                for entry in &existing_timeline.entries {
                    // アクティブなインスタンスのみ
                    if !active_instances.contains(&entry.group_id) {
                        continue;
                    }

                    // セグメントレベルの重複チェック
                    for existing_seg in &entry.segments {
                        let ex_start = existing_seg.start_time;
                        let ex_end = existing_seg.end_time;

                        // 時間範囲の重複判定: start < other_end && end > other_start
                        // NOTE(数値境界): セグメント時刻に NaN が含まれる場合は両比較が
                        // false となり、当該セグメントは競合として検出されない（黙って
                        // 素通り）。NaN の流入は指示書数値の有限性検証の欠如による
                        // （proposals.md P8/P14 参照）。
                        if new_start < ex_end && new_end > ex_start {
                            conflicting.insert(entry.group_id);
                        }
                    }
                }
            }
        }
    }

    conflicting
}

