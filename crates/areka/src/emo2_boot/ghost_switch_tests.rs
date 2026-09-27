//! 切替要求の入口・名前の突き合わせ・台本の切替要求の取り出しの決定論テスト（areka-P0-ghost-shell-balloon-switch task 7.1・7.2）。
//!
//! 確かめること: 突き合わせの順（`name` → フォルダ名）と大文字小文字の区別・メニューの指し方
//! （フォルダ名だけ）・該当なし（未知の名前・`random`・`lastinstalled`）で `warn!` 1 件と送出 0 件・
//! 二重要求で `warn!` 1 件・受理で予約 1 つと kanade への切替の要求 1 件（今のゴースト自身も受理）・
//! 文脈／置き場が無い・送出の失敗で記録が 1 件残り予約が無いこと（要件 1.5〜1.9・8.7・10.6・11.8・11.9）。
//! 実行系は起こさない（置き場には kanade の送出端だけを持つ中身を据え、受信端で送出を数える）。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use areka_ghost::{BasewareRoot, GhostEntry, Identity};
use areka_kanade::{ChangeOrigin, ChangeRequest, ChangeTarget, KanadeMsg};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};

// ---------------------------------------------------------------- 道具立て

/// 目録の 1 項目（fs を見ない突き合わせ用）。
fn entry(folder: &str, name: Option<&str>) -> GhostEntry {
    GhostEntry {
        dir: PathBuf::from(format!(r"C:\ghost-switch-tests\ghost\{folder}")),
        identity: Identity {
            folder: folder.to_owned(),
            name: name.map(str::to_owned),
            ..Identity::default()
        },
    }
}

/// 突き合わせの結果をフォルダ名で見る。
fn resolved_folder(entries: &[GhostEntry], spec: GhostSpec) -> Option<String> {
    resolve_switch_target(entries, &spec).map(|t| t.folder)
}

fn name(s: &str) -> GhostSpec {
    GhostSpec::Name(s.to_owned())
}

fn folder(s: &str) -> GhostSpec {
    GhostSpec::Folder(s.to_owned())
}

/// `event` の記録の件数。
fn count_event(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// `event` の記録がちょうど 1 件あり、そのレベルが `level` であることを確かめる。
fn assert_one_event(events: &[CapturedEvent], event: &str, level: tracing::Level) {
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect();
    assert_eq!(hits.len(), 1, "{event} はちょうど 1 件: {events:?}");
    assert_eq!(hits[0].level, level, "{event} のレベル");
}

/// 根に 2 体のゴーストを組む: `A`（`name`＝Alice・`sakura.name`＝さくら）と `B`（`name` も
/// `sakura.name` も無い）。
fn fixture_root(tmp: &TempPath) -> BasewareRoot {
    let put = |folder: &str, descript: &str| {
        let master = tmp.child("ghost").join(folder).join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), descript).expect("descript");
    };
    put("A", "charset,UTF-8\nname,Alice\nsakura.name,さくら\n");
    put("B", "charset,UTF-8\n");
    BasewareRoot::new(tmp.path().to_path_buf())
}

/// 今のゴーストを `current` のフォルダとする起動の文脈。
fn boot_context(root: &BasewareRoot, current: &str) -> BootContext {
    BootContext {
        root: root.clone(),
        app_profile_dir: root.dir().join("profile"),
        helper_exe: root.dir().join("helper.exe"),
        argv_session: false,
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

/// 文脈と置き場（kanade の送出端だけを持つ中身）を据えた World と、kanade の受信端。
/// 今のゴーストは `A`。
fn world_with_slot(root: &BasewareRoot) -> (World, Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_resource(boot_context(root, "A"));
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        root.ghost_dir("A"),
    ))));
    (world, rx)
}

fn request(spec: GhostSpec, raise_event: bool, origin: ChangeOrigin) -> SwitchRequest {
    SwitchRequest {
        ghost: spec,
        raise_event,
        origin,
    }
}

/// kanade の受信端に届いた切替の要求（それ以外の便りは届かない前提で数えもする）。
fn sent_changes(rx: &Receiver<KanadeMsg>) -> Vec<ChangeRequest> {
    rx.try_iter()
        .map(|m| match m {
            KanadeMsg::ChangeGhost(req) => req,
            _ => panic!("切替の要求以外が送られた"),
        })
        .collect()
}

