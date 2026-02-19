//! マウス移動・ヒットテスト・離脱ハンドラ
//!
//! WM_NCHITTEST, WM_MOUSEMOVE, WM_MOUSELEAVE

#![allow(non_snake_case)]

use tracing::{debug, trace};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_NCHITTEST: 非クライアント領域ヒットテスト
///
/// クライアント領域判定を実装し、キャッシュ付きhit_testを呼び出す。
/// HTCLIENT / HTTRANSPARENT を返す（クリックスルー対応）。
#[inline]
pub(super) fn WM_NCHITTEST(
    hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    // スクリーン座標を取得（lparam から）
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

    // クライアント領域判定用に座標変換（クライアント領域外はDefWindowProcWに委譲）
    unsafe {
        use windows::Win32::Graphics::Gdi::ScreenToClient;
        let mut pt = POINT { x, y };
        if !ScreenToClient(hwnd, &mut pt).as_bool() {
            return None; // DefWindowProcW に委譲
        }

        // クライアント領域外の場合は DefWindowProcW に委譲
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return None;
        }
        if pt.x < rect.left || pt.x >= rect.right || pt.y < rect.top || pt.y >= rect.bottom {
            return None; // DefWindowProcW に委譲（非クライアント領域処理）
        }
    }

    // Entity 取得
    let Some(entity) = super::get_entity_from_hwnd(hwnd) else {
        return None;
    };

    // World 取得
    let Some(world) = super::try_get_ecs_world() else {
        return None;
    };

    // キャッシュ付きヒットテスト実行
    crate::ecs::nchittest_cache::cached_nchittest(hwnd, (x, y), entity, &world)
}

/// 当該ウィンドウに属し、exclude と異なるエンティティの PointerState 保持者を収集する。
///
/// WM_MOUSEMOVE の leave 処理で使用するヘルパー関数。
/// `find_owner_window` でウィンドウスコーピングを行い、他ウィンドウの PointerState を保護する。
pub(super) fn collect_entities_to_leave(
    world: &mut bevy_ecs::world::World,
    window_entity: bevy_ecs::prelude::Entity,
    exclude: bevy_ecs::prelude::Entity,
) -> Vec<bevy_ecs::prelude::Entity> {
    use crate::ecs::pointer::PointerState;
    use crate::ecs::window::find_owner_window;

    let mut result = Vec::new();
    let mut query = world.query::<(bevy_ecs::prelude::Entity, &PointerState)>();
    for (e, _) in query.iter(world) {
        if e != exclude && find_owner_window(world, e) == Some(window_entity) {
            result.push(e);
        }
    }
    result
}

