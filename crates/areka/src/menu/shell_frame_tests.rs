//! 「シェル」枠の決定論テスト（areka-P0-shell-balloon-switch task 10・要件 7.1・7.2・7.4・11.9）。
//!
//! 確かめること: 目録（隠しを除く列挙）の並びのまま子が並び、ラベルは `name`（無ければフォルダ名）で
//! あること・メニューを出すたびに目録を読み直すこと・1 つだけでも枠が出ること・置き場のゴーストが
//! 無ければ無効の枠だけが出ること・結線をやり直すと「シェル」「バルーン」の登記が新品になること。
//! 実行系は起こさない（置き場の中身はテスト用の組み立て＝今のシェルは分からず印は付かない）。
//! 今のシェルの印と、選ぶと入口が要求を受けることは、実行系を起こす
//! `shell_balloon_switch_session_tests.rs` が確かめる。

use std::path::Path;
use std::rc::Rc;

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::menu::ghost_frame::tests::{FrameShape, boot_context, shape};
use crate::menu::{Frame, ItemBody, MenuContext, MenuItem, MenuWiring, wire_menu};

// ---------------------------------------------------------------- 道具立て

/// `<dir>/descript.txt` を書く。
fn put_descript(dir: &Path, body: &str) {
    std::fs::create_dir_all(dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), format!("charset,UTF-8\n{body}")).expect("descript");
}

/// ゴースト `A` のシェル `<folder>` を書く。
fn put_shell(root: &BasewareRoot, folder: &str, body: &str) {
    put_descript(&root.ghost_dir("A").join("shell").join(folder), body);
}

/// メニューを結線し 2 枠を登記した World。`with_ghost` なら置き場にゴースト `A`（実行系なし）を据える。
fn wired_world(root: &BasewareRoot, with_ghost: bool) -> World {
    let mut world = World::new();
    if with_ghost {
        world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
            None,
            root.ghost_dir("A"),
        ))));
        world.insert_resource(boot_context(root, "A"));
    }
    wire_menu(&mut world, std::sync::mpsc::channel().0);
    register(&mut world);
    crate::menu::balloon_frame::register(&mut world);
    world
}

/// メニューを出したときの `frame` 枠の項目（登記が無ければ `None`）。
fn frame_item(world: &World, frame: Frame) -> Option<MenuItem> {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(f, item)| (f == frame).then_some(item))
}

fn shell_shape(world: &World) -> Option<FrameShape> {
    frame_item(world, Frame::Shell).map(|i| shape(&i))
}

// ---------------------------------------------------------------- 列挙

/// 目録の並び（フォルダ名の昇順）のまま、隠しを除いて子が並ぶ。ラベルは `name`（無ければ
/// フォルダ名）。今のシェルが分からなければ印はどれにも付かない（要件 7.1・7.4）。
#[test]
fn shells_listed_in_catalog_order_without_hidden() {
    let tmp = TempPath::new("shell-frame-list");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_shell(&root, "master", "name,通常\n");
    put_shell(&root, "secret", "name,隠し\nmenu,hidden\n");
    put_shell(&root, "summer", "");
    let world = wired_world(&root, true);

    assert_eq!(
        shell_shape(&world),
        Some((
            "シェル".to_owned(),
            Some("shellrootbutton.caption"),
            true,
            vec![
                ("通常".to_owned(), true, Some(false)),
                ("summer".to_owned(), true, Some(false)),
            ],
        ))
    );
}

/// シェルが 1 つだけでも枠は出て、子を 1 つ選べる（要件 7.1）。
#[test]
fn a_single_shell_still_shows_the_frame() {
    let tmp = TempPath::new("shell-frame-single");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_shell(&root, "master", "name,通常\n");
    let world = wired_world(&root, true);

    assert_eq!(
        shell_shape(&world),
        Some((
            "シェル".to_owned(),
            Some("shellrootbutton.caption"),
            true,
            vec![("通常".to_owned(), true, Some(false))],
        ))
    );
}

/// メニューを出すたびに目録を読み直す: 登記の後に足したシェルも次の表示に出る。
#[test]
fn shell_catalog_is_read_every_time_the_menu_is_shown() {
    let tmp = TempPath::new("shell-frame-reread");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_shell(&root, "master", "name,通常\n");
    let world = wired_world(&root, true);
    let first = shell_shape(&world).map(|s| s.3.len());

    put_shell(&root, "winter", "name,冬服\n");
    let second = shell_shape(&world).map(|s| s.3);

    assert_eq!(
        (first, second),
        (
            Some(1),
            Some(vec![
                ("通常".to_owned(), true, Some(false)),
                ("冬服".to_owned(), true, Some(false)),
            ])
        )
    );
}

/// 置き場のゴーストが無ければ無効の枠だけが出る。
#[test]
fn without_ghost_only_a_disabled_shell_frame_is_shown() {
    let tmp = TempPath::new("shell-frame-noghost");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_shell(&root, "master", "name,通常\n");
    let world = wired_world(&root, false);

    assert_eq!(
        shell_shape(&world),
        Some((
            "シェル".to_owned(),
            Some("shellrootbutton.caption"),
            false,
            vec![],
        ))
    );
}

// ---------------------------------------------------------------- 登記のやり直し

/// 結線をやり直すと登記は新品: 1 周目の余分な登記は消え、「シェル」「バルーン」枠は置き換えの
/// 記録なしに登記し直される（要件 11.9）。
#[test]
fn rewiring_gives_a_fresh_registration_with_shell_and_balloon_frames() {
    let tmp = TempPath::new("shell-frame-rewire");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_shell(&root, "master", "name,通常\n");
    let mut world = wired_world(&root, true);
    crate::menu::register(
        &mut world,
        Frame::Shell,
        Rc::new(|_: &World, _: &MenuContext| MenuItem {
            label: "1 周目の余分な登記".to_owned(),
            caption_resource: None,
            enabled: true,
            checked: None,
            body: ItemBody::Action(Rc::new(|_: &mut World, _: &MenuContext| {})),
        }),
    );

    let ((), events) = capture(|| {
        wire_menu(&mut world, std::sync::mpsc::channel().0);
        register(&mut world);
        crate::menu::balloon_frame::register(&mut world);
    });

    let replaced = events
        .iter()
        .filter(|e| e.field_str("event") == Some("menu_registration_replaced"))
        .count();
    assert_eq!(
        (
            world.non_send::<MenuWiring>().registry.registered_frames(),
            replaced,
            shell_shape(&world).map(|s| s.0),
        ),
        (
            vec![Frame::Shell, Frame::Balloon, Frame::Readme, Frame::Close],
            0,
            Some("シェル".to_owned())
        ),
        "{events:?}"
    );
}