fn absolute(path: &Path) -> String {
    std::path::absolute(path)
        .expect("絶対パスにする")
        .display()
        .to_string()
}

// ---------------------------------------------------------------- 突き合わせ（純粋）

/// `name` の一致がフォルダ名の一致より先（並びが後ろでも `name` が勝つ）。`name` に一致が
/// 無ければフォルダ名で引く（要件 1.5・11.8）。
#[test]
fn name_matches_before_folder_name() {
    // 先頭の `Beta` はフォルダ名が Beta、2 番目の `alpha` は `name` が Beta。
    let entries = [entry("Beta", Some("Gamma")), entry("alpha", Some("Beta"))];
    assert_eq!(
        (
            resolved_folder(&entries, name("Beta")),
            resolved_folder(&entries, name("Gamma")),
            resolved_folder(&entries, name("alpha")),
        ),
        (
            Some("alpha".to_owned()),
            Some("Beta".to_owned()),
            Some("alpha".to_owned()),
        ),
        "(name 優先・name 一致・フォルダ名へ落ちる)"
    );
}

/// 大文字小文字を区別する（名前でもフォルダ名でも）。
#[test]
fn matching_is_case_sensitive() {
    let entries = [entry("Alpha", Some("Emily"))];
    assert_eq!(
        (
            resolved_folder(&entries, name("emily")),
            resolved_folder(&entries, name("alpha")),
            resolved_folder(&entries, folder("alpha")),
            resolved_folder(&entries, name("Emily")),
            resolved_folder(&entries, folder("Alpha")),
        ),
        (
            None,
            None,
            None,
            Some("Alpha".to_owned()),
            Some("Alpha".to_owned())
        ),
    );
}

/// メニューの指し方（フォルダ名）は `name` と突き合わせない。
#[test]
fn folder_spec_matches_folder_name_only() {
    let entries = [entry("alpha", Some("Beta"))];
    assert_eq!(
        (
            resolved_folder(&entries, folder("Beta")),
            resolved_folder(&entries, folder("alpha")),
        ),
        (None, Some("alpha".to_owned())),
    );
}

/// 切替先の名前は `name`（無ければフォルダ名）。`sakura.name` は突き合わせでは読まない。
#[test]
fn target_name_falls_back_to_folder_name() {
    let entries = [entry("alpha", Some("Beta")), entry("gamma", None)];
    let beta = resolve_switch_target(&entries, &name("Beta")).expect("在る");
    let gamma = resolve_switch_target(&entries, &folder("gamma")).expect("在る");
    assert_eq!(
        (beta, gamma),
        (
            SwitchTarget {
                dir: entries[0].dir.clone(),
                folder: "alpha".to_owned(),
                name: "Beta".to_owned(),
                sakura_name: None,
            },
            SwitchTarget {
                dir: entries[1].dir.clone(),
                folder: "gamma".to_owned(),
                name: "gamma".to_owned(),
                sakura_name: None,
            },
        ),
    );
}

// ---------------------------------------------------------------- 入口

/// 受理: 予約が 1 つ立ち、kanade へ切替の要求が 1 件だけ届く。要求の中身は切替先の
/// `sakura.name`・`name`・フォルダの絶対パスと出どころ・`raise_event`。今のゴースト（`A`）自身への
/// 切替も除外せず受理する（要件 1.8）。
#[test]
fn accepted_request_reserves_once_and_sends_one_change() {
    let tmp = TempPath::new("ghost-switch-accepted");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);

    let (verdict, events) = capture(|| {
        request_ghost_switch(
            &mut world,
            request(name("Alice"), true, ChangeOrigin::Automatic),
        )
    });

    let reserved = world.get_non_send::<SwitchInFlight>().map(|r| {
        (
            r.target.folder.clone(),
            r.target.sakura_name.clone(),
            r.prev.dir.clone(),
            r.prev.name.clone(),
            r.prev.sakura_name.clone(),
            r.stage,
        )
    });
    assert_eq!(
        (verdict, sent_changes(&rx), reserved),
        (
            SwitchVerdict::Accepted,
            vec![ChangeRequest {
                target: ChangeTarget {
                    sakura_name: "さくら".to_owned(),
                    name: "Alice".to_owned(),
                    dir: absolute(&root.ghost_dir("A")),
                },
                origin: ChangeOrigin::Automatic,
                raise_event: true,
            }],
            Some((
                "A".to_owned(),
                Some("さくら".to_owned()),
                root.ghost_dir("A"),
                // 実行系を持たない中身は名前情報を持たないので、名前は文脈のフォルダ名へ落ちる。
                Some("A".to_owned()),
                None,
                SwitchStage::SendOff,
            )),
        ),
        "{events:?}"
    );
    assert_one_event(&events, "ghost_switch_requested", tracing::Level::INFO);
}

