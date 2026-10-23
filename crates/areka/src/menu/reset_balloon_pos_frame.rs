//! 右クリックメニューの「バルーン位置初期化」の葉（areka-P0-window-link-directives 要件 7.5〜7.7・
//! design「reset_balloon_pos_frame」）。
//!
//! 選ぶと [`reset_balloon_positions`] をきっかけ「メニュー」で呼ぶ。対のイベントが無いので
//! ゴーストへ知らせは送らず、答えも待たない（7.7）。

use std::rc::Rc;

use bevy_ecs::world::World;

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::placement::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use crate::placement::reset_pos::DefaultPlacementBasis;
use crate::placement::spawn::GhostWindows;

/// 「バルーン位置初期化」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の後）。
pub(crate) fn register(world: &mut World) {
    super::register(
        world,
        Frame::ResetBalloonPosition,
        Rc::new(reset_balloon_item),
    );
}

/// 選べる場面: 窓の組と材料（[`DefaultPlacementBasis`]）が在るとき（7.5）。答え待ちの灰色は無い。
pub(crate) fn reset_balloon_enabled(world: &World) -> bool {
    world.contains_resource::<GhostWindows>() && world.contains_resource::<DefaultPlacementBasis>()
}

/// 「バルーン位置初期化」の葉（要件 7.5〜7.7）。
fn reset_balloon_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::ResetBalloonPosition).to_string(),
        caption_resource: captions::resource_for(Frame::ResetBalloonPosition),
        enabled: reset_balloon_enabled(world),
        checked: None,
        body: ItemBody::Action(Rc::new(|world: &mut World, _: &MenuContext| {
            reset_balloon_positions(world, BalloonResetTrigger::Menu);
        })),
    }
}

#[cfg(test)]
#[path = "reset_balloon_pos_frame_tests.rs"]
mod tests;