/// WM_MOUSEMOVE: マウス移動メッセージ
///
/// 位置をPointerBufferに蓄積し、hit_testでヒットしたエンティティにPointerStateを付与。
/// 初回移動時にTrackMouseEventを設定。
#[inline]
pub(super) fn WM_MOUSEMOVE(
    hwnd: HWND,
    _message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    use crate::ecs::layout::hit_test::{PhysicalPoint as HitTestPoint, hit_test_in_window};
    use crate::ecs::pointer::{
        PointerLeave, PointerState, WindowPointerTracking, push_pointer_sample, set_modifier_state,
    };
    use std::time::Instant;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    };

    let Some(window_entity) = super::get_entity_from_hwnd(hwnd) else {
        return None;
    };

    // 位置取得（物理ピクセル、クライアント座標）
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

    tracing::trace!("[WM_MOUSEMOVE] Received: x={}, y={}", x, y);

    // 修飾キー状態を抽出
    let wparam_val = wparam.0 as u32;
    let shift = (wparam_val & 0x04) != 0; // MK_SHIFT
    let ctrl = (wparam_val & 0x08) != 0; // MK_CONTROL

    // World借用してhit_testとPointerState管理
    // ドラッグ中のSetWindowPosはWorld借用外で実行する（WM_WINDOWPOSCHANGED同期発火対策）
    let mut deferred_set_window_pos: Option<(HWND, i32, i32)> = None;

    if let Some(world) = super::try_get_ecs_world() {
        if let Ok(mut world_borrow) = world.try_borrow_mut() {
            // TrackMouseEvent 設定（ウィンドウに対して）
            if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(window_entity) {
                let needs_tracking = entity_ref
                    .get::<WindowPointerTracking>()
                    .is_none_or(|t| !t.0);

                if needs_tracking {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = unsafe { TrackMouseEvent(&mut tme) };

                    if entity_ref.get::<WindowPointerTracking>().is_some() {
                        if let Some(mut tracking) = entity_ref.get_mut::<WindowPointerTracking>() {
                            tracking.0 = true;
                        }
                    } else {
                        drop(entity_ref);
                        world_borrow
                            .world_mut()
                            .entity_mut(window_entity)
                            .insert(WindowPointerTracking(true));
                    }

                    trace!(
                        entity = ?window_entity,
                        hwnd = ?hwnd,
                        "TrackMouseEvent(TME_LEAVE) set"
                    );
                }
            }

            // hit_test でヒットしたエンティティを取得
            let hit_entity = hit_test_in_window(
                world_borrow.world(),
                window_entity,
                HitTestPoint::new(x as f32, y as f32),
            );

            // WindowPosを取得してスクリーン座標を計算
            let window_pos = world_borrow
                .world()
                .get::<crate::ecs::window::WindowPos>(window_entity);
            let (screen_x, screen_y) = if let Some(wp) = window_pos {
                if let Some(pos) = wp.position {
                    (x + pos.x, y + pos.y)
                } else {
                    (x, y)
                }
            } else {
                (x, y)
            };

            // ドラッグ処理（thread_local DragState + DragAccumulatorResource）
            // ヒットテスト結果に関わらず、Preparing/Dragging状態なら処理を続ける
            let state_snapshot = crate::ecs::drag::read_drag_state(|state| state.clone());

            match state_snapshot {
                crate::ecs::drag::DragState::Preparing {
                    entity,
                    start_pos,
                    start_time,
                } => {
                    if let Some(drag_config) = world_borrow
                        .world()
                        .get::<crate::ecs::drag::DragConfig>(entity)
                    {
                        if drag_config.enabled {
                            let current_pos =
                                crate::ecs::pointer::PhysicalPoint::new(screen_x, screen_y);

                            // 閾値チェック
                            let dx = current_pos.x - start_pos.x;
                            let dy = current_pos.y - start_pos.y;
                            let distance_sq = dx * dx + dy * dy;
                            let threshold_sq = drag_config.threshold * drag_config.threshold;

                            if distance_sq >= threshold_sq {
                                // 閾値到達：Dragging状態に遷移
                                crate::ecs::drag::start_dragging(current_pos);

                                // DragAccumulatorResourceにStarted遷移を記録
                                if let Some(accumulator) = world_borrow
                                    .world()
                                    .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                                ) {
                                    accumulator.set_transition(
                                        crate::ecs::drag::DragTransition::Started {
                                            entity,
                                            start_pos,
                                            timestamp: start_time,
                                        },
                                    );
                                    accumulator.update_position(current_pos);
                                }
                            }
                        }
                    }
                }
                crate::ecs::drag::DragState::Dragging {
                    prev_pos,
                    start_pos,
                    hwnd: drag_hwnd,
                    initial_window_pos,
                    move_window,
                    constraint,
                    ..
                } => {
                    let current_pos = crate::ecs::pointer::PhysicalPoint::new(screen_x, screen_y);

                    // ドラッグ中：デルタを累積
                    let delta = crate::ecs::pointer::PhysicalPoint::new(
                        current_pos.x - prev_pos.x,
                        current_pos.y - prev_pos.y,
                    );

                    tracing::trace!(
                        "[WM_MOUSEMOVE] Dragging: delta=({}, {}), current=({}, {})",
                        delta.x,
                        delta.y,
                        current_pos.x,
                        current_pos.y
                    );

                    // DragAccumulatorResourceにデルタを累積
                    if let Some(accumulator) = world_borrow
                        .world()
                        .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                    ) {
                        accumulator.accumulate_delta(delta);
                        accumulator.update_position(current_pos);
                    }

                    // thread_local DragStateのprev_posを更新
                    let ctx_res = world_borrow
                        .world()
                        .get_resource::<crate::ecs::drag::WindowDragContextResource>();
                    crate::ecs::drag::update_dragging(current_pos, ctx_res);

                    // WndProcレベル直接SetWindowPos（ECSパイプラインバイパス）
                    // NOTE: SetWindowPosはWM_WINDOWPOSCHANGEDを同期発火し、そこで
                    // World borrowが必要になるため、借用スコープ外に遅延実行する
                    if move_window {
                        let mut new_x = initial_window_pos.x + (current_pos.x - start_pos.x);
                        let mut new_y = initial_window_pos.y + (current_pos.y - start_pos.y);

                        // DragConstraint 適用
                        if let Some(ref c) = constraint {
                            let (cx, cy) = c.apply(new_x, new_y);
                            new_x = cx;
                            new_y = cy;
                        }

                        deferred_set_window_pos = Some((drag_hwnd, new_x, new_y));
                    }
                }
                _ => {}
            }

            // ヒットしたエンティティが存在する場合
            if let Some(target_entity) = hit_entity {
                // 当該ウィンドウに属し、target_entity と異なるエンティティを Leave 対象として収集
                let entities_to_leave = collect_entities_to_leave(
                    world_borrow.world_mut(),
                    window_entity,
                    target_entity,
                );

                // 古いエンティティからPointerStateを削除し、PointerLeaveを付与
                for old_entity in entities_to_leave {
                    if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(old_entity)
                    {
                        entity_ref.remove::<PointerState>();
                        entity_ref.insert(PointerLeave);
                        debug!(
                            old_entity = ?old_entity,
                            new_entity = ?target_entity,
                            "PointerState moved, Leave marker inserted"
                        );
                    }
                }

                // 新しいエンティティにPointerStateを挿入または維持
                if let Ok(entity_ref) = world_borrow.world_mut().get_entity_mut(target_entity) {
                    let needs_insert = entity_ref.get::<PointerState>().is_none();
                    tracing::trace!(
                        entity = ?target_entity,
                        needs_insert,
                        "[WM_MOUSEMOVE] PointerState check"
                    );
                    drop(entity_ref);

                    if needs_insert {
                        world_borrow
                            .world_mut()
                            .entity_mut(target_entity)
                            .insert(PointerState {
                                client_point: crate::ecs::pointer::PhysicalPoint::new(x, y),
                                local_point: crate::ecs::pointer::PhysicalPoint::new(x, y),
                                shift_down: shift,
                                ctrl_down: ctrl,
                                ..Default::default()
                            });
                        debug!(
                            entity = ?target_entity,
                            x, y,
                            "PointerState inserted (Enter)"
                        );
                    }
                } else {
                    tracing::warn!(
                        entity = ?target_entity,
                        "[WM_MOUSEMOVE] Failed to get entity_mut"
                    );
                }

                // PointerStateが存在することを保証した上で、バッファに蓄積
                push_pointer_sample(target_entity, x as f32, y as f32, Instant::now());
                set_modifier_state(target_entity, shift, ctrl);
            } else {
                // hit_test失敗（Windowの空白部分）
                // Windowエンティティに対して処理
                let target_entity = window_entity;

                // 当該ウィンドウに属し、target_entity と異なるエンティティを Leave 対象として収集
                let entities_to_leave = collect_entities_to_leave(
                    world_borrow.world_mut(),
                    window_entity,
                    target_entity,
                );

                // 古いエンティティからPointerStateを削除し、PointerLeaveを付与
                for old_entity in entities_to_leave {
                    if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(old_entity)
                    {
                        entity_ref.remove::<PointerState>();
                        entity_ref.insert(PointerLeave);
                        debug!(
                            old_entity = ?old_entity,
                            new_entity = ?target_entity,
                            "PointerState moved to Window, Leave marker inserted"
                        );
                    }
                }

                // PointerStateを挿入または維持
                if let Ok(entity_ref) = world_borrow.world_mut().get_entity_mut(target_entity) {
                    let needs_insert = entity_ref.get::<PointerState>().is_none();
                    drop(entity_ref);

                    if needs_insert {
                        world_borrow
                            .world_mut()
                            .entity_mut(target_entity)
                            .insert(PointerState {
                                client_point: crate::ecs::pointer::PhysicalPoint::new(x, y),
                                local_point: crate::ecs::pointer::PhysicalPoint::new(x, y),
                                shift_down: shift,
                                ctrl_down: ctrl,
                                ..Default::default()
                            });
                        debug!(
                            entity = ?target_entity,
                            x, y,
                            "PointerState inserted on Window (no hit)"
                        );
                    }
                }

                // PointerStateが存在することを保証した上で、バッファに蓄積
                push_pointer_sample(target_entity, x as f32, y as f32, Instant::now());
                set_modifier_state(target_entity, shift, ctrl);
            }
        }
    }

    // World借用解放後にSetWindowPosを実行
    // これにより同期発火するWM_WINDOWPOSCHANGEDがWorldを借用でき、
    // WindowPos.positionが正しく更新される
    if let Some((drag_hwnd, new_x, new_y)) = deferred_set_window_pos {
        let _ = unsafe {
            crate::ecs::window::guarded_set_window_pos(
                drag_hwnd,
                None,
                new_x,
                new_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    }

    Some(LRESULT(0))
}

/// WM_MOUSELEAVE: マウス離脱メッセージ
///
/// 当該ウィンドウに属するエンティティのPointerStateのみを削除し、
/// PointerLeaveマーカーを付与する（他ウィンドウのPointerStateは保持）。
#[inline]
pub(super) fn WM_MOUSELEAVE(
    hwnd: HWND,
    _message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    use crate::ecs::pointer::{PointerLeave, PointerState, WindowPointerTracking};
    use crate::ecs::window::find_owner_window;

    let Some(window_entity) = super::get_entity_from_hwnd(hwnd) else {
        return None;
    };

    if let Some(world) = super::try_get_ecs_world() {
        if let Ok(mut world_borrow) = world.try_borrow_mut() {
            // PointerStateを持つ全エンティティを収集（当該ウィンドウに属するもののみ）
            let mut entities_with_pointer_state = Vec::new();
            {
                let world_mut = world_borrow.world_mut();
                let mut query = world_mut.query::<(bevy_ecs::prelude::Entity, &PointerState)>();
                for (e, _) in query.iter(world_mut) {
                    if find_owner_window(world_mut, e) == Some(window_entity) {
                        entities_with_pointer_state.push(e);
                    }
                }
            }

            // 当該ウィンドウのエンティティからPointerStateを削除し、PointerLeaveを付与
            for entity in entities_with_pointer_state {
                if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(entity) {
                    entity_ref.remove::<PointerState>();
                    entity_ref.insert(PointerLeave);
                    debug!(
                        entity = ?entity,
                        hwnd = ?hwnd,
                        "PointerLeave marker inserted (scoped to window)"
                    );
                }
            }

            // WindowPointerTrackingを無効化
            if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(window_entity) {
                if let Some(mut tracking) = entity_ref.get_mut::<WindowPointerTracking>() {
                    tracking.0 = false;
                }
            }
        }
    }

    Some(LRESULT(0))
}
