//! マウスボタン押下・解放メッセージハンドラ
//!
//! WM_*BUTTONDOWN, WM_*BUTTONUP の処理を担当する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::{debug, info, trace};
use windows::Win32::Foundation::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// ボタンメッセージハンドラ共通処理
///
/// hit_test でヒット対象エンティティを特定し、ButtonBuffer に記録する。
/// PointerState がない場合は付与する。
#[inline]
fn handle_button_message(
    world: &Rc<RefCell<EcsWorld>>,
    window_entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
    button: crate::ecs::pointer::PointerButton,
    is_down: bool,
) -> HandlerResult {
    use crate::ecs::layout::hit_test::{PhysicalPoint as HitTestPoint, hit_test_in_window};
    use crate::ecs::pointer::{PhysicalPoint, PointerState};

    // クリック位置を取得
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

    tracing::debug!(
        "[WM_BUTTON] button={:?}, is_down={}, x={}, y={}",
        button,
        is_down,
        x,
        y
    );

    // 修飾キー状態を抽出
    let wparam_val = wparam.0 as u32;
    let shift = (wparam_val & 0x04) != 0;
    let ctrl = (wparam_val & 0x08) != 0;

    // スクリーン座標を事前計算（フォールバック処理でも使用）
    let (screen_x, screen_y) = if let Ok(world_borrow) = world.try_borrow() {
        let window_pos = world_borrow
            .world()
            .get::<crate::ecs::window::WindowPos>(window_entity);
        if let Some(wp) = window_pos {
            if let Some(pos) = wp.position {
                (x + pos.x, y + pos.y)
            } else {
                (x, y)
            }
        } else {
            (x, y)
        }
    } else {
        (x, y)
    };

    // hit_test でターゲットエンティティを特定し、PointerState を確保
    {
        if let Ok(mut world_borrow) = world.try_borrow_mut() {
            if let Some(target_entity) = hit_test_in_window(
                world_borrow.world(),
                window_entity,
                HitTestPoint::new(x as f32, y as f32),
            ) {
                // ヒットしたエンティティの名前とGlobalArrangement.boundsをログ出力
                let entity_name = world_borrow
                    .world()
                    .get::<bevy_ecs::name::Name>(target_entity)
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| format!("{:?}", target_entity));
                let bounds_info = world_borrow
                    .world()
                    .get::<crate::ecs::layout::GlobalArrangement>(target_entity)
                    .map(|g| {
                        format!(
                            "({:.0},{:.0})-({:.0},{:.0})",
                            g.bounds.left, g.bounds.top, g.bounds.right, g.bounds.bottom
                        )
                    })
                    .unwrap_or_else(|| "no bounds".to_string());
                info!(
                    target_entity = ?target_entity,
                    entity_name = %entity_name,
                    client_x = x,
                    client_y = y,
                    screen_x,
                    screen_y,
                    bounds = %bounds_info,
                    button = ?button,
                    is_down,
                    "[handle_button_message] hit_test result"
                );

                // PointerState がない場合は付与
                if world_borrow
                    .world()
                    .get::<PointerState>(target_entity)
                    .is_none()
                {
                    world_borrow
                        .world_mut()
                        .entity_mut(target_entity)
                        .insert(PointerState {
                            client_point: PhysicalPoint::new(x, y),
                            local_point: PhysicalPoint::new(x, y),
                            left_down: button == crate::ecs::pointer::PointerButton::Left
                                && is_down,
                            right_down: button == crate::ecs::pointer::PointerButton::Right
                                && is_down,
                            middle_down: button == crate::ecs::pointer::PointerButton::Middle
                                && is_down,
                            shift_down: shift,
                            ctrl_down: ctrl,
                            ..Default::default()
                        });
                    debug!(
                        entity = ?target_entity,
                        button = ?button,
                        is_down,
                        "PointerState inserted on button event"
                    );
                }

                // 修飾キー状態を記録
                crate::ecs::pointer::set_modifier_state(target_entity, shift, ctrl);

                // ボタン状態をバッファに記録
                if is_down {
                    crate::ecs::pointer::record_button_down(target_entity, button);

                    // ドラッグ準備開始（DragConfigがあり、有効な場合）
                    // target_entity自身または祖先からDragConfigを探索
                    if button == crate::ecs::pointer::PointerButton::Left {
                        let drag_entity = {
                            let w = world_borrow.world();
                            find_ancestor_with_drag_config(w, target_entity)
                        };
                        if let Some((entity_with_config, drag_config)) = drag_entity {
                            if drag_config.enabled && drag_config.left_button {
                                tracing::info!(
                                    hit_entity = ?target_entity,
                                    drag_entity = ?entity_with_config,
                                    x = screen_x,
                                    y = screen_y,
                                    "[handle_button_message] Calling start_preparing (ancestor search)"
                                );
                                crate::ecs::drag::start_preparing(
                                    entity_with_config,
                                    PhysicalPoint::new(screen_x, screen_y),
                                    hwnd,
                                );
                            }
                        }
                    }
                } else {
                    crate::ecs::pointer::record_button_up(target_entity, button);

                    // ドラッグ終了（HWND ガード付き: 当該ウィンドウのドラッグのみ終了）
                    if button == crate::ecs::pointer::PointerButton::Left {
                        let state_snapshot = crate::ecs::drag::snapshot_drag_state();

                        let should_end = match &state_snapshot {
                            crate::ecs::drag::DragStateSnapshot::Dragging {
                                hwnd: drag_hwnd,
                                ..
                            } => *drag_hwnd == hwnd,
                            crate::ecs::drag::DragStateSnapshot::Preparing { entity, .. }
                            | crate::ecs::drag::DragStateSnapshot::JustStarted { entity, .. } => {
                                crate::ecs::window::find_owner_window(world_borrow.world(), *entity)
                                    == Some(window_entity)
                            }
                            _ => false,
                        };

                        if should_end {
                            if let crate::ecs::drag::DragStateSnapshot::Dragging {
                                entity, ..
                            }
                            | crate::ecs::drag::DragStateSnapshot::Preparing {
                                entity, ..
                            }
                            | crate::ecs::drag::DragStateSnapshot::JustStarted {
                                entity, ..
                            } = &state_snapshot
                            {
                                // DragAccumulatorResourceにEnded遷移を記録
                                if let Some(accumulator) = world_borrow
                                    .world()
                                    .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                                ) {
                                    accumulator.set_transition(
                                        crate::ecs::drag::DragTransition::Ended {
                                            entity: *entity,
                                            end_pos: PhysicalPoint::new(screen_x, screen_y),
                                            cancelled: false,
                                        },
                                    );
                                }
                            }

                            // thread_local DragStateをJustEndedに遷移
                            // 旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
                            crate::ecs::drag::end_dragging(
                                PhysicalPoint::new(screen_x, screen_y),
                                false,
                            );
                        } else {
                            trace!(
                                hwnd = format!("0x{:X}", hwnd.0 as usize),
                                "[handle_button_message] Drag end skipped: HWND mismatch"
                            );
                        }
                    }
                }

                return Some(LRESULT(0));
            }
        }
    }

    // フォールバック: ウィンドウエンティティに記録
    if is_down {
        crate::ecs::pointer::record_button_down(window_entity, button);
    } else {
        crate::ecs::pointer::record_button_up(window_entity, button);

        // ドラッグ終了（hit_test失敗時でもドラッグ中なら終了処理、HWNDガード付き）
        if button == crate::ecs::pointer::PointerButton::Left {
            let state_snapshot = crate::ecs::drag::snapshot_drag_state();

            let should_end = match &state_snapshot {
                crate::ecs::drag::DragStateSnapshot::Dragging {
                    hwnd: drag_hwnd, ..
                } => *drag_hwnd == hwnd,
                crate::ecs::drag::DragStateSnapshot::Preparing { entity, .. }
                | crate::ecs::drag::DragStateSnapshot::JustStarted { entity, .. } => {
                    if let Ok(world_borrow) = world.try_borrow() {
                        crate::ecs::window::find_owner_window(world_borrow.world(), *entity)
                            == Some(window_entity)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if should_end {
                {
                    if let Ok(world_borrow) = world.try_borrow() {
                        if let crate::ecs::drag::DragStateSnapshot::Dragging { entity, .. }
                        | crate::ecs::drag::DragStateSnapshot::Preparing { entity, .. }
                        | crate::ecs::drag::DragStateSnapshot::JustStarted { entity, .. } =
                            &state_snapshot
                        {
                            // DragAccumulatorResourceにEnded遷移を記録
                            if let Some(accumulator) = world_borrow
                                .world()
                                .get_resource::<crate::ecs::drag::DragAccumulatorResource>(
                            ) {
                                accumulator.set_transition(
                                    crate::ecs::drag::DragTransition::Ended {
                                        entity: *entity,
                                        end_pos: PhysicalPoint::new(screen_x, screen_y),
                                        cancelled: false,
                                    },
                                );
                            }
                        }
                    }
                }

                // thread_local DragStateをJustEndedに遷移
                // 旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
                crate::ecs::drag::end_dragging(PhysicalPoint::new(screen_x, screen_y), false);
            } else {
                trace!(
                    hwnd = format!("0x{:X}", hwnd.0 as usize),
                    "[handle_button_message] Fallback drag end skipped: HWND mismatch"
                );
            }
        }
    }

    Some(LRESULT(0))
}

/// WM_LBUTTONDOWN: 左ボタン押下
#[inline]
pub(super) fn WM_LBUTTONDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Left,
        true,
    )
}

