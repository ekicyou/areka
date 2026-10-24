//! 押下の控えと解放の判定（mouse-click-wheel-events 要件 1・2・3・4・design「PressNote・ClickJudge」）。
//!
//! 控えの置き場所は `MouseWiring.presses`（鍵＝キャラ窓の entity とボタン）。

use areka_kanade::{ClickButton, MouseEventKind, MouseInput};
use bevy_ecs::prelude::*;
use wintf::ecs::WindowPos;
use wintf::ecs::drag::DragState;
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};

use super::{MouseWiring, char_scope, resolve_hit_owned};

/// 押下 1 つの控え。`double`＝その押下がそのボタンのダブルクリックとして届いた。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PressNote {
    pub double: bool,
    pub by_box: bool,
}

/// 押下の受け手から呼ぶ。その tick に押されたボタンごとに控えを書く（同じ鍵は上書き）。
/// `double` は `double_click` が同じボタンのとき真。結線の前は何もしない。
pub(super) fn note_press(world: &mut World, entity: Entity, state: &PointerState, by_box: bool) {
    let Some(mut wiring) = world.get_non_send_mut::<MouseWiring>() else {
        return;
    };
    let pressed = [
        (state.left_down, ClickButton::Left, DoubleClick::Left),
        (state.right_down, ClickButton::Right, DoubleClick::Right),
        (state.middle_down, ClickButton::Middle, DoubleClick::Middle),
        (
            state.xbutton1_down,
            ClickButton::XButton1,
            DoubleClick::XButton1,
        ),
        (
            state.xbutton2_down,
            ClickButton::XButton2,
            DoubleClick::XButton2,
        ),
    ];
    for (down, button, as_double) in pressed {
        if down {
            let double = state.double_click == as_double;
            wiring
                .presses
                .insert((entity, button), PressNote { double, by_box });
        }
    }
}

/// 左の解放がドラッグの終わりか（呼び手が wintf の状態から読む）。左以外は `NotLeft`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LeftDrag {
    NotLeft,
    Clean,
    Dragged,
}

/// クリックにしなかった理由（記録の `reason`・要件 3.6・9.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NotClickReason {
    OtherWindow,
    BoxChoice,
    Drag,
    Outside,
}

impl NotClickReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::OtherWindow => "other_window",
            Self::BoxChoice => "box_choice",
            Self::Drag => "drag",
            Self::Outside => "outside",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReleaseVerdict {
    Click { second_of_double: bool },
    NotClick(NotClickReason),
}

/// 判定の純関数。順: 控えが無い → OtherWindow／箱 → BoxChoice／左のドラッグ → Drag／
/// 放した点が窓の外 → Outside／それ以外 → Click{ second_of_double: note.double }。
pub(super) fn judge_release(
    note: Option<PressNote>,
    left: LeftDrag,
    inside: bool,
) -> ReleaseVerdict {
    let reason = match note {
        None => NotClickReason::OtherWindow,
        Some(n) if n.by_box => NotClickReason::BoxChoice,
        Some(_) if left == LeftDrag::Dragged => NotClickReason::Drag,
        Some(_) if !inside => NotClickReason::Outside,
        Some(n) => {
            return ReleaseVerdict::Click {
                second_of_double: n.double,
            };
        }
    };
    ReleaseVerdict::NotClick(reason)
}

/// 記録の `button`。kanade の `mouse_click_not_sent` と同じ Ref5 の書き方にそろえる。
fn button_name(button: ClickButton) -> &'static str {
    match button {
        ClickButton::Left => "0",
        ClickButton::Right => "1",
        ClickButton::Middle => "middle",
        ClickButton::XButton1 => "xbutton1",
        ClickButton::XButton2 => "xbutton2",
    }
}

/// 解放の受け手（Bubble だけ・常に false＝伝播は止めない）。`released` の 5 つの旗を
/// 左・右・中・拡張 1・拡張 2 の順に見て、控えを必ず取り出し、1 つずつ判定して投函する。
pub(crate) fn on_char_pointer_released(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let Phase::Bubble(state) = ev else {
        return false;
    };
    if !state.released.any() {
        return false;
    }
    if world.get_non_send::<MouseWiring>().is_none() {
        tracing::trace!(
            event = "mouse_released_no_wiring",
            "MouseWiring 不在: no-op"
        );
        return false;
    }
    let Some(scope) = char_scope(world, entity) else {
        return false;
    };
    let x = state.client_point.x as i64;
    let y = state.client_point.y as i64;
    let inside = match world.get::<WindowPos>(entity).and_then(|pos| pos.size) {
        Some(size) => (0..size.width as i64).contains(&x) && (0..size.height as i64).contains(&y),
        None => {
            tracing::trace!(
                event = "mouse_release_no_size",
                scope,
                "窓の大きさが無い: 中とみなす"
            );
            true
        }
    };
    let r = state.released;
    let released = [
        (r.left, ClickButton::Left),
        (r.right, ClickButton::Right),
        (r.middle, ClickButton::Middle),
        (r.xbutton1, ClickButton::XButton1),
        (r.xbutton2, ClickButton::XButton2),
    ];
    for (up, button) in released {
        if !up {
            continue;
        }
        let note = world
            .get_non_send_mut::<MouseWiring>()
            .expect("MouseWiring は直上で存在確認済み")
            .presses
            .remove(&(entity, button));
        let left = match button {
            ClickButton::Left if DragState::last_release_was_drag() => LeftDrag::Dragged,
            ClickButton::Left => LeftDrag::Clean,
            _ => LeftDrag::NotLeft,
        };
        match judge_release(note, left, inside) {
            ReleaseVerdict::Click { second_of_double } => {
                let hit = resolve_hit_owned(world, scope, x, y);
                let (sx, sy) = hit.surface_point;
                let input = MouseInput {
                    scope,
                    x: sx,
                    y: sy,
                    region: hit.region,
                    kind: MouseEventKind::ButtonUp {
                        button,
                        second_of_double,
                    },
                };
                world
                    .get_non_send::<MouseWiring>()
                    .expect("MouseWiring は直上で存在確認済み")
                    .send_mouse(input, "button_up");
            }
            ReleaseVerdict::NotClick(reason) => tracing::info!(
                event = "mouse_click_not_sent",
                reason = reason.as_str(),
                scope,
                button = button_name(button),
                "解放をクリックにしない"
            ),
        }
    }
    false
}

#[cfg(test)]
#[path = "click_tests.rs"]
mod tests;
