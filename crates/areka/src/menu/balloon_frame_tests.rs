//! 「バルーン」枠の決定論テスト（areka-P0-shell-balloon-switch task 10・要件 7.2・7.4）。
//!
//! 確かめること: 根の目録の並びのまま子が並び、ラベルは `name`（無ければフォルダ名）で、今の
//! バルーン（起動の文脈の `current.balloon.folder`）だけに印が付くこと・1 つだけでも枠が出ること・
//! 起動の文脈が無ければ無効の枠だけが出ること。選ぶと入口が要求を受けることは、実行系を起こす
//! `shell_balloon_switch_session_tests.rs` が確かめる。
//! ゴーストの中のバルーンが根より先に並び、使っているときにそこへ印が付くことも確かめる
//! （areka-P0-ghost-inner-balloon 要件 2.1・2.4・2.5）。

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use temp_path_kit::TempPath;

use super::*;
use crate::menu::ghost_frame::tests::{FrameShape, boot_context, shape};
use crate::menu::{Frame, MenuContext, MenuWiring, wire_menu};

/// 根の `balloon/<folder>/descript.txt` を書く。
fn put_balloon(root: &BasewareRoot, folder: &str, body: &str) {
    let dir = root.balloon_dir(folder);
    std::fs::create_dir_all(&dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), format!("charset,UTF-8\n{body}")).expect("descript");
}

/// メニューを結線し「バルーン」枠を登記した World（今のバルーンは `StayseeBalloon`）。
fn wired_world(root: &BasewareRoot, with_context: bool) -> World {
    let mut world = World::new();
    if with_context {
        world.insert_resource(boot_context(root, "A"));
    }
    wire_menu(&mut world, std::sync::mpsc::channel().0);
    register(&mut world);
    world
}

fn balloon_shape(world: &World) -> Option<FrameShape> {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(f, item)| (f == Frame::Balloon).then(|| shape(&item)))
}

/// 目録の並び（フォルダ名の昇順）のまま子が並び、ラベルは `name`（無ければフォルダ名）、今の
/// バルーン（先頭でない方）だけに印（要件 7.2・7.4）。
#[test]
fn balloons_listed_in_catalog_order_with_current_mark() {
    let tmp = TempPath::under_target("balloon-frame-list");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_balloon(&root, "Alpha", "name,アルファ\n");
    put_balloon(&root, "StayseeBalloon", "");
    let world = wired_world(&root, true);

    assert_eq!(
        balloon_shape(&world),
        Some((
            "バルーン".to_owned(),
            Some("balloonrootbutton.caption"),
            true,
            vec![
                ("アルファ".to_owned(), true, Some(false)),
                ("StayseeBalloon".to_owned(), true, Some(true)),
            ],
        ))
    );
}

/// バルーンが 1 つ（今のもの）だけでも枠は出る（要件 7.2）。
#[test]
fn a_single_balloon_still_shows_the_frame() {
    let tmp = TempPath::under_target("balloon-frame-single");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_balloon(&root, "StayseeBalloon", "name,ステイシー\n");
    let world = wired_world(&root, true);

    assert_eq!(
        balloon_shape(&world),
        Some((
            "バルーン".to_owned(),
            Some("balloonrootbutton.caption"),
            true,
            vec![("ステイシー".to_owned(), true, Some(true))],
        ))
    );
}

/// 起動の文脈が無ければ無効の枠だけが出る。
#[test]
fn without_boot_context_only_a_disabled_balloon_frame_is_shown() {
    let tmp = TempPath::under_target("balloon-frame-nocontext");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_balloon(&root, "StayseeBalloon", "name,ステイシー\n");
    let world = wired_world(&root, false);

    assert_eq!(
        balloon_shape(&world),
        Some((
            "バルーン".to_owned(),
            Some("balloonrootbutton.caption"),
            false,
            vec![],
        ))
    );
}

/// 2 つの根の和: 両方の根のバルーンが隠れを除いてフォルダ名のバイト順に並び、今のバルーン
/// （後ろの根の `StayseeBalloon`）と同じフォルダ名として先の根の `stayseeballoon` に印が付く
/// （areka-P0-baseware-root-list 4.3・要件 2.8・3.5）。
#[test]
fn union_of_two_roots_marks_same_folder_name_balloon() {
    let first = TempPath::under_target("balloon-frame-union-first");
    let second = TempPath::under_target("balloon-frame-union-second");
    let first_root = BasewareRoot::new(first.path().to_path_buf());
    let second_root = BasewareRoot::new(second.path().to_path_buf());
    put_balloon(&first_root, "stayseeballoon", "");
    put_balloon(&second_root, "StayseeBalloon", "");
    put_balloon(&second_root, "Alpha", "");
    let mut ctx = boot_context(&second_root, "A");
    ctx.roots = areka_ghost::BasewareRoots::new(vec![first_root, second_root]);
    let mut world = World::new();
    world.insert_resource(ctx);
    wire_menu(&mut world, std::sync::mpsc::channel().0);
    register(&mut world);

    assert_eq!(
        balloon_shape(&world).map(|s| s.3),
        Some(vec![
            ("Alpha".to_owned(), true, Some(false)),
            ("stayseeballoon".to_owned(), true, Some(true)),
        ])
    );
}

/// 今のゴーストの `balloon/<folder>/descript.txt` を書く。
fn put_ghost_balloon(root: &BasewareRoot, ghost: &str, folder: &str, body: &str) {
    let dir = root.ghost_dir(ghost).join("balloon").join(folder);
    std::fs::create_dir_all(&dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), format!("charset,UTF-8\n{body}")).expect("descript");
}

/// ゴーストの中のバルーンが先・根のバルーンが後に 1 つの一覧で並ぶ（要件 2.1・2.4・2.5）。
#[test]
fn ghost_inner_balloons_listed_before_root_balloons() {
    let tmp = TempPath::under_target("balloon-frame-ghost-inner-first");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_balloon(&root, "Alpha", "");
    put_balloon(&root, "StayseeBalloon", "");
    put_ghost_balloon(&root, "A", "Zeta", "name,ゼータ\n");
    let world = wired_world(&root, true);

    assert_eq!(
        balloon_shape(&world).map(|s| s.3),
        Some(vec![
            ("ゼータ".to_owned(), true, Some(false)),
            ("Alpha".to_owned(), true, Some(false)),
            ("StayseeBalloon".to_owned(), true, Some(true)),
        ])
    );
}

/// ゴーストの中のバルーンを使っているとき、そこへ今と同じ印が付く（要件 2.4）。
#[test]
fn current_ghost_inner_balloon_gets_the_mark() {
    let tmp = TempPath::under_target("balloon-frame-ghost-inner-mark");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_balloon(&root, "Alpha", "");
    put_ghost_balloon(&root, "A", "Inner", "name,内側\n");
    let mut ctx = boot_context(&root, "A");
    ctx.current.balloon.dir = root.ghost_dir("A").join("balloon").join("Inner");
    ctx.current.balloon.folder = Some("Inner".to_owned());
    let mut world = World::new();
    world.insert_resource(ctx);
    wire_menu(&mut world, std::sync::mpsc::channel().0);
    register(&mut world);

    assert_eq!(
        balloon_shape(&world).map(|s| s.3),
        Some(vec![
            ("内側".to_owned(), true, Some(true)),
            ("Alpha".to_owned(), true, Some(false)),
        ])
    );
}
