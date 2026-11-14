//! ドラッグイベントディスパッチ
//!
//! DragStateからイベントを生成し、Phase<T>でTunnel/Bubble配信する。

use crate::ecs::pointer::{EventHandler, PhysicalPoint, build_bubble_path};
use bevy_ecs::prelude::*;
use std::time::Instant;

/// ドラッグ開始イベント
#[derive(Clone, Debug)]
pub struct DragStartEvent {
    /// ドラッグ対象エンティティ
    pub target: Entity,
    /// ドラッグ開始位置（物理ピクセル、スクリーン座標）
    pub position: PhysicalPoint,
    /// イベント発生時刻
    pub timestamp: Instant,
}

/// ドラッグ中イベント
#[derive(Clone, Debug)]
pub struct DragEvent {
    /// ドラッグ対象エンティティ
    pub target: Entity,
    /// ドラッグ開始位置（物理ピクセル、スクリーン座標）
    pub start_position: PhysicalPoint,
    /// 現在位置（物理ピクセル、スクリーン座標）
    pub position: PhysicalPoint,
    /// イベント発生時刻
    pub timestamp: Instant,
}

/// ドラッグ終了イベント
#[derive(Clone, Debug)]
pub struct DragEndEvent {
    /// ドラッグ対象エンティティ
    pub target: Entity,
    /// 終了位置（物理ピクセル、スクリーン座標）
    pub position: PhysicalPoint,
    /// キャンセルされたかどうか
    pub cancelled: bool,
    /// イベント発生時刻
    pub timestamp: Instant,
}

/// ドラッグ開始ハンドラコンポーネント
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnDragStart(pub EventHandler<DragStartEvent>);

/// ドラッグ中ハンドラコンポーネント
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnDrag(pub EventHandler<DragEvent>);

/// ドラッグ終了ハンドラコンポーネント
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct OnDragEnd(pub EventHandler<DragEndEvent>);

