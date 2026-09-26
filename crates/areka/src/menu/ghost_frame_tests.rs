//! 「ゴースト」枠の決定論テスト（areka-P0-ghost-shell-balloon-switch task 7.3・要件 1.4・1.11・1.12・10.9）。
//!
//! 確かめること: 目録の並びのまま子が並び、ラベルは `name`（無ければフォルダ名）で `sakura.name`
//! ではないこと・今のゴーストだけに印が付くこと・メニューを出すたびに目録を読み直すこと・
//! 0 体でも枠が出ること・起動の文脈が無ければ無効の枠だけが出ること・子を選ぶとフォルダ名で指す
//! 「手動」の切替要求が入口を通って kanade へ届くこと・結線をやり直すと登記が新品になること。
//! 実行系は起こさない（置き場には kanade の送出端だけを持つ中身を据え、受信端で送出を数える）。
//! 本番の結線（`ghost_session::boot_wired` が `wire_menu` の直後に登記する）は
//! `ghost_session_restart_tests.rs` の 2 周の登記の一覧が確かめる。

use std::sync::mpsc::{self, Receiver};

use areka_ghost::BasewareRoot;
use areka_kanade::{ChangeOrigin, KanadeMsg};
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::menu::{MenuAction, MenuWiring, wire_menu};

// ---------------------------------------------------------------- 道具立て

/// 根の `ghost/<folder>/ghost/master/descript.txt` を書く。
fn put_ghost(tmp: &TempPath, folder: &str, descript: &str) {
    let master = tmp.child("ghost").join(folder).join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(master.join("descript.txt"), descript).expect("descript");
}

/// 根に 2 体: `A`（`name`＝Alice・`sakura.name`＝さくら）と `B`（`name` も `sakura.name` も無い）。
fn fixture_root(tmp: &TempPath) -> BasewareRoot {
    put_ghost(tmp, "A", "charset,UTF-8\nname,Alice\nsakura.name,さくら\n");
    put_ghost(tmp, "B", "charset,UTF-8\n");
    BasewareRoot::new(tmp.path().to_path_buf())
}

/// 今のゴーストを `current` のフォルダとする起動の文脈。
fn boot_context(root: &BasewareRoot, current: &str) -> BootContext {
    BootContext {
        root: root.clone(),
        app_profile_dir: root.dir().join("profile"),
        helper_exe: root.dir().join("helper.exe"),
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir(current),
                balloon_root: root.balloon_dir("StayseeBalloon"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir(current),
                folder: Some(current.to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("StayseeBalloon"),
                folder: Some("StayseeBalloon".to_owned()),
            },
        },
    }
}

/// メニューを結線し「ゴースト」枠を登記した World と kanade の受信端。文脈は `ctx` があれば据え、
/// 置き場には kanade の送出端だけを持つ中身（今のゴーストの根）を据える。
fn wired_world(ctx: Option<BootContext>) -> (World, Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    if let Some(ctx) = ctx {
        world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
            Some(tx.clone()),
            ctx.current.cfg.ghost_root.clone(),
        ))));
        world.insert_resource(ctx);
    }
    wire_menu(&mut world, tx);
    register(&mut world);
    (world, rx)
}

/// メニューを出したときの「ゴースト」枠の項目（登記が無ければ `None`）。
fn ghost_item(world: &World) -> Option<MenuItem> {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(frame, item)| (frame == Frame::Ghost).then_some(item))
}

/// 枠の見出し（既定名・リソース名・有効）と子の（ラベル・有効・印）の列。
type FrameShape = (
    String,
    Option<&'static str>,
    bool,
    Vec<(String, bool, Option<bool>)>,
);

fn shape(item: &MenuItem) -> FrameShape {
    let children = match &item.body {
        ItemBody::Submenu(children) => children
            .iter()
            .map(|c| (c.label.clone(), c.enabled, c.checked))
            .collect(),
        ItemBody::Action(_) => panic!("「ゴースト」枠は子を持つ見出しのはず"),
    };
    (
        item.label.clone(),
        item.caption_resource,
        item.enabled,
        children,
    )
}

/// ラベル `label` の子の動作。
fn child_action(item: &MenuItem, label: &str) -> MenuAction {
    let ItemBody::Submenu(children) = &item.body else {
        panic!("「ゴースト」枠は子を持つ見出しのはず");
    };
    let child = children
        .iter()
        .find(|c| c.label == label)
        .unwrap_or_else(|| panic!("子 {label} が無い"));
    match &child.body {
        ItemBody::Action(action) => action.clone(),
        ItemBody::Submenu(_) => panic!("子は葉のはず"),
    }
}

// ---------------------------------------------------------------- 列挙

