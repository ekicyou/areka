//! 右クリックメニューの「バルーン」枠（areka-P0-shell-balloon-switch・design「ShellFrame／BalloonFrame」）。
//!
//! メニューを出すたびに起動の文脈の根でバルーンの目録を読み、目録の並びのまま子項目に並べて今の
//! バルーン（起動の文脈の `current.balloon.folder`）に印を付ける。1 つだけでも枠は出す（要件 7.2・7.4）。
//! 子を選ぶとフォルダ名で指す出どころ＝メニューの要求を切替の入口へ出す（要件 1.5・7.3）。
//! 子の並べ方と選んだときの動作は「シェル」枠と共有する（[`super::shell_frame::skin_frame`]）。

use std::rc::Rc;

use bevy_ecs::world::World;

use super::shell_frame::skin_frame;
use super::{Frame, MenuContext, MenuItem};
use crate::boot_config::BootContext;
use crate::emo2_boot::shell_balloon_switch::SkinKind;

/// 「バルーン」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の直後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::Balloon, Rc::new(balloon_frame_item));
}

/// 「バルーン」枠の項目。起動の文脈が無ければ選べない見出しだけ。
fn balloon_frame_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::trace!(
            event = "menu_balloon_frame_no_context",
            "[menu] 起動の文脈が無いので「バルーン」枠は選べない見出しだけを出す"
        );
        return skin_frame(Frame::Balloon, SkinKind::Balloon, Vec::new(), None);
    };
    let balloons = areka_ghost::catalog::list_balloons(&ctx.root)
        .into_iter()
        .map(|entry| entry.identity)
        .collect();
    skin_frame(
        Frame::Balloon,
        SkinKind::Balloon,
        balloons,
        ctx.current.balloon.folder.as_deref(),
    )
}

#[cfg(test)]
#[path = "balloon_frame_tests.rs"]
mod tests;