/// 受理（メニューの指し方）: `name` も `sakura.name` も無いゴーストは、名前がフォルダ名・
/// 本体側の名前が空で送られる。出どころは手動。
#[test]
fn accepted_folder_request_without_names_sends_folder_and_empty_sakura_name() {
    let tmp = TempPath::new("ghost-switch-accepted-folder");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);

    let verdict = request_ghost_switch(
        &mut world,
        request(folder("B"), false, ChangeOrigin::Manual),
    );

    assert_eq!(
        (verdict, sent_changes(&rx)),
        (
            SwitchVerdict::Accepted,
            vec![ChangeRequest {
                target: ChangeTarget {
                    sakura_name: String::new(),
                    name: "B".to_owned(),
                    dir: absolute(&root.ghost_dir("B")),
                },
                origin: ChangeOrigin::Manual,
                raise_event: false,
            }],
        ),
    );
}

/// 該当なし（未知の名前・`random`・`lastinstalled`）: `warn!` 1 件・送出 0 件・予約なし
/// （降ろさず `OnGhostChanging` も送らない・要件 1.6・1.7）。
#[test]
fn unknown_names_warn_once_and_send_nothing() {
    let tmp = TempPath::new("ghost-switch-unknown");
    let root = fixture_root(&tmp);
    for unknown in ["Nobody", "random", "lastinstalled"] {
        let (mut world, rx) = world_with_slot(&root);
        let (verdict, events) = capture(|| {
            request_ghost_switch(
                &mut world,
                request(name(unknown), true, ChangeOrigin::Automatic),
            )
        });
        assert_eq!(
            (
                verdict,
                sent_changes(&rx).len(),
                world.get_non_send::<SwitchInFlight>().is_some(),
                count_event(&events, "ghost_switch_requested"),
            ),
            (SwitchVerdict::NotFound, 0, false, 0),
            "{unknown}: {events:?}"
        );
        assert_one_event(&events, "ghost_switch_unknown", tracing::Level::WARN);
    }
}

/// 二重要求: 予約が在る間の 2 件目は `warn!` 1 件で捨てる（送出は 1 件目の 1 件だけ・要件 1.9）。
#[test]
fn second_request_while_reserved_warns_once() {
    let tmp = TempPath::new("ghost-switch-busy");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);

    let first = request_ghost_switch(
        &mut world,
        request(name("Alice"), true, ChangeOrigin::Automatic),
    );
    let (second, events) = capture(|| {
        request_ghost_switch(&mut world, request(folder("B"), true, ChangeOrigin::Manual))
    });

    let reserved = world
        .get_non_send::<SwitchInFlight>()
        .map(|r| r.target.folder.clone());
    assert_eq!(
        (first, second, sent_changes(&rx).len(), reserved),
        (
            SwitchVerdict::Accepted,
            SwitchVerdict::Busy,
            1,
            Some("A".to_owned()),
        ),
        "{events:?}"
    );
    assert_one_event(&events, "ghost_switch_busy", tracing::Level::WARN);
}

/// 文脈が無い・置き場が無い・置き場が空・送出端が無い: `warn!` 1 件・予約なし。
#[test]
fn missing_context_or_slot_warns_and_reserves_nothing() {
    let tmp = TempPath::new("ghost-switch-no-context");
    let root = fixture_root(&tmp);
    let (tx, _rx) = mpsc::channel::<KanadeMsg>();

    let no_context = {
        let mut world = World::new();
        world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
            Some(tx.clone()),
            root.ghost_dir("A"),
        ))));
        world
    };
    let no_slot = no_slot_world(&root);
    let empty_slot = {
        let mut world = no_slot_world(&root);
        world.insert_non_send(GhostSlot(None));
        world
    };
    let no_sender = {
        let mut world = no_slot_world(&root);
        world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
            None,
            root.ghost_dir("A"),
        ))));
        world
    };

    for (what, mut world) in [
        ("文脈なし", no_context),
        ("置き場なし", no_slot),
        ("置き場が空", empty_slot),
        ("送出端なし", no_sender),
    ] {
        let (verdict, events) = capture(|| {
            request_ghost_switch(
                &mut world,
                request(name("Alice"), true, ChangeOrigin::Automatic),
            )
        });
        assert_eq!(
            (verdict, world.get_non_send::<SwitchInFlight>().is_some()),
            (SwitchVerdict::NoContext, false),
            "{what}: {events:?}"
        );
        assert_one_event(&events, "ghost_switch_no_context", tracing::Level::WARN);
    }
}