/// ドラッグイベントディスパッチシステム
///
/// DragAccumulatorResourceから累積量をflushしてドラッグイベントを配信する。
pub fn dispatch_drag_events(world: &mut World) {
    // DragAccumulatorResourceをflush
    let flush_result = world
        .resource::<crate::ecs::drag::DragAccumulatorResource>()
        .flush();

    let Some(flush_result) = flush_result else {
        return;
    };

    // 状態遷移イベントを積んだ順にすべて処理
    for transition in flush_result.transitions {
        match transition {
            crate::ecs::drag::DragTransition::Started {
                entity,
                start_pos,
                timestamp,
            } => {
                // 持ち主の窓を引き、HWND・始めの位置を求める（位置が取れなければ窓は動かさない）。
                // HWND は位置と別に控える: 下流（バルーンの止め）が控えの hwnd と move_window を組で読む。
                let window_entity = crate::ecs::find_owner_window(world, entity);
                let hwnd = window_entity
                    .and_then(|w| world.get::<crate::ecs::window::WindowHandle>(w))
                    .map(|wh| wh.hwnd);
                let origin = match window_entity
                    .ok_or_else(|| "持ち主の窓が無い".to_string())
                    .and_then(|window| drag_origin(world, window))
                {
                    Ok(pos) => Some(pos),
                    Err(reason) => {
                        tracing::warn!(
                            ?entity,
                            window = ?window_entity,
                            reason,
                            "[dispatch_drag_events] ドラッグを始めた窓の位置が取れない: 窓は動かさない"
                        );
                        None
                    }
                };
                let initial_window_pos = origin.unwrap_or(crate::ecs::Point { x: 0, y: 0 });
                let mut move_window = false;
                let mut constraint: Option<crate::ecs::drag::DragConstraint> = None;

                // DragConfig.move_window と DragConstraint を取得（ドラッグ対象エンティティから）
                if let Some(dc) = world.get::<crate::ecs::drag::DragConfig>(entity) {
                    move_window = dc.move_window;
                }
                if let Some(dc) = world.get::<crate::ecs::drag::DragConstraint>(entity) {
                    constraint = Some(*dc);
                }

                // WindowDragContextResource に書き込み（update_dragging での読み取り用）
                if let Some(ctx_res) =
                    world.get_resource::<crate::ecs::drag::WindowDragContextResource>()
                {
                    ctx_res.set(crate::ecs::drag::WindowDragContext {
                        hwnd,
                        initial_window_pos: origin,
                        move_window,
                        constraint,
                    });
                }

                // DraggingStateコンポーネント挿入
                if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                    entity_mut.insert(crate::ecs::drag::DraggingState {
                        drag_start_pos: start_pos,
                        initial_window_pos: (
                            initial_window_pos.x as f32,
                            initial_window_pos.y as f32,
                        ),
                    });

                    tracing::debug!(
                        entity = ?entity,
                        initial_window_x = initial_window_pos.x,
                        initial_window_y = initial_window_pos.y,
                        move_window = move_window,
                        "[dispatch_drag_events] DraggingState inserted"
                    );
                }

                // WindowDragging マーカーを Window entity に insert
                if let Some(we) = window_entity {
                    if let Ok(mut window_mut) = world.get_entity_mut(we) {
                        window_mut.insert(crate::ecs::drag::WindowDragging);
                        tracing::debug!(
                            window_entity = ?we,
                            "[dispatch_drag_events] WindowDragging marker inserted"
                        );
                    }
                }

                // DragStartEvent送信
                let event = DragStartEvent {
                    target: entity,
                    position: start_pos,
                    timestamp,
                };

                tracing::debug!(
                    entity = ?entity,
                    x = start_pos.x,
                    y = start_pos.y,
                    "[DragStartEvent] Dispatching"
                );

                let path = build_bubble_path(world, entity);
                crate::ecs::pointer::dispatch_event_for_handler::<DragStartEvent, OnDragStart>(
                    world,
                    entity,
                    &path,
                    &event,
                    |h| h.0,
                );

                // JustStarted→Dragging遷移（次のWM_MOUSEMOVEでDragEventが発火できるように）。
                // 前のドラッグの始まりの腕が、次のドラッグの状態を前の控えで進めない（指摘 1 ⑵）。
                if super::state::read_drag_state(
                    |s| matches!(s, super::DragState::JustStarted { entity: e, .. } if *e == entity),
                ) {
                    let ctx_res =
                        world.get_resource::<crate::ecs::drag::WindowDragContextResource>();
                    super::state::update_dragging(start_pos, ctx_res);
                }
            }

            crate::ecs::drag::DragTransition::Ended {
                entity,
                end_pos,
                cancelled,
            } => {
                // DragEndEvent送信
                let event = DragEndEvent {
                    target: entity,
                    position: end_pos,
                    cancelled,
                    timestamp: Instant::now(),
                };

                tracing::debug!(
                    entity = ?entity,
                    x = end_pos.x,
                    y = end_pos.y,
                    cancelled,
                    "[DragEndEvent] Dispatching"
                );

                let path = build_bubble_path(world, entity);
                crate::ecs::pointer::dispatch_event_for_handler::<DragEndEvent, OnDragEnd>(
                    world,
                    entity,
                    &path,
                    &event,
                    |h| h.0,
                );

                // DraggingStateコンポーネント削除
                if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                    entity_mut.remove::<crate::ecs::drag::DraggingState>();

                    tracing::debug!(
                        entity = ?entity,
                        "[dispatch_drag_events] DraggingState removed"
                    );
                }

                // 持ち主の窓から WindowDragging を remove + Arrangement.offset を同期
                if let Some(window) = crate::ecs::find_owner_window(world, entity) {
                    // WindowDragging マーカーを remove
                    if let Ok(mut window_mut) = world.get_entity_mut(window) {
                        window_mut.remove::<crate::ecs::drag::WindowDragging>();
                        tracing::debug!(
                            window_entity = ?window,
                            "[dispatch_drag_events] WindowDragging marker removed"
                        );
                    }

                    // Arrangement.offset を WindowPos.position から直接同期
                    //
                    // 旧方式: wp.set_changed() で Changed<WindowPos> を発火 →
                    //   sync_window_arrangement_from_window_pos で offset 更新
                    //   + apply_window_pos_changes で冗長な SetWindowPos 発火
                    //
                    // 問題: apply_window_pos_changes が client→window 座標変換を行い
                    //   SetWindowPos を呼ぶが、変換の丸め誤差やタイミングにより
                    //   ウィンドウがフレームオフセット分だけジャンプする場合がある。
                    //
                    // 新方式: Arrangement.offset を直接更新し、Changed<WindowPos> は
                    //   発火させない。これにより冗長な SetWindowPos を回避しつつ、
                    //   hit_test に必要な GA.bounds の整合性を維持する。
                    if let Ok(mut window_mut) = world.get_entity_mut(window) {
                        let pos_opt = window_mut
                            .get::<crate::ecs::window::WindowPos>()
                            .and_then(|wp| wp.position);

                        if let Some(pos) = pos_opt {
                            let new_offset = crate::ecs::layout::Offset {
                                x: pos.x as f32,
                                y: pos.y as f32,
                            };
                            if let Some(mut arr) =
                                window_mut.get_mut::<crate::ecs::layout::Arrangement>()
                            {
                                if arr.offset != new_offset {
                                    tracing::debug!(
                                        window_entity = ?window,
                                        old_offset = format_args!("({:.0},{:.0})", arr.offset.x, arr.offset.y),
                                        new_offset = format_args!("({:.0},{:.0})", new_offset.x, new_offset.y),
                                        "[DragEnd] Direct Arrangement.offset sync"
                                    );
                                    arr.offset = new_offset;
                                } else {
                                    tracing::debug!(
                                        window_entity = ?window,
                                        offset = format_args!("({:.0},{:.0})", new_offset.x, new_offset.y),
                                        "[DragEnd] Arrangement.offset unchanged"
                                    );
                                }
                            }
                        }
                    }
                }

                // WindowDragContextResource をクリア
                if let Some(ctx_res) =
                    world.get_resource::<crate::ecs::drag::WindowDragContextResource>()
                {
                    ctx_res.clear();
                }
            }
        }
    }

    // 累積デルタが非ゼロなら DragEvent 配信
    if let Some(entity) = flush_result.current_dragging_entity {
        if flush_result.delta.x != 0 || flush_result.delta.y != 0 {
            // DraggingStateからstart_posを取得
            let start_pos = world
                .get::<crate::ecs::drag::DraggingState>(entity)
                .map(|ds| ds.drag_start_pos)
                .unwrap_or(flush_result.current_position);

            // DragEvent送信
            let event = DragEvent {
                target: entity,
                start_position: start_pos,
                position: flush_result.current_position,
                timestamp: Instant::now(),
            };

            tracing::trace!(
                entity = ?entity,
                start_x = start_pos.x,
                start_y = start_pos.y,
                current_x = flush_result.current_position.x,
                current_y = flush_result.current_position.y,
                delta_x = flush_result.current_position.x - start_pos.x,
                delta_y = flush_result.current_position.y - start_pos.y,
                "[DragEvent] Dispatching"
            );

            let path = build_bubble_path(world, entity);
            crate::ecs::pointer::dispatch_event_for_handler::<DragEvent, OnDrag>(
                world,
                entity,
                &path,
                &event,
                |h| h.0,
            );
        }
    }
}

/// ドラッグを始めたときの窓の位置（枠を含むスクリーン座標）。
///
/// `WindowHandle` があれば `WindowPos.position` を `client_to_window_coords` で枠込みへ直す。
/// `WindowHandle` が無ければ `WindowPos.position` をそのまま使う（テストの道）。
/// 位置が無い・変換が失敗なら `Err`（理由の文）。
fn drag_origin(world: &World, window: Entity) -> Result<crate::ecs::Point, String> {
    let wp = world
        .get::<crate::ecs::window::WindowPos>(window)
        .ok_or_else(|| "WindowPos が無い".to_string())?;
    let pos = wp
        .position
        .ok_or_else(|| "WindowPos.position が無い".to_string())?;
    match world.get::<crate::ecs::window::WindowHandle>(window) {
        // SetWindowPos はウィンドウ枠を含むウィンドウ座標を期待するため
        Some(wh) => wh
            .client_to_window_coords(pos, wp.size.unwrap_or_default())
            .map(|(x, y, _, _)| crate::ecs::Point { x, y }),
        None => Ok(pos),
    }
}

#[cfg(test)]
#[path = "dispatch_order_tests.rs"]
mod order_tests;
