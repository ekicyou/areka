//! 右クリックメニューの「インストール」枠（areka-P0-ghost-install・design「install_frame」）。
//!
//! 項目は既定名「インストール…」と `ghostinstallbutton.caption` を持つ葉 1 つ（要件 1.8）。選べるのは
//! 窓口が在り、終了が始まっておらず、ファイルを選ぶ画面が出ていないときで、選ぶと選ぶ画面を出す
//! （[`desk::pick_and_submit`]・要件 1.3）。登記はゴーストを起こすたびに `ghost_session::boot_wired` が
//! `wire_menu` の後でやり直す（窓の無い起動では登記されない＝要件 1.12）。

use std::rc::Rc;

use bevy_ecs::world::World;

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::install::desk;

/// 「インストール」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::Install, Rc::new(install_item));
}

/// 「インストール」枠の項目。選べるかはメニューを出すたびに窓口へ尋ねる。
fn install_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::Install).to_string(),
        caption_resource: Some(captions::resource_for(Frame::Install)),
        enabled: desk::can_pick(world),
        checked: None,
        body: ItemBody::Action(Rc::new(|world: &mut World, _: &MenuContext| {
            desk::pick_and_submit(world)
        })),
    }
}

#[cfg(test)]
#[path = "install_frame_tests.rs"]
mod tests;
