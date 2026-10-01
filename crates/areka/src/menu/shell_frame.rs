//! 右クリックメニューの「シェル」枠（areka-P0-shell-balloon-switch・design「ShellFrame／BalloonFrame」）。
//!
//! メニューを出すたびに今のゴーストのシェルの目録（隠しを除く列挙）を読み、目録の並びのまま子項目に
//! 並べて今のシェル（実行系のマウントの末尾）に印を付ける。1 つだけでも枠は出す（要件 7.1・7.4）。
//! 子を選ぶとフォルダ名で指す出どころ＝メニューの要求を切替の唯一の入口
//! （[`request_skin_switch`]）へ出す。可否はメニューでなく入口が決める（要件 1.5・7.3）。登記は
//! ゴーストを起こすたびに `ghost_session::boot_wired` が `wire_menu` の直後でやり直す（要件 11.9）。
//! 子の並べ方は「バルーン」枠（`balloon_frame`）と共有する（[`skin_frame`]）。

use std::rc::Rc;

use areka_ghost::Identity;
use bevy_ecs::world::World;

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinOrigin, SkinRequest, SkinSpec, request_skin_switch,
};
use crate::ghost_session::GhostSlot;

/// 「シェル」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の直後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::Shell, Rc::new(shell_frame_item));
}

/// 「シェル」枠の項目。置き場のゴーストが無ければ選べない見出しだけ。
fn shell_frame_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    let Some(session) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
    else {
        tracing::trace!(
            event = "menu_shell_frame_no_ghost",
            "[menu] 置き場にゴーストが無いので「シェル」枠は選べない見出しだけを出す"
        );
        return skin_frame(Frame::Shell, SkinKind::Shell, Vec::new(), None);
    };
    let shells = areka_ghost::catalog::list_shells(session.ghost_dir())
        .into_iter()
        .map(|entry| entry.identity)
        .collect();
    skin_frame(
        Frame::Shell,
        SkinKind::Shell,
        shells,
        session.current_shell_folder().as_deref(),
    )
}

/// シェル・バルーンの枠の項目を組む。子のラベルは `name`（無ければフォルダ名）で目録の並びのまま、
/// `current` のフォルダに印を付ける。子が 0 なら選べない見出し。子を選ぶとフォルダ名で指した
/// 出どころ＝メニューの要求を入口へ出す。
pub(super) fn skin_frame(
    frame: Frame,
    kind: SkinKind,
    entries: Vec<Identity>,
    current: Option<&str>,
) -> MenuItem {
    let children: Vec<MenuItem> = entries
        .into_iter()
        .map(|identity| {
            let folder = identity.folder;
            MenuItem {
                label: identity.name.unwrap_or_else(|| folder.clone()),
                caption_resource: None,
                enabled: true,
                checked: Some(current == Some(folder.as_str())),
                body: ItemBody::Action(Rc::new(move |world: &mut World, _: &MenuContext| {
                    let verdict = request_skin_switch(
                        world,
                        SkinRequest {
                            kind,
                            target: SkinSpec::Folder(folder.clone()),
                            origin: SkinOrigin::Menu,
                        },
                    );
                    tracing::debug!(
                        event = "menu_skin_selected",
                        kind = ?kind,
                        folder = %folder,
                        verdict = ?verdict,
                        "[menu] 「シェル」「バルーン」枠の子を選んだ"
                    );
                })),
            }
        })
        .collect();
    MenuItem {
        label: captions::default_label(frame).to_string(),
        caption_resource: Some(captions::resource_for(frame)),
        enabled: !children.is_empty(),
        checked: None,
        body: ItemBody::Submenu(children),
    }
}

#[cfg(test)]
#[path = "shell_frame_tests.rs"]
mod tests;
