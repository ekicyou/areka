//! マウスボタン押下・解放メッセージハンドラ
//!
//! WM_*BUTTONDOWN, WM_*BUTTONUP の処理を担当する。WM_*BUTTONDBLCLK も押下としてここの共通処理を通る。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::{debug, trace};
use windows::Win32::Foundation::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// ボタンメッセージハンドラ共通処理
///
/// hit_test でヒット対象エンティティを特定し、ButtonBuffer に記録する。
/// PointerState がない場合は付与する。当たらないときは窓の entity に記録する。
/// `double_click` が true（`WM_*BUTTONDBLCLK` の 2 回目の押下）なら、当たった entity の
/// PointerState にボタンから決めたダブルクリックの種類を入れる。押下・ドラッグ準備の決まりはふつうの押下と同じ。
#[inline]
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_button_message(
    world: &Rc<RefCell<EcsWorld>>,
    window_entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
    button: crate::ecs::pointer::PointerButton,
    is_down: bool,
    double_click: bool,
) -> HandlerResult {
    use crate::ecs::pointer::{PhysicalPoint, PointerState};
    use crate::ecs::{PointF as HitTestPoint, hit_test_in_window};

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
    let (screen_x, screen_y) = world
        .try_borrow()
        .map(|w| screen_point(w.world(), window_entity, x, y))
        .unwrap_or((x, y));

    // hit_test でターゲットエンティティを特定し、PointerState を確保
    {
        if let Ok(mut world_borrow) = world.try_borrow_mut() {
            if let Some(target_entity) = hit_test_in_window(
                world_borrow.world(),
                window_entity,
                HitTestPoint::new(x as f32, y as f32),
            ) {
                // 当たった entity の名前と GlobalArrangement.bounds を記録（文字列は debug が開いているときだけ組む）
                debug!(
                    target_entity = ?target_entity,
                    entity_name = %world_borrow
                        .world()
                        .get::<bevy_ecs::name::Name>(target_entity)
                        .map(|n| n.as_str().to_string())
                        .unwrap_or_else(|| format!("{:?}", target_entity)),
                    client_x = x,
                    client_y = y,
                    screen_x,
                    screen_y,
                    bounds = %world_borrow
                        .world()
                        .get::<crate::ecs::layout::GlobalArrangement>(target_entity)
                        .map(|g| {
                            format!(
                                "({:.0},{:.0})-({:.0},{:.0})",
                                g.bounds.left, g.bounds.top, g.bounds.right, g.bounds.bottom
                            )
                        })
                        .unwrap_or_else(|| "no bounds".to_string()),
                    button = ?button,
                    is_down,
                    double_click,
                    "[handle_button_message] hit_test result"
                );

                let double_click_kind = if double_click {
                    double_click_of(button)
                } else {
                    crate::ecs::pointer::DoubleClick::None
                };

                // PointerState がない場合は付与（ボタンの状態は次の tick の溜めの転送が入れる）
                if let Some(mut ps) = world_borrow
                    .world_mut()
                    .get_mut::<PointerState>(target_entity)
                {
                    if double_click {
                        ps.double_click = double_click_kind;
                    }
                } else {
                    world_borrow
                        .world_mut()
                        .entity_mut(target_entity)
                        .insert(PointerState {
                            client_point: PhysicalPoint::new(x, y),
                            local_point: PhysicalPoint::new(x, y),
                            shift_down: shift,
                            ctrl_down: ctrl,
                            double_click: double_click_kind,
                            ..Default::default()
                        });
                    debug!(
                        entity = ?target_entity,
                        button = ?button,
                        is_down,
                        double_click,
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
                                debug!(
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
                    // 捕捉を取った窓と同じ窓の離しだけを、取り消しの印なしで休ませる。
                    // 開始済みなら終了の種も積む（ここで積むと二重になる）。
                    // 旧状態の CaptureGuard が Drop され ReleaseCapture が自動呼び出し
                    if button == crate::ecs::pointer::PointerButton::Left
                        && crate::ecs::drag::end_dragging_on_release(
                            hwnd,
                            Some(PhysicalPoint::new(screen_x, screen_y)),
                        ) == crate::ecs::drag::DragClose::OtherWindow
                    {
                        trace!(
                            hwnd = format!("0x{:X}", hwnd.0 as usize),
                            "[handle_button_message] Drag end skipped: HWND mismatch"
                        );
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
        // 当たり判定の枝と同じく、捕捉を取った窓と同じ窓の離しだけを休ませる。
        if button == crate::ecs::pointer::PointerButton::Left
            && crate::ecs::drag::end_dragging_on_release(
                hwnd,
                Some(PhysicalPoint::new(screen_x, screen_y)),
            ) == crate::ecs::drag::DragClose::OtherWindow
        {
            trace!(
                hwnd = format!("0x{:X}", hwnd.0 as usize),
                "[handle_button_message] Fallback drag end skipped: HWND mismatch"
            );
        }
    }

    Some(LRESULT(0))
}

/// ダブルクリックの種類をボタンから決める（Left → DoubleClick::Left ほか）。
fn double_click_of(button: crate::ecs::pointer::PointerButton) -> crate::ecs::pointer::DoubleClick {
    use crate::ecs::pointer::{DoubleClick, PointerButton};
    match button {
        PointerButton::Left => DoubleClick::Left,
        PointerButton::Right => DoubleClick::Right,
        PointerButton::Middle => DoubleClick::Middle,
        PointerButton::XButton1 => DoubleClick::XButton1,
        PointerButton::XButton2 => DoubleClick::XButton2,
    }
}

/// クライアント座標を画面座標に直す（窓の `WindowPos.position` を足す。位置が無ければそのまま）。
/// ドラッグの押した位置（押下）と今の位置（移動）が同じ規則で揃うよう、ここだけで計算する。
pub(super) fn screen_point(
    world: &bevy_ecs::world::World,
    window_entity: Entity,
    x: i32,
    y: i32,
) -> (i32, i32) {
    match world
        .get::<crate::ecs::window::WindowPos>(window_entity)
        .and_then(|wp| wp.position)
    {
        Some(pos) => (x + pos.x, y + pos.y),
        None => (x, y),
    }
}

/// X ボタンの番号（`GET_XBUTTON_WPARAM`＝wParam の上の語）をボタンの種類に直す。1 なら XButton1、ほかは XButton2。
pub(super) fn xbutton_of(wparam: WPARAM) -> crate::ecs::pointer::PointerButton {
    if ((wparam.0 >> 16) & 0xFFFF) as u16 == 1 {
        crate::ecs::pointer::PointerButton::XButton1
    } else {
        crate::ecs::pointer::PointerButton::XButton2
    }
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
        false,
    )
}

/// 解放の後にカーソルを出し直す（捕まえている間は `WM_SETCURSOR` が来ないため）。
/// 押下・解放の処理が世界の借用を返した後に呼ぶ。
fn push_release_cursor(world: &Rc<RefCell<EcsWorld>>, entity: Entity) {
    crate::ecs::cursor::answer::push_cursor_rc(
        world,
        entity,
        crate::ecs::cursor::CursorTrigger::Release,
    );
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
    let result = handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Left,
        false,
        false,
    );
    push_release_cursor(world, entity);
    result
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
        false,
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
    let result = handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Right,
        false,
        false,
    );
    push_release_cursor(world, entity);
    result
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
        false,
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
    let result = handle_button_message(
        world,
        entity,
        hwnd,
        wparam,
        lparam,
        crate::ecs::pointer::PointerButton::Middle,
        false,
        false,
    );
    push_release_cursor(world, entity);
    result
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
    let button = xbutton_of(wparam);
    handle_button_message(world, entity, hwnd, wparam, lparam, button, true, false)
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
    let button = xbutton_of(wparam);
    let result = handle_button_message(world, entity, hwnd, wparam, lparam, button, false, false);
    push_release_cursor(world, entity);
    result
}

/// target_entity自身または祖先（ChildOf辿り）からDragConfigを持つエンティティを探す。
/// 見つかった場合は (entity, DragConfig clone) を返す。
pub(super) fn find_ancestor_with_drag_config(
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

    #[test]
    fn test_screen_point_adds_window_position_or_keeps_client_point() {
        use super::screen_point;
        use crate::ecs::Point;
        use crate::ecs::window::WindowPos;
        let mut world = World::new();
        let placed = world
            .spawn(WindowPos {
                position: Some(Point { x: 100, y: -20 }),
                ..Default::default()
            })
            .id();
        let unplaced = world
            .spawn(WindowPos {
                position: None,
                ..Default::default()
            })
            .id();
        let bare = world.spawn_empty().id();
        assert_eq!(screen_point(&world, placed, 5, 7), (105, -13));
        assert_eq!(screen_point(&world, unplaced, 5, 7), (5, 7));
        assert_eq!(screen_point(&world, bare, 5, 7), (5, 7));
    }

    #[test]
    fn test_xbutton_of_reads_hiword_of_wparam() {
        use super::xbutton_of;
        use crate::ecs::pointer::PointerButton;
        use windows::Win32::Foundation::WPARAM;
        // 下の語（修飾キー）は読み分けに効かない
        assert_eq!(
            xbutton_of(WPARAM((1 << 16) | 0x04)),
            PointerButton::XButton1
        );
        assert_eq!(xbutton_of(WPARAM(2 << 16)), PointerButton::XButton2);
    }
}

#[cfg(test)]
#[path = "mouse_click_tests.rs"]
mod mouse_click_tests;
