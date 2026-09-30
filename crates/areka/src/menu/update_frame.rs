//! 右クリックメニューの「ネットワーク更新」枠（areka-P0-network-update・design「メニュー」）。
//!
//! 項目は既定名「ネットワーク更新」と `updatebutton.caption` を持つ葉 1 つ（要件 1.2）。選べるのは
//! 今のゴースト・シェル・バルーンのどれかに更新先があり、更新が走っておらず、終了が始まっていない
//! ときで（[`desk::can_update`]・要件 1.3）、選ぶと今の 3 つを理由 `manual` で受付へ掛ける
//! （[`desk::update_current`]・要件 1.4）。登記はゴーストを起こすたびに `ghost_session::boot_wired` が
//! `wire_menu` の後でやり直す（窓の無い起動では登記されない）。

use std::rc::Rc;

use bevy_ecs::world::World;

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::update::desk;

/// 「ネットワーク更新」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::Update, Rc::new(update_item));
}

/// 「ネットワーク更新」枠の項目。選べるかはメニューを出すたびに窓口へ尋ねる。
fn update_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::Update).to_string(),
        caption_resource: Some(captions::resource_for(Frame::Update)),
        enabled: desk::can_update(world),
        checked: None,
        body: ItemBody::Action(Rc::new(|world: &mut World, _: &MenuContext| {
            desk::update_current(world)
        })),
    }
}

#[cfg(test)]
#[path = "update_frame_tests.rs"]
mod tests;
