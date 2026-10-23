//! 押下の控えと解放の判定（mouse-click-wheel-events 要件 1・2・3・4・design「PressNote・ClickJudge」）。
//!
//! 控えの置き場所は `MouseWiring.presses`（鍵＝キャラ窓の entity とボタン）。解放の判定は task 3.4。

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "控えの中身は解放の判定（task 3.4）が読むまで本番の読み手が居ない"
    )
)]

use areka_kanade::ClickButton;
use bevy_ecs::prelude::*;
use wintf::ecs::pointer::{DoubleClick, PointerState};

use super::MouseWiring;

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

#[cfg(test)]
#[path = "click_tests.rs"]
mod tests;
