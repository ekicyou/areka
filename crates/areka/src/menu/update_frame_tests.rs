//! 「ネットワーク更新」枠の決定論テスト（areka-P0-network-update・要件 1.2・1.3・1.4・9.5）。
//!
//! 確かめること: 枠の項目は既定名「ネットワーク更新」と `updatebutton.caption` を持つ葉であること・
//! 3 つとも更新先が無ければ灰色（1 つに在れば選べる）・走っている間は灰色・選ぶと理由 `manual` の
//! 依頼がちょうど 1 件受付を通ること・結線をやり直すと枠が置き換えの記録なしに登記し直されること。
//! 本番の結線（`ghost_session::boot_wired` が起こすたびに登記する）は
//! `ghost_session_restart_tests.rs`・`ghost_session_switch_tests.rs` の登記の一覧が確かめる。
//!
//! kanade の受信端はすぐ落とす: 受けた依頼で起きる背景スレッドは最初の送出で「kanade が居ない」として
//! 依頼をやめる（取得へ進まない＝ネットへ出ない）。受付の判定は UI スレッドの記録で見る。

use std::sync::mpsc;

use areka_ghost::BasewareRoot;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::menu::{Frame, ItemBody, MenuContext, MenuItem, MenuWiring, wire_menu};
use crate::update::{self, RawUpdateRequest, SubmitVerdict, TargetKind, UpdateReason};

const GHOST_URL: &str = "https://ghost.example/emo/";

/// 窓口・起動の文脈・置き場のゴースト（`emo`・名前 `Emo`・`homeurl` は引数）を置き、メニューを結線して
/// 「ネットワーク更新」枠を登記した World。バルーン `kaku` は `descript.txt` が無く、置き場は実行系を
/// 持たないのでシェルは解けない＝更新先はゴーストの `homeurl` だけ。
fn wired_world(label: &str, ghost_homeurl: Option<&str>) -> (World, TempPath) {
    let tmp = TempPath::new(label);
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let master = root.ghost_dir("emo").join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("検体のフォルダを作る");
    let homeurl = ghost_homeurl.map_or(String::new(), |url| format!("homeurl,{url}\r\n"));
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\r\nname,Emo\r\n{homeurl}"),
    )
    .expect("descript.txt を書く");
    let mut world = World::new();
    world.init_resource::<Schedules>();
    update::register(&mut world);
    world.insert_resource(BootContext {
        root: root.clone(),
        app_profile_dir: tmp.path().join("profile"),
        helper_exe: tmp.path().join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("emo"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("emo"),
                folder: Some("emo".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    });
    let (kanade, _) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(kanade),
        root.ghost_dir("emo"),
    ))));
    wire_menu(&mut world, mpsc::channel().0);
    register(&mut world);
    (world, tmp)
}

/// メニューを出したときの「ネットワーク更新」枠の項目（登記が無ければ None）。
fn update_item(world: &World) -> Option<MenuItem> {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(frame, item)| (frame == Frame::Update).then_some(item))
}

/// 項目を選ぶ（メニューを閉じた後の動作を 1 回呼ぶ）。
fn select(world: &mut World) {
    let item = update_item(world).expect("「ネットワーク更新」枠が登記されている");
    let ItemBody::Action(action) = item.body else {
        panic!("葉のはず");
    };
    action(world, &MenuContext { scope: 0 });
}

fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 更新先が在れば、既定名「ネットワーク更新」・`updatebutton.caption`・選べる・印なしの葉（要件 1.2・1.3）。
#[test]
fn item_is_a_selectable_leaf_with_the_default_label_and_caption() {
    let (world, _tmp) = wired_world("areka-update-frame-shape", Some(GHOST_URL));
    let shape = update_item(&world).map(|item| {
        (
            item.label.clone(),
            item.caption_resource,
            item.enabled,
            item.checked,
            matches!(item.body, ItemBody::Action(_)),
        )
    });
    assert_eq!(
        shape,
        Some((
            "ネットワーク更新".to_owned(),
            Some("updatebutton.caption"),
            true,
            None,
            true,
        ))
    );
}

/// 3 つとも更新先が無ければ灰色（枠は出る）。対照: ゴーストの `descript.txt` に在れば選べる（要件 1.3・9.5）。
#[test]
fn item_is_greyed_when_none_of_the_three_has_a_source() {
    let (none, _tmp_none) = wired_world("areka-update-frame-none", None);
    let (one, _tmp_one) = wired_world("areka-update-frame-one", Some(GHOST_URL));
    assert_eq!(
        (
            update_item(&none).map(|i| i.enabled),
            update_item(&one).map(|i| i.enabled),
        ),
        (Some(false), Some(true))
    );
}

/// 走っている間は灰色: 台本の依頼が走り出した後（`Running`）も、メニューで選んで答えを待つ間
/// （`AwaitingExec`）も選べない（要件 1.3・9.5）。
#[test]
fn item_is_greyed_while_an_update_runs() {
    let (mut script, _tmp_script) = wired_world("areka-update-frame-running", Some(GHOST_URL));
    let verdict = update::submit(
        &mut script,
        RawUpdateRequest::Current(vec![TargetKind::Ghost]),
        UpdateReason::Script,
    );
    assert_eq!(
        verdict,
        SubmitVerdict::Started,
        "対照: 台本の依頼が走り出す"
    );

    let (mut manual, _tmp_manual) = wired_world("areka-update-frame-awaiting", Some(GHOST_URL));
    assert_eq!(update_item(&manual).map(|i| i.enabled), Some(true), "対照");
    select(&mut manual);

    assert_eq!(
        (
            update_item(&script).map(|i| i.enabled),
            update_item(&manual).map(|i| i.enabled),
        ),
        (Some(false), Some(false))
    );
}

/// 選ぶと今の 3 つを対象にした理由 `manual` の依頼がちょうど 1 件受付を通る（解けないシェルと
/// `descript.txt` の無いバルーンは飛ばされ、渡るのはゴーストだけ・要件 1.4）。
#[test]
fn selecting_submits_one_manual_order_to_the_intake() {
    let (mut world, _tmp) = wired_world("areka-update-frame-select", Some(GHOST_URL));
    let ((), events) = capture(|| select(&mut world));
    let started = named(&events, "update_order_started");
    assert_eq!(started.len(), 1, "依頼はちょうど 1 件: {events:?}");
    assert_eq!(started[0].field_str("origin"), Some("manual"));
    assert_eq!(started[0].field("targets"), Some(r#"["Emo"]"#));
    let skipped: Vec<_> = named(&events, "update_target_skipped")
        .iter()
        .map(|e| e.field_str("kind"))
        .collect();
    assert_eq!(
        skipped,
        vec![Some("shell"), Some("balloon")],
        "シェルとバルーンも頼んだ（飛ばされた）: {events:?}"
    );
}

/// 結線をやり直すと登記は新品で、「ネットワーク更新」枠は置き換えの記録なしに登記し直される（要件 1.2）。
#[test]
fn rewiring_gives_a_fresh_registration_with_the_update_frame() {
    let (mut world, _tmp) = wired_world("areka-update-frame-rewire", Some(GHOST_URL));
    let ((), events) = capture(|| {
        wire_menu(&mut world, mpsc::channel().0);
        register(&mut world);
    });
    assert_eq!(
        (
            world.non_send::<MenuWiring>().registry.registered_frames(),
            named(&events, "menu_registration_replaced").len(),
            update_item(&world).map(|i| i.enabled),
        ),
        (
            vec![Frame::Update, Frame::Readme, Frame::Close],
            0,
            Some(true)
        ),
        "{events:?}"
    );
}
