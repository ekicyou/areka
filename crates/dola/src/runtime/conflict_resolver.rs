//! 競合検出と終了戦略の適用（Tier 3）。
//!
//! 同一変数の時間的重複を検出し、`InterruptionPolicy` に基づく
//! 5種の終了戦略を group_id 単位で適用する。

use std::collections::HashSet;

use crate::compile::CompiledStoryboard;
use crate::storyboard::InterruptionPolicy;

use super::instance_manager::{InstanceManager};
use super::instance_state::InstanceState;
use super::subscription_manager::SubscriptionManager;
use super::timeline_manager::TimelineManager;
use super::types::RuntimeError;

/// 競合を検出し終了戦略を適用する。影響を受けた group_id のリストを返す。
/// Never 競合が検出された場合は Err(RuntimeError::Conflict) を返す。
pub(crate) fn resolve_conflicts(
    new_group_id: u64,
    compiled: &CompiledStoryboard,
    start_time: f64,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
) -> Result<Vec<u64>, RuntimeError> {
    // 1. 競合検出
    let conflicting = detect_overlaps(compiled, start_time, timeline_manager, instance_manager);

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

        match policy {
            InterruptionPolicy::Cancel => {
                apply_cancel(
                    gid,
                    start_time,
                    timeline_manager,
                    instance_manager,
                    subscription_manager,
                );
            }
            InterruptionPolicy::Conclude => {
                apply_conclude(
                    gid,
                    start_time,
                    timeline_manager,
                    instance_manager,
                    subscription_manager,
                );
            }
            InterruptionPolicy::Trim => {
                apply_trim(
                    gid,
                    start_time,
                    timeline_manager,
                    instance_manager,
                    subscription_manager,
                );
            }
            InterruptionPolicy::Compress => {
                apply_compress(
                    gid,
                    timeline_manager,
                    instance_manager,
                    subscription_manager,
                );
            }
            InterruptionPolicy::Never => {
                unreachable!("Never policy should have been handled above");
            }
        }

        affected.push(gid);
    }

    Ok(affected)
}

/// 新セグメントと既存タイムテーブルの時間重複を検出し、
/// 競合する group_id のセットを返す。
/// Playing/Paused 状態のインスタンスのみ対象。
fn detect_overlaps(
    compiled: &CompiledStoryboard,
    _start_time: f64,
    timeline_manager: &TimelineManager,
    instance_manager: &InstanceManager,
) -> HashSet<u64> {
    let mut conflicting = HashSet::new();

    // Playing/Paused のインスタンスのみ対象
    let active_instances: HashSet<u64> = instance_manager
        .instances()
        .iter()
        .filter(|(_, inst)| {
            inst.state == InstanceState::Playing || inst.state == InstanceState::Paused
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

/// Cancel: start_time 時点の補間値で凍結 → Cancelled 遷移 → エントリ除去
fn apply_cancel(
    group_id: u64,
    start_time: f64,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
) {
    // 1. start_time 時点の補間値を取得
    let freeze_values =
        timeline_manager.evaluate_all_for_group(group_id, start_time, instance_manager.instances());

    // 2. 凍結値を購読者に伝播
    if !freeze_values.is_empty() {
        subscription_manager.force_update_last_values(&freeze_values);
    }

    // 3. Cancelled 遷移（is_terminal() → 自動削除）
    let _ = instance_manager.transition(group_id, InstanceState::Cancelled);

    // 4. タイムテーブルエントリ削除
    timeline_manager.remove_entries(group_id);
}

/// Conclude: 現在再生中セグメントの最終値にジャンプ → Concluded 遷移 → エントリ除去
fn apply_conclude(
    group_id: u64,
    start_time: f64,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
) {
    // 1. 現在再生中セグメントの最終値を取得
    let final_values = timeline_manager.collect_current_segment_final_values(
        group_id,
        start_time,
        instance_manager.instances(),
    );

    // 2. 最終値を購読者に伝播
    if !final_values.is_empty() {
        subscription_manager.force_update_last_values(&final_values);
    }

    // 3. Concluded 遷移（自動削除）
    let _ = instance_manager.transition(group_id, InstanceState::Concluded);

    // 4. タイムテーブルエントリ削除
    timeline_manager.remove_entries(group_id);
}

/// Trim: start_time 時点の補間値で確定 → 購読者伝播 → Trimmed 遷移 → エントリ除去
fn apply_trim(
    group_id: u64,
    start_time: f64,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
) {
    // 1. start_time 時点の補間値を取得（Cancel と同じパターン）
    let trim_values =
        timeline_manager.evaluate_all_for_group(group_id, start_time, instance_manager.instances());

    // 2. 確定値を購読者に伝播
    if !trim_values.is_empty() {
        subscription_manager.force_update_last_values(&trim_values);
    }

    // 3. Trimmed 遷移（is_terminal() → 自動削除）
    let _ = instance_manager.transition(group_id, InstanceState::Trimmed);

    // 4. タイムテーブルエントリ削除
    timeline_manager.remove_entries(group_id);
}

/// Compress: ストーリーボード全体最終値にジャンプ → Compressed 遷移 → エントリ除去
fn apply_compress(
    group_id: u64,
    timeline_manager: &mut TimelineManager,
    instance_manager: &mut InstanceManager,
    subscription_manager: &mut SubscriptionManager,
) {
    // 1. 全セグメントの最終値を収集（既存 collect_final_values を再利用）
    let final_values = timeline_manager.collect_final_values(group_id);

    // 2. 最終値を購読者に伝播
    if !final_values.is_empty() {
        subscription_manager.force_update_last_values(&final_values);
    }

    // 3. Compressed 遷移（is_terminal() → 自動削除）
    let _ = instance_manager.transition(group_id, InstanceState::Compressed);

    // 4. タイムテーブルエントリ削除
    timeline_manager.remove_entries(group_id);
}