/// WM_LBUTTONUP: 左ボタン解放
#[inline]
pub(super) fn WM_LBUTTONUP(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Left,
        false,
    )
}

/// WM_RBUTTONDOWN: 右ボタン押下
#[inline]
pub(super) fn WM_RBUTTONDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Right,
        true,
    )
}

/// WM_RBUTTONUP: 右ボタン解放
#[inline]
pub(super) fn WM_RBUTTONUP(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Right,
        false,
    )
}

/// WM_MBUTTONDOWN: 中ボタン押下
#[inline]
pub(super) fn WM_MBUTTONDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Middle,
        true,
    )
}

/// WM_MBUTTONUP: 中ボタン解放
#[inline]
pub(super) fn WM_MBUTTONUP(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Middle,
        false,
    )
}

/// WM_XBUTTONDOWN: 拡張ボタン押下
#[inline]
pub(super) fn WM_XBUTTONDOWN(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    // GET_XBUTTON_WPARAM: HIWORD of wParam
    let xbutton = ((wparam.0 >> 16) & 0xFFFF) as u16;
    let button = if xbutton == 1 {
        crate::ecs::pointer::PointerButton::XButton1
    } else {
        crate::ecs::pointer::PointerButton::XButton2
    };
    handle_button_message(world, entity, hwnd, wparam, lparam, button, true)
}

