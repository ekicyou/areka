//! update_cue_sheet_trackers — cue-system のシステム定義。

use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Query, Res};

use super::queue::{CueQueue, CueQueueState};
use super::tracker::{CueSheetTracker, QueueSnapshot, TrackerAction};
use crate::ecs::graphics::FrameTime;

/// 全 CueSheetTracker の update() を呼び出すシステム。
///
/// Schedule: Update（消費者システムの **後** に実行）
/// 理由: 消費者が receive_barrier() で応答を報告した後に集計するため。
pub fn update_cue_sheet_trackers(
    mut tracker_query: Query<&mut CueSheetTracker>,
    mut queue_query: Query<&mut CueQueue>,
    frame_time: Res<FrameTime>,
) {
    let current_time = frame_time.0;

    for mut tracker in tracker_query.iter_mut() {
        let target_entities: Vec<Entity> = tracker.targets().iter().map(|(_, _, e)| *e).collect();

        // Phase 1: スナップショット収集（read-only アクセス）
        let snapshots: Vec<QueueSnapshot> = target_entities
            .iter()
            .filter_map(|&entity| {
                queue_query.get(entity).ok().map(|queue| QueueSnapshot {
                    entity,
                    state: queue.state().clone(),
                    timed_out: queue.check_timeout(current_time),
                })
            })
            .collect();

        // Phase 2: Tracker 更新
        let action = tracker.update(&snapshots);

        // Phase 3: アクション適用
        match action {
            TrackerAction::None => {}
            TrackerAction::ResolveAllClicks => {
                for &entity in &target_entities {
                    if let Ok(mut queue) = queue_query.get_mut(entity) {
                        if *queue.state() == CueQueueState::WaitingForClick {
                            queue.resolve_click();
                        }
                    }
                }
            }
            TrackerAction::SkipAllBarriers => {
                for &entity in &target_entities {
                    if let Ok(mut queue) = queue_query.get_mut(entity) {
                        queue.skip_barrier();
                    }
                }
            }
            TrackerAction::ClearAll => {
                for &entity in &target_entities {
                    if let Ok(mut queue) = queue_query.get_mut(entity) {
                        queue.clear();
                    }
                }
            }
        }
    }
}
