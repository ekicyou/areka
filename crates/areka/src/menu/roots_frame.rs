//! 右クリックメニューの「置き場を追加…」「置き場を外す ▸」枠（areka-P0-baseware-root-list・design「RootsFrame」）。
//!
//! どちらもメニューを出すたびに起動の文脈の並びと出どころを読むので、並びの変更は再起動なしで
//! 次のメニューから効く（要件 8.2）。`AREKA_ROOT` で渡しているときは 2 項目とも選べない（要件 8.6）。
//! SSP に当たる SHIORI のリソース名は無い（`caption_resource: None`）。登記はゴーストを起こすたびに
//! `ghost_session::boot_wired` が「インストール」枠の隣でやり直す。

use std::rc::Rc;

use bevy_ecs::world::World;

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::boot_config::BootContext;
use crate::roots::{self, RootsOrigin, desk, pick};

/// 2 枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::RootAdd, Rc::new(add_item));
    super::register(world, Frame::RootRemove, Rc::new(remove_item));
}

/// 並びを変えてよいか（起動の文脈が在り、`AREKA_ROOT` で渡していない）。
fn editable(world: &World) -> bool {
    world
        .get_resource::<BootContext>()
        .is_some_and(|ctx| ctx.roots_origin != RootsOrigin::EnvVar)
}

/// 「置き場を追加…」: 葉 1 つ。選ぶとフォルダを選ぶ画面を出す（要件 6.1・6.2）。
fn add_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::RootAdd).to_string(),
        caption_resource: captions::resource_for(Frame::RootAdd),
        enabled: editable(world) && desk::can_pick(world),
        checked: None,
        body: ItemBody::Action(Rc::new(|world: &mut World, _: &MenuContext| {
            let _ = desk::start_pick(world, crate::alert::suppressed(), pick::pick_folder);
        })),
    }
}

/// 「置き場を外す ▸」: 子は並びの全部の根（見つからない根も）を並びの順に絶対パスで並べる。
/// 根が 1 つなら子は選べない（要件 7.1・7.3）。
fn remove_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    let dirs: Vec<_> = world
        .get_resource::<BootContext>()
        .map(|ctx| {
            ctx.roots
                .roots()
                .iter()
                .map(|r| r.dir().to_path_buf())
                .collect()
        })
        .unwrap_or_default();
    let removable = dirs.len() > 1;
    let children = dirs
        .into_iter()
        .map(|dir| MenuItem {
            label: dir.display().to_string(),
            caption_resource: None,
            enabled: removable,
            checked: None,
            body: ItemBody::Action(Rc::new(move |world: &mut World, _: &MenuContext| {
                roots::remove_root(world, dir.clone())
            })),
        })
        .collect();
    MenuItem {
        label: captions::default_label(Frame::RootRemove).to_string(),
        caption_resource: captions::resource_for(Frame::RootRemove),
        enabled: editable(world),
        checked: None,
        body: ItemBody::Submenu(children),
    }
}

#[cfg(test)]
#[path = "roots_frame_tests.rs"]
mod tests;
