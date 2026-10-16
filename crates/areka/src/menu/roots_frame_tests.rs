//! 「置き場を追加…」「置き場を外す ▸」枠の決定論テスト（要件 6.1・7.1・7.3・8.2・8.6・12.5）。
//!
//! 確かめること: 2 項目が出ること・外すの子が並びの順の絶対パスで見つからない根も含むこと・
//! 根が 1 つのとき子が選べないこと・`AREKA_ROOT` で渡しているとき両方が選べないこと。
//! 項目の動作は呼ばない（追加は本物の選ぶ画面が出る・外すの判断は `roots/edit_tests.rs` が確かめる）。

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use areka_ghost::{BasewareRoot, BasewareRoots};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use temp_path_kit::TempPath;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::menu::{MenuWiring, wire_menu};
use crate::roots::RootsOrigin;

/// 並び `dirs`・出どころ `origin` の起動の文脈と置き場の受け口を据え、2 枠を登記した World。
fn wired_world(dirs: &[PathBuf], origin: RootsOrigin) -> World {
    let a = dirs[0].clone();
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(BootContext {
        roots: BasewareRoots::new(dirs.iter().cloned().map(BasewareRoot::new).collect()),
        roots_origin: origin,
        app_profile_dir: a.join("profile"),
        helper_exe: a.join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: a.join(r"ghost\ga"),
                balloon_root: a.join(r"balloon\B"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: a.join(r"ghost\ga"),
                folder: Some("ga".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: a.join(r"balloon\B"),
                folder: Some("B".to_owned()),
            },
        },
    });
    crate::roots::install(&mut world);
    wire_menu(&mut world, mpsc::channel().0);
    register(&mut world);
    world
}

/// メニューを出したときの枠 `frame` の項目。
fn item(world: &World, frame: Frame) -> MenuItem {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(f, item)| (f == frame).then_some(item))
        .expect("枠が登記されている")
}

/// 外すの子の（名前・選べるか）。
fn children(item: &MenuItem) -> Vec<(String, bool)> {
    match &item.body {
        ItemBody::Submenu(children) => children
            .iter()
            .map(|c| (c.label.clone(), c.enabled))
            .collect(),
        ItemBody::Action(_) => panic!("外すはサブメニュー"),
    }
}

fn label(dir: &Path) -> String {
    dir.display().to_string()
}

/// 2 項目が既定名で出て、追加は選べる葉・外すは選べる見出しで、子は並びの順の絶対パス
/// （見つからない根も含む）で選べる（要件 6.1・7.1・8.2）。
#[test]
fn both_items_appear_and_children_list_every_root_in_order() {
    let tmp = TempPath::under_target("roots_frame_two_items");
    let a = tmp.path().join("a");
    let missing = tmp.path().join("missing");
    let c = tmp.path().join("c");
    for dir in [&a, &c] {
        std::fs::create_dir_all(dir.join("ghost")).unwrap();
    }
    let world = wired_world(
        &[c.clone(), missing.clone(), a.clone()],
        RootsOrigin::Memory,
    );
    let add = item(&world, Frame::RootAdd);
    let remove = item(&world, Frame::RootRemove);
    assert_eq!(
        (
            add.label.as_str(),
            add.caption_resource,
            add.enabled,
            matches!(add.body, ItemBody::Action(_)),
            remove.label.as_str(),
            remove.caption_resource,
            remove.enabled,
            children(&remove),
        ),
        (
            "置き場を追加…",
            None,
            true,
            true,
            "置き場を外す",
            None,
            true,
            vec![
                (label(&c), true),
                (label(&missing), true),
                (label(&a), true)
            ],
        )
    );
}

/// 根が 1 つなら外すの子は選べない（見出しは選べる・要件 7.3）。
#[test]
fn single_root_child_is_disabled() {
    let tmp = TempPath::under_target("roots_frame_single");
    let a = tmp.path().join("a");
    let world = wired_world(std::slice::from_ref(&a), RootsOrigin::Default);
    let remove = item(&world, Frame::RootRemove);
    assert_eq!(
        (remove.enabled, children(&remove)),
        (true, vec![(label(&a), false)])
    );
}

/// `AREKA_ROOT` で渡しているときは 2 項目とも選べない（要件 8.6）。
#[test]
fn env_var_disables_both_items() {
    let tmp = TempPath::under_target("roots_frame_env");
    let dirs = [tmp.path().join("a"), tmp.path().join("b")];
    let world = wired_world(&dirs, RootsOrigin::EnvVar);
    assert_eq!(
        (
            item(&world, Frame::RootAdd).enabled,
            item(&world, Frame::RootRemove).enabled,
        ),
        (false, false)
    );
}
