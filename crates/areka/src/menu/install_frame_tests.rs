//! 「インストール」枠の決定論テスト（areka-P0-ghost-install・要件 1.3・1.8・1.12）。
//!
//! 確かめること: 枠の項目は既定名「インストール…」と `ghostinstallbutton.caption` を持つ葉で、
//! 窓口が在り終了が始まっていなければ選べること・窓口が無い／終了が始まった後は選べないこと・
//! 結線をやり直すと登記が新品になり「インストール」枠が置き換えの記録なしに登記し直されること。
//! 選ぶ画面が出ている間に選べないことは `install/desk_pick_tests.rs` が確かめる（旗は窓口の中）。
//! 本番の結線（`ghost_session::boot_wired` が起こすたびに登記する）は
//! `ghost_session_restart_tests.rs`・`ghost_session_switch_tests.rs` の登記の一覧が確かめる。
//! 項目の動作は呼ばない（本物の選ぶ画面が出る）。

use std::sync::mpsc;

use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::capture;

use super::*;
use crate::exit_wait::begin_close;
use crate::menu::{Frame, ItemBody, MenuContext, MenuItem, MenuWiring, wire_menu};

/// メニューを結線し「インストール」枠を登記した World（`desk` が真なら窓口も据える）。
fn wired_world(desk: bool) -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    if desk {
        crate::install::register(&mut world);
    }
    wire_menu(&mut world, mpsc::channel().0);
    register(&mut world);
    world
}

/// メニューを出したときの「インストール」枠の項目（登記が無ければ None）。
fn install_item(world: &World) -> Option<MenuItem> {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(frame, item)| (frame == Frame::Install).then_some(item))
}

/// 項目の（既定名・リソース名・選べるか・印・葉か）。
fn shape(item: &MenuItem) -> (String, Option<&'static str>, bool, Option<bool>, bool) {
    (
        item.label.clone(),
        item.caption_resource,
        item.enabled,
        item.checked,
        matches!(item.body, ItemBody::Action(_)),
    )
}

/// 窓口が在れば、既定名「インストール…」・`ghostinstallbutton.caption`・選べる・印なしの葉
/// （要件 1.3・1.8）。
#[test]
fn item_is_a_selectable_leaf_with_the_default_label_and_caption() {
    let world = wired_world(true);
    assert_eq!(
        install_item(&world).as_ref().map(shape),
        Some((
            "インストール…".to_owned(),
            Some("ghostinstallbutton.caption"),
            true,
            None,
            true,
        ))
    );
}

/// 窓口が無い（系の登録の前）・終了が始まった後は選べない（枠は出る）。
#[test]
fn item_is_disabled_without_a_desk_or_after_closing_began() {
    let without_desk = wired_world(false);
    let mut closing = wired_world(true);
    let _ = begin_close(&mut closing);
    assert_eq!(
        (
            install_item(&without_desk).map(|i| i.enabled),
            install_item(&closing).map(|i| i.enabled),
        ),
        (Some(false), Some(false))
    );
}

/// 結線をやり直すと登記は新品で、「インストール」枠は置き換えの記録なしに登記し直される
/// （要件 1.8）。
#[test]
fn rewiring_gives_a_fresh_registration_with_the_install_frame() {
    let mut world = wired_world(true);
    let ((), events) = capture(|| {
        wire_menu(&mut world, mpsc::channel().0);
        register(&mut world);
    });
    let replaced = events
        .iter()
        .filter(|e| e.field_str("event") == Some("menu_registration_replaced"))
        .count();
    assert_eq!(
        (
            world.non_send::<MenuWiring>().registry.registered_frames(),
            replaced,
            install_item(&world).map(|i| i.enabled),
        ),
        (
            vec![Frame::Install, Frame::Readme, Frame::Close],
            0,
            Some(true)
        ),
        "{events:?}"
    );
}