/// 2 体の目録で子 2 つ・目録の並び・ラベルは `name`（無ければフォルダ名・`sakura.name` ではない）・
/// 今のゴースト（`B`＝先頭でない方）だけに印（要件 1.11）。
#[test]
fn two_ghosts_listed_in_catalog_order_with_current_mark() {
    let tmp = TempPath::new("ghost-frame-two");
    let root = fixture_root(&tmp);
    let (world, _rx) = wired_world(Some(boot_context(&root, "B")));

    let item = ghost_item(&world).expect("「ゴースト」枠が登記されている");
    assert_eq!(
        shape(&item),
        (
            "ゴースト".to_owned(),
            Some("ghostrootbutton.caption"),
            true,
            vec![
                ("Alice".to_owned(), true, Some(false)),
                ("B".to_owned(), true, Some(true)),
            ],
        )
    );
}

/// メニューを出すたびに目録を読み直す: 登記の後に足したゴーストも次の表示に出る。
#[test]
fn catalog_is_read_every_time_the_menu_is_shown() {
    let tmp = TempPath::new("ghost-frame-reread");
    let root = fixture_root(&tmp);
    let (world, _rx) = wired_world(Some(boot_context(&root, "A")));
    let first = ghost_item(&world).map(|i| shape(&i).3.len());

    put_ghost(&tmp, "C", "charset,UTF-8\nname,Carol\n");
    let second = ghost_item(&world).map(|i| shape(&i).3);

    assert_eq!(
        (first, second),
        (
            Some(2),
            Some(vec![
                ("Alice".to_owned(), true, Some(true)),
                ("B".to_owned(), true, Some(false)),
                ("Carol".to_owned(), true, Some(false)),
            ])
        )
    );
}

/// 0 体でも枠は出る（子 0・選べない見出し）。
#[test]
fn empty_catalog_still_shows_the_frame() {
    let tmp = TempPath::new("ghost-frame-empty");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let (world, _rx) = wired_world(Some(boot_context(&root, "A")));

    assert_eq!(
        ghost_item(&world).map(|i| shape(&i)),
        Some((
            "ゴースト".to_owned(),
            Some("ghostrootbutton.caption"),
            false,
            vec![]
        ))
    );
}

/// 起動の文脈が無ければ無効の枠だけが出る。
#[test]
fn without_boot_context_only_a_disabled_frame_is_shown() {
    let (world, _rx) = wired_world(None);

    assert_eq!(
        ghost_item(&world).map(|i| shape(&i)),
        Some((
            "ゴースト".to_owned(),
            Some("ghostrootbutton.caption"),
            false,
            vec![]
        ))
    );
}

// ---------------------------------------------------------------- 選択

/// 子を選ぶと、フォルダ名で指し `OnGhostChanging` を送る「手動」の要求が入口を通って kanade へ
/// 届き、切替の予約が立つ（要件 1.4）。
#[test]
fn selecting_a_child_sends_a_manual_request_through_the_entrance() {
    let tmp = TempPath::new("ghost-frame-select");
    let root = fixture_root(&tmp);
    let (mut world, rx) = wired_world(Some(boot_context(&root, "A")));
    let action = child_action(&ghost_item(&world).expect("枠"), "B");

    let ((), events) = capture(|| action(&mut world, &MenuContext { scope: 0 }));

    let sent: Vec<_> = rx
        .try_iter()
        .map(|m| match m {
            KanadeMsg::ChangeGhost(req) => (req.target.name, req.origin, req.raise_event),
            _ => panic!("切替の要求以外が送られた"),
        })
        .collect();
    let reserved = world
        .get_non_send::<SwitchInFlight>()
        .map(|r| r.target.folder.clone());
    assert_eq!(
        (sent, reserved),
        (
            vec![("B".to_owned(), ChangeOrigin::Manual, true)],
            Some("B".to_owned())
        ),
        "{events:?}"
    );
}

// ---------------------------------------------------------------- 登記のやり直し

/// 結線をやり直すと登記は新品: 1 周目の余分な登記は消え、「ゴースト」枠は置き換えの記録なしに
/// 登記し直される（要件 1.12）。
#[test]
fn rewiring_gives_a_fresh_registration_with_the_ghost_frame() {
    let tmp = TempPath::new("ghost-frame-rewire");
    let root = fixture_root(&tmp);
    let (mut world, _rx) = wired_world(Some(boot_context(&root, "A")));
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

    let (tx2, _rx2) = mpsc::channel();
    let ((), events) = capture(|| {
        wire_menu(&mut world, tx2);
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
            ghost_item(&world).map(|i| shape(&i).3.len()),
        ),
        (vec![Frame::Ghost, Frame::Readme, Frame::Close], 0, Some(2)),
        "{events:?}"
    );
}