fn no_slot_world(root: &BasewareRoot) -> World {
    let mut world = World::new();
    world.insert_resource(boot_context(root, "A"));
    world
}

/// 送出の失敗（kanade の受信端が既に無い）: `error!` 1 件・予約なし・`requested` は残さない。
#[test]
fn send_failure_logs_error_and_leaves_no_reservation() {
    let tmp = TempPath::new("ghost-switch-send-failed");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);
    drop(rx);

    let (verdict, events) = capture(|| {
        request_ghost_switch(
            &mut world,
            request(name("Alice"), true, ChangeOrigin::Automatic),
        )
    });

    assert_eq!(
        (
            verdict,
            world.get_non_send::<SwitchInFlight>().is_some(),
            count_event(&events, "ghost_switch_requested"),
        ),
        (SwitchVerdict::NoContext, false, 0),
        "{events:?}"
    );
    assert_one_event(&events, "ghost_switch_send_failed", tracing::Level::ERROR);
}

// ---------------------------------------------------------------- 台本の切替要求の取り出し（task 7.2）

/// 入力の段に載っている系の数（段がまだ無ければ 0）。
fn input_systems_len(world: &World) -> usize {
    world
        .resource::<Schedules>()
        .get(Input)
        .map_or(0, |input| input.systems_len())
}

/// 登録は入力の段へ取り出しの系をちょうど 1 つ足し、受信端は置かない。受信端が無いまま段を
/// 回しても無操作（LogSink の起動と同じ）。
#[test]
fn register_change_drain_adds_exactly_one_input_system() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    register_change_drain(&mut world);
    world.run_schedule(Input);

    assert_eq!(
        (
            input_systems_len(&world),
            world.get_non_send::<ChangeRx>().is_some()
        ),
        (1, false),
        "(入力の段の系の数, 受信端の有無)"
    );
}

/// 結線は受信端を据えるだけで系を登録しない。
#[test]
fn wire_change_rx_inserts_the_receiver_without_registering() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    let (_tx, rx) = mpsc::channel::<ChangeRequestRaw>();

    wire_change_rx(&mut world, rx);

    assert_eq!(
        (
            input_systems_len(&world),
            world.get_non_send::<ChangeRx>().is_some()
        ),
        (0, true)
    );
}

/// 取り出しは届いた要求を名前の指し方・「自動」の出どころで入口へ渡す（要件 1.2・1.3）。
/// 2 件目は予約中なので入口が `ghost_switch_busy` で退ける（取り出しは全件を渡す）。
#[test]
fn drain_passes_each_request_to_the_entry_as_automatic() {
    let tmp = TempPath::new("ghost-switch-drain");
    let root = fixture_root(&tmp);
    let (mut world, kanade_rx) = world_with_slot(&root);
    world.init_resource::<Schedules>();
    register_change_drain(&mut world);
    let (tx, rx) = mpsc::channel();
    wire_change_rx(&mut world, rx);
    for name in ["Alice", "B"] {
        tx.send(ChangeRequestRaw {
            name: name.to_owned(),
            raise_event: true,
        })
        .expect("送れる");
    }

    let ((), events) = capture(|| world.run_schedule(Input));

    let sent: Vec<_> = sent_changes(&kanade_rx)
        .into_iter()
        .map(|c| (c.target.name, c.origin, c.raise_event))
        .collect();
    assert_eq!(
        (
            sent,
            world
                .get_non_send::<SwitchInFlight>()
                .map(|r| r.target.folder.clone()),
            count_event(&events, "ghost_switch_busy"),
        ),
        (
            vec![("Alice".to_owned(), ChangeOrigin::Automatic, true)],
            Some("A".to_owned()),
            1,
        ),
        "{events:?}"
    );
}
