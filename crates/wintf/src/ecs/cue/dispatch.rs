//! dispatch — CueSheet を各演者の CueQueue に配送するメカニズム。

use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, Query, ResMut};

use super::command::{CuePayload, EntityKey, Entry, RoutingCommand};
use super::component::PendingCueSheet;
use super::queue::CueQueue;
use super::registry::EntityRegistry;
use super::tracker::CueSheetTracker;
use super::{ActorKey, CueTarget};

/// dispatch の戻り値。CueSheetTracker 生成に必要な配送先情報。
#[derive(Debug)]
pub struct CueSheetHandle {
    pub targets: Vec<(ActorKey, CueTarget, Entity)>,
}

/// CueSheet を各演者の CueQueue に分配する内部関数。
///
/// 処理フロー:
/// 1. 各 Cue を走査
/// 2. CuePayload::Routing → EntityRegistry を更新（CueQueue には入らない）
/// 3. CuePayload::Command / Barrier → Entry に変換し routes_for_actor() で全スロットにブロードキャスト
/// 4. 各スロットの CueQueue に insert(Entry)
/// 5. 配送先リスト を返却 → CueSheetTracker 生成に使用
pub fn dispatch_cue_sheet_internal(
    sheet: &super::CueSheet,
    start_time: f64,
    registry: &mut EntityRegistry,
    cue_queues: &mut Query<&mut CueQueue>,
    tracker_entity: Entity,
) -> CueSheetHandle {
    let mut all_targets: Vec<(ActorKey, CueTarget, Entity)> = Vec::new();
    let mut seen_targets = std::collections::HashSet::new();

    tracing::debug!(
        cue_count = sheet.len(),
        start_time = start_time,
        "[dispatch_cue_sheet_internal] Dispatching CueSheet"
    );

    for cue in sheet.cues() {
        match &cue.payload {
            // ── ルーティング → EntityRegistry 更新（CueQueue には入らない）──
            CuePayload::Routing(routing) => {
                match routing {
                    RoutingCommand::RouteAdd { target, to } => {
                        if let Some(entity) = registry.resolve(to) {
                            registry.register_actor(cue.actor.clone(), target.clone(), entity);
                            tracing::debug!(
                                actor = %cue.actor.as_str(),
                                target = ?target,
                                "[dispatch] RouteAdd applied"
                            );
                        } else {
                            tracing::warn!(
                                key = ?to,
                                "[dispatch] RouteAdd target entity not found, skipping"
                            );
                        }
                    }
                    RoutingCommand::RouteSwitch { target, to } => {
                        if let Some(entity) = registry.resolve(to) {
                            registry.register_actor(cue.actor.clone(), target.clone(), entity);
                            tracing::debug!(
                                actor = %cue.actor.as_str(),
                                target = ?target,
                                "[dispatch] RouteSwitch applied"
                            );
                        } else {
                            tracing::warn!(
                                key = ?to,
                                "[dispatch] RouteSwitch target entity not found, skipping"
                            );
                        }
                    }
                    RoutingCommand::RouteRemove { target } => {
                        let key = EntityKey::Actor(cue.actor.clone(), target.clone());
                        tracing::debug!(
                            actor = %cue.actor.as_str(),
                            target = ?target,
                            "[dispatch] RouteRemove noted (full impl deferred)"
                        );
                        let _ = key; // suppress unused warning
                    }
                }
                // ルーティングは CueQueue に入らない
                continue;
            }

            // ── データコマンド → Entry::Payload に変換し各スロットへ ──
            CuePayload::Command(cmd) => {
                let absolute_time = start_time + cue.start_time;
                let routes = registry.routes_for_actor(&cue.actor);
                if routes.is_empty() {
                    tracing::warn!(
                        actor = %cue.actor.as_str(),
                        "[dispatch] No routes found for actor, skipping command"
                    );
                    continue;
                }

                for (target, entity) in routes {
                    let entry = Entry::Payload(absolute_time, cmd.clone());

                    if let Ok(mut queue) = cue_queues.get_mut(entity) {
                        if let Err(e) = queue.insert(entry) {
                            tracing::warn!(
                                actor = %cue.actor.as_str(),
                                target = ?target,
                                error = %e,
                                "[dispatch] Failed to push command to CueQueue"
                            );
                        } else {
                            queue.set_cue_sheet(tracker_entity);
                            let key = (cue.actor.clone(), target.clone(), entity);
                            if seen_targets.insert((cue.actor.clone(), target.clone())) {
                                all_targets.push(key);
                            }
                        }
                    } else {
                        tracing::warn!(
                            actor = %cue.actor.as_str(),
                            target = ?target,
                            entity = ?entity,
                            "[dispatch] Entity has no CueQueue component"
                        );
                    }
                }
            }

            // ── バリア → Entry::Barrier に変換し各スロットへ ──
            CuePayload::Barrier(barrier) => {
                let absolute_time = start_time + cue.start_time;
                let routes = registry.routes_for_actor(&cue.actor);
                if routes.is_empty() {
                    tracing::warn!(
                        actor = %cue.actor.as_str(),
                        "[dispatch] No routes found for actor, skipping barrier"
                    );
                    continue;
                }

                for (target, entity) in routes {
                    let entry = Entry::Barrier(absolute_time, barrier.clone());

                    if let Ok(mut queue) = cue_queues.get_mut(entity) {
                        if let Err(e) = queue.insert(entry) {
                            tracing::warn!(
                                actor = %cue.actor.as_str(),
                                target = ?target,
                                error = %e,
                                "[dispatch] Failed to push barrier to CueQueue"
                            );
                        } else {
                            queue.set_cue_sheet(tracker_entity);
                            let key = (cue.actor.clone(), target.clone(), entity);
                            if seen_targets.insert((cue.actor.clone(), target.clone())) {
                                all_targets.push(key);
                            }
                        }
                    } else {
                        tracing::warn!(
                            actor = %cue.actor.as_str(),
                            target = ?target,
                            entity = ?entity,
                            "[dispatch] Entity has no CueQueue component"
                        );
                    }
                }
            }
        }
    }

    CueSheetHandle {
        targets: all_targets,
    }
}

/// PendingCueSheet を検出し、CueSheet を各演者の CueQueue に配送するシステム。
///
/// 1. PendingCueSheet を持つエンティティを走査
/// 2. dispatch_cue_sheet_internal() でルーティング + 分配
/// 3. PendingCueSheet を除去し、同一エンティティに CueSheetTracker を付与
///
/// Schedule: Update（消費者システムより前）
pub fn dispatch_pending_cue_sheets(
    mut commands: Commands,
    query: Query<(Entity, &PendingCueSheet)>,
    mut registry: ResMut<EntityRegistry>,
    mut cue_queues: Query<&mut CueQueue>,
) {
    for (entity, pending) in query.iter() {
        let handle = dispatch_cue_sheet_internal(
            &pending.sheet,
            pending.start_time,
            &mut registry,
            &mut cue_queues,
            entity,
        );

        let tracker = CueSheetTracker::new(handle.targets);

        commands
            .entity(entity)
            .remove::<PendingCueSheet>()
            .insert(tracker);
    }
}
