//! 右クリックメニューの「ゴースト」枠（areka-P0-ghost-shell-balloon-switch・design「GhostFrame」）。
//!
//! メニューを出すたびに起動の文脈の根で目録を読み、目録の並びのままゴーストを子項目に並べて
//! 今のゴーストに印を付ける（要件 1.11）。子を選ぶとフォルダ名で指す「手動」の切替要求を
//! 切替の唯一の入口（[`request_ghost_switch`]）へ出す（要件 1.4）。登記はゴーストを起こすたびに
//! `ghost_session::boot_wired` が `wire_menu` の直後でやり直す（要件 1.12）。

use std::rc::Rc;

use areka_kanade::ChangeOrigin;
use bevy_ecs::world::World;

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::{GhostSpec, SwitchRequest, request_ghost_switch};

/// 「ゴースト」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の直後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::Ghost, Rc::new(ghost_frame_item));
}

/// 「ゴースト」枠の項目。子のラベルは `descript.txt` の `name`（無ければフォルダ名）で、
/// `sakura.name` は読まない。0 体なら子 0 の選べない見出し、起動の文脈が無ければ選べない見出しだけ。
fn ghost_frame_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    let frame = |enabled, children| MenuItem {
        label: captions::default_label(Frame::Ghost).to_string(),
        caption_resource: Some(captions::resource_for(Frame::Ghost)),
        enabled,
        checked: None,
        body: ItemBody::Submenu(children),
    };
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::trace!(
            event = "menu_ghost_frame_no_context",
            "[menu] 起動の文脈が無いので「ゴースト」枠は選べない見出しだけを出す"
        );
        return frame(false, Vec::new());
    };
    let current = ctx.current.ghost.folder.as_deref();
    let children: Vec<MenuItem> = areka_ghost::catalog::list_ghosts(&ctx.root)
        .into_iter()
        .map(|entry| {
            let folder = entry.identity.folder;
            MenuItem {
                label: entry.identity.name.unwrap_or_else(|| folder.clone()),
                caption_resource: None,
                enabled: true,
                checked: Some(current == Some(folder.as_str())),
                body: ItemBody::Action(Rc::new(move |world: &mut World, _: &MenuContext| {
                    let verdict = request_ghost_switch(
                        world,
                        SwitchRequest {
                            ghost: GhostSpec::Folder(folder.clone()),
                            raise_event: true,
                            origin: ChangeOrigin::Manual,
                            boot_event: None,
                        },
                    );
                    tracing::debug!(
                        event = "menu_ghost_selected",
                        folder = %folder,
                        verdict = ?verdict,
                        "[menu] 「ゴースト」枠の子を選んだ"
                    );
                })),
            }
        })
        .collect();
    frame(!children.is_empty(), children)
}

#[cfg(test)]
#[path = "ghost_frame_tests.rs"]
mod tests;
