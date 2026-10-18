//! 「置き場を追加…」「置き場を先頭へ ▸」「置き場を外す ▸」枠の決定論テスト（要件 6.1・7.1・7.3・8.2・8.6・12.5／areka-P0-user-data-root 要件 3.3・3.8・10.3）。
//! 「置き場を先頭へ ▸」の子の選べる・選べないの各分岐（先頭・印の付いた並びの実行ファイルの隣・見出しの根の数と `AREKA_ROOT`）も確かめる。
//!
//! 確かめること: 3 項目が出ること・外すの子が並びの順の絶対パスで見つからない根も含むこと・
//! 根が 1 つのとき子が選べないこと・書ける根（印の付いた項目）の子が選べないこと・`AREKA_ROOT` で渡しているとき両方が選べないこと。
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
    wired_world_roots(
        dirs.iter().cloned().map(BasewareRoot::new).collect(),
        origin,
    )
}

/// 印つきの項目を含む並び `roots` で [`wired_world`] と同じ World を作る。
pub(super) fn wired_world_roots(roots: Vec<BasewareRoot>, origin: RootsOrigin) -> World {
    let a = roots[0].dir().to_path_buf();
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(BootContext {
        roots: BasewareRoots::new(roots),
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
pub(super) fn item(world: &World, frame: Frame) -> MenuItem {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(f, item)| (f == frame).then_some(item))
        .expect("枠が登記されている")
}

/// 外すの子の（名前・選べるか）。
pub(super) fn children(item: &MenuItem) -> Vec<(String, bool)> {
    match &item.body {
        ItemBody::Submenu(children) => children
            .iter()
            .map(|c| (c.label.clone(), c.enabled))
            .collect(),
        ItemBody::Action(_) => panic!("外すはサブメニュー"),
    }
}

pub(super) fn label(dir: &Path) -> String {
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

/// 書ける根（印の付いた項目）の子は選べず、ほかの子は今どおり選べる
/// （areka-P0-user-data-root 要件 3.3・10.3）。
#[test]
fn user_root_child_is_disabled_others_unchanged() {
    let tmp = TempPath::under_target("roots_frame_user_root");
    let user = tmp.path().join("user");
    let exe = tmp.path().join("exe");
    let extra = tmp.path().join("extra");
    let world = wired_world_roots(
        vec![
            BasewareRoot::user_root(user.clone(), false),
            BasewareRoot::new(exe.clone()),
            BasewareRoot::new(extra.clone()),
        ],
        RootsOrigin::Memory,
    );
    let remove = item(&world, Frame::RootRemove);
    assert_eq!(
        (remove.enabled, children(&remove)),
        (
            true,
            vec![
                (label(&user), false),
                (label(&exe), true),
                (label(&extra), true)
            ]
        )
    );
}

/// 実行ファイルのフォルダ（「置き場を先頭へ ▸」が印の付いた並びで選べなくする子）。
fn exe_dir() -> PathBuf {
    crate::roots::user_root::exe_dir().expect("テストの実行ファイルのフォルダ")
}

/// 書ける根を使わない形: 「置き場を先頭へ ▸」は既定名で SHIORI のリソース名なしに出て、見出しは選べ、
/// 子は並びの順の絶対パスで、今の先頭だけ選べない（印が無いので実行ファイルの隣の子も選べる）
/// （areka-P0-user-data-root 要件 3.8・7.1・10.3）。
#[test]
fn front_lists_every_root_and_disables_only_the_head_without_mark() {
    let tmp = TempPath::under_target("roots_frame_front_plain");
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    let exe = exe_dir();
    let world = wired_world(&[a.clone(), exe.clone(), b.clone()], RootsOrigin::Memory);
    let front = item(&world, Frame::RootFront);
    assert_eq!(
        (
            front.label.as_str(),
            front.caption_resource,
            front.enabled,
            children(&front),
        ),
        (
            "置き場を先頭へ",
            None,
            true,
            vec![(label(&a), false), (label(&exe), true), (label(&b), true)],
        )
    );
}

/// 書ける根を使う形（印の付いた項目が先頭）: 先頭（書ける根）と実行ファイルの隣の子は選べず、
/// ほかの子は選べる（要件 3.8）。
#[test]
fn front_with_user_root_head_disables_head_and_exe_child() {
    let tmp = TempPath::under_target("roots_frame_front_user_head");
    let user = tmp.path().join("user");
    let extra = tmp.path().join("extra");
    let exe = exe_dir();
    let world = wired_world_roots(
        vec![
            BasewareRoot::user_root(user.clone(), false),
            BasewareRoot::new(exe.clone()),
            BasewareRoot::new(extra.clone()),
        ],
        RootsOrigin::Memory,
    );
    let front = item(&world, Frame::RootFront);
    assert_eq!(
        (front.enabled, children(&front)),
        (
            true,
            vec![
                (label(&user), false),
                (label(&exe), false),
                (label(&extra), true)
            ]
        )
    );
}

/// 印の付いた項目が先頭でないとき: 先頭（足した置き場）と実行ファイルの隣の子は選べず、
/// 書ける根の子は選べる（要件 3.8）。
#[test]
fn front_with_user_root_not_head_keeps_user_root_selectable() {
    let tmp = TempPath::under_target("roots_frame_front_user_mid");
    let user = tmp.path().join("user");
    let extra = tmp.path().join("extra");
    let exe = exe_dir();
    let world = wired_world_roots(
        vec![
            BasewareRoot::new(extra.clone()),
            BasewareRoot::user_root(user.clone(), false),
            BasewareRoot::new(exe.clone()),
        ],
        RootsOrigin::Memory,
    );
    let front = item(&world, Frame::RootFront);
    assert_eq!(
        (front.enabled, children(&front)),
        (
            true,
            vec![
                (label(&extra), false),
                (label(&user), true),
                (label(&exe), false)
            ]
        )
    );
}

/// 根が 1 つなら見出しは選べない（子は先頭なので選べない・要件 3.8）。
#[test]
fn front_header_disabled_with_single_root() {
    let tmp = TempPath::under_target("roots_frame_front_single");
    let a = tmp.path().join("a");
    let world = wired_world(std::slice::from_ref(&a), RootsOrigin::Default);
    let front = item(&world, Frame::RootFront);
    assert_eq!(
        (front.enabled, children(&front)),
        (false, vec![(label(&a), false)])
    );
}

/// `AREKA_ROOT` で渡しているときは根が 2 つ以上でも見出しは選べない（要件 3.8）。
#[test]
fn front_header_disabled_with_env_var() {
    let tmp = TempPath::under_target("roots_frame_front_env");
    let dirs = [tmp.path().join("a"), tmp.path().join("b")];
    let world = wired_world(&dirs, RootsOrigin::EnvVar);
    assert!(!item(&world, Frame::RootFront).enabled);
}