/// WM_XBUTTONUP: 拡張ボタン解放
#[inline]
pub(super) fn WM_XBUTTONUP(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    let xbutton = ((wparam.0 >> 16) & 0xFFFF) as u16;
    let button = if xbutton == 1 {
        crate::ecs::pointer::PointerButton::XButton1
    } else {
        crate::ecs::pointer::PointerButton::XButton2
    };
    handle_button_message(world, entity, hwnd, wparam, lparam, button, false)
}

/// target_entity自身または祖先（ChildOf辿り）からDragConfigを持つエンティティを探す。
/// 見つかった場合は (entity, DragConfig clone) を返す。
fn find_ancestor_with_drag_config(
    world: &bevy_ecs::world::World,
    start: bevy_ecs::entity::Entity,
) -> Option<(bevy_ecs::entity::Entity, crate::ecs::drag::DragConfig)> {
    let mut current = start;
    loop {
        if let Some(config) = world.get::<crate::ecs::drag::DragConfig>(current) {
            return Some((current, config.clone()));
        }
        if let Some(child_of) = world.get::<bevy_ecs::prelude::ChildOf>(current) {
            current = child_of.parent();
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::find_ancestor_with_drag_config;
    use crate::ecs::drag::DragConfig;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::prelude::*;

    // find_ancestor_with_drag_config は World ベースの ChildOf 走査でデバイス非依存。
    // start 自身 / 祖先 / 不在 / 最近傍優先 の各分岐を特性化する。

    #[test]
    fn test_find_drag_config_on_start_entity_itself() {
        // start 自身が DragConfig を持つ → (start, config) を返す
        let mut world = World::new();
        let start = world
            .spawn(DragConfig {
                threshold: 7,
                ..Default::default()
            })
            .id();

        let result = find_ancestor_with_drag_config(&world, start);
        let (entity, config) = result.expect("start 自身の DragConfig を返す");
        assert_eq!(entity, start);
        assert_eq!(config.threshold, 7); // clone されたフィールド値を確認
    }

    #[test]
    fn test_find_drag_config_on_ancestor() {
        // start には DragConfig がなく、祖先（親の親）が持つ → 祖先 entity を返す
        let mut world = World::new();
        let grandparent = world
            .spawn(DragConfig {
                threshold: 12,
                ..Default::default()
            })
            .id();
        let parent = world.spawn(ChildOf(grandparent)).id();
        let start = world.spawn(ChildOf(parent)).id();

        let result = find_ancestor_with_drag_config(&world, start);
        let (entity, config) = result.expect("祖先の DragConfig を返す");
        assert_eq!(entity, grandparent);
        assert_eq!(config.threshold, 12);
    }

    #[test]
    fn test_find_drag_config_returns_none_when_absent() {
        // チェーン上のどこにも DragConfig がない → None
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let parent = world.spawn(ChildOf(root)).id();
        let start = world.spawn(ChildOf(parent)).id();

        assert!(find_ancestor_with_drag_config(&world, start).is_none());
    }

    #[test]
    fn test_find_drag_config_returns_none_for_isolated_entity() {
        // ChildOf を持たない孤立エンティティで DragConfig なし → None（ループ終端）
        let mut world = World::new();
        let isolated = world.spawn_empty().id();

        assert!(find_ancestor_with_drag_config(&world, isolated).is_none());
    }

    #[test]
    fn test_find_drag_config_prefers_nearest_ancestor() {
        // start と祖先の両方が DragConfig を持つ → 最近傍（start 自身）を優先
        let mut world = World::new();
        let grandparent = world
            .spawn(DragConfig {
                threshold: 99,
                ..Default::default()
            })
            .id();
        let parent = world.spawn(ChildOf(grandparent)).id();
        let start = world
            .spawn((
                ChildOf(parent),
                DragConfig {
                    threshold: 3,
                    ..Default::default()
                },
            ))
            .id();

        let result = find_ancestor_with_drag_config(&world, start);
        let (entity, config) = result.expect("最近傍の DragConfig を返す");
        assert_eq!(entity, start);
        assert_eq!(config.threshold, 3); // 祖先の 99 ではなく start の 3
    }
}
