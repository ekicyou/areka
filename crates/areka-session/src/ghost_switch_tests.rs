//! 切替要求の入口・名前の突き合わせ・台本の切替要求の取り出しの決定論テスト。
//!
//! 確かめること: 突き合わせの順（`name` → フォルダ名）と大文字小文字の区別・メニューの指し方
//! （フォルダ名だけ）・該当なし（未知の名前）で `warn!`（`reason = "name"`）1 件と送出 0 件・
//! 二重要求で `warn!` 1 件・受理で予約 1 つと kanade への切替の要求 1 件（今のゴースト自身も受理）・
//! 文脈／置き場が無い・送出の失敗で記録が 1 件残り予約が無いこと。
//! 実行系は起こさない（置き場には kanade の送出端だけを持つ中身を据え、受信端で送出を数える）。
//! 特別な名前（`random`・`sequential`・`lastinstalled`）の純粋な解決は areka-runtime 側で確かめる。
//! 入口での解決の分岐（切替先へ解ける・解けない理由の語・`lastinstalled` の記録・
//! 予約中・メニューのフォルダ名）は実 fs の目録で確かめる。

use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use areka_ghost::{BasewareRoot, GhostEntry};
use areka_kanade::{ChangeOrigin, ChangeRequest, ChangeTarget, KanadeMsg};
use areka_runtime::special_name::catalog_entry;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::*;
use crate::body_systems::NO_BODY;
use crate::boot_resolve::boot_resolve_tests::{assert_one_event, count_event};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::ghost_switch_test_support::boot_context;

// ---------------------------------------------------------------- 道具立て

pub(super) fn resolved_folder(entries: &[GhostEntry], spec: GhostSpec) -> Option<String> {
    resolve_switch_target(entries, &spec).map(|t| t.folder)
}

pub(super) fn name(s: &str) -> GhostSpec {
    GhostSpec::Name(s.to_owned())
}

pub(super) fn folder(s: &str) -> GhostSpec {
    GhostSpec::Folder(s.to_owned())
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

/// 文脈と置き場（kanade の送出端だけを持つ中身）を据えた World と、kanade の受信端。
/// 今のゴーストは `A`。
fn world_with_slot(root: &BasewareRoot) -> (World, Receiver<KanadeMsg>) {
    world_at(root, "A")
}

/// [`world_with_slot`] の今のゴーストを `current` にしたもの。
fn world_at(root: &BasewareRoot, current: &str) -> (World, Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_resource(boot_context(root, current));
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        root.ghost_dir(current),
    ))));
    (world, rx)
}

fn request(spec: GhostSpec, raise_event: bool, origin: ChangeOrigin) -> SwitchRequest {
    SwitchRequest {
        ghost: spec,
        raise_event,
        origin,
        boot_event: None,
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
/// 無ければフォルダ名で引く。
#[test]
fn name_matches_before_folder_name() {
    // 先頭の `Beta` はフォルダ名が Beta、2 番目の `alpha` は `name` が Beta。
    let entries = [
        catalog_entry("Beta", Some("Gamma")),
        catalog_entry("alpha", Some("Beta")),
    ];
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

/// `name` は大文字小文字を区別し、フォルダ名は 1 つの規則で区別しない。
#[test]
fn name_is_case_sensitive_folder_name_is_not() {
    let entries = [catalog_entry("Alpha", Some("Emily"))];
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
            Some("Alpha".to_owned()),
            Some("Alpha".to_owned()),
            Some("Alpha".to_owned()),
            Some("Alpha".to_owned())
        ),
    );
}

/// メニューの指し方（フォルダ名）は `name` と突き合わせない。
#[test]
fn folder_spec_matches_folder_name_only() {
    let entries = [catalog_entry("alpha", Some("Beta"))];
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
    let entries = [
        catalog_entry("alpha", Some("Beta")),
        catalog_entry("gamma", None),
    ];
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

/// 呼ばれてはならない乱数。
fn no_pick(n: usize) -> usize {
    panic!("乱数は呼ばれないはず（n = {n}）")
}

// ---------------------------------------------------------------- 入口

/// 受理: 予約が 1 つ立ち、kanade へ切替の要求が 1 件だけ届く。要求の中身は切替先の
/// `sakura.name`・`name`・フォルダの絶対パスと出どころ・`raise_event`。今のゴースト（`A`）自身への
/// 切替も除外せず受理する。
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

/// 名指しの該当なし（未知の名前）: `warn!(ghost_switch_unknown, reason = "name")` 1 件・送出 0 件・
/// 予約なし（降ろさず `OnGhostChanging` も送らない）。
#[test]
fn unknown_names_warn_once_and_send_nothing() {
    let tmp = TempPath::new("ghost-switch-unknown");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);
    let (verdict, events) = capture(|| {
        request_ghost_switch(
            &mut world,
            request(name("Nobody"), true, ChangeOrigin::Automatic),
        )
    });
    assert_ignored((&world, &rx), (verdict, &events), "name");
}

/// 二重要求: 予約が在る間の 2 件目は `warn!` 1 件で捨てる（送出は 1 件目の 1 件だけ）。
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

/// 入口を通らない受理（読み直しの窓口が直に呼ぶ）でも、予約が在れば `warn!(ghost_switch_busy)`
/// 1 件で退ける: 送出 0 件・予約は 1 件目のまま。
#[test]
fn accept_while_reserved_is_busy_and_keeps_the_reservation() {
    let tmp = TempPath::new("ghost-switch-accept-busy");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);
    let first = request_ghost_switch(
        &mut world,
        request(name("Alice"), true, ChangeOrigin::Automatic),
    );
    let first_sent = sent_changes(&rx).len();
    let target = SwitchTarget {
        dir: root.ghost_dir("B"),
        folder: "B".to_owned(),
        name: "B".to_owned(),
        sakura_name: None,
    };

    let (second, events) = capture(|| {
        accept_switch(
            &mut world,
            target,
            request(folder("B"), false, ChangeOrigin::Automatic),
        )
    });

    let reserved = world
        .get_non_send::<SwitchInFlight>()
        .map(|r| r.target.folder.clone());
    assert_eq!(
        (first, first_sent, second, sent_changes(&rx).len(), reserved),
        (
            SwitchVerdict::Accepted,
            1,
            SwitchVerdict::Busy,
            0,
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

// ---------------------------------------------------------------- 台本の切替要求の取り出し

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

/// 取り出しは届いた要求を名前の指し方・「自動」の出どころで入口へ渡す。
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

/// 1 回の取り出しで同じ解けない名 3 件・解ける名・解けない名を渡すと、該当なしは 3 行・進行中は 1 行で、
/// 目録は 1 度だけ読む。受理の後は埋めた目録を捨て、次の要求で 1 度だけ読み直す。
#[test]
fn unresolved_names_in_one_drain_read_the_catalog_once() {
    let tmp = TempPath::new("ghost-switch-drain-catalog");
    let root = fixture_root(&tmp);
    let (mut world, _kanade_rx) = world_with_slot(&root);
    let (tx, rx) = mpsc::channel();
    wire_change_rx(&mut world, rx);
    for name in ["Nobody", "Nobody", "Nobody", "Alice", "Nobody"] {
        tx.send(ChangeRequestRaw {
            name: name.to_owned(),
            raise_event: false,
        })
        .expect("送れる");
    }
    let reads = std::cell::Cell::new(0);
    let mut read = |roots: &areka_ghost::BasewareRoots| {
        reads.set(reads.get() + 1);
        list_ghosts(roots)
    };

    let ((), events) = capture(|| drain_change_requests_with(&mut world, &mut read));
    let drained = (
        count_event(&events, "ghost_switch_unknown"),
        count_event(&events, "ghost_switch_busy"),
        reads.get(),
    );
    world.remove_non_send::<SwitchInFlight>();
    let mut catalog = None;
    let accepted = request_ghost_switch_with(
        &mut world,
        request(name("Alice"), false, ChangeOrigin::Automatic),
        no_pick,
        &mut catalog,
        &mut read,
    );
    world.remove_non_send::<SwitchInFlight>();
    let after = request_ghost_switch_with(
        &mut world,
        request(name("Nobody"), false, ChangeOrigin::Automatic),
        no_pick,
        &mut catalog,
        &mut read,
    );

    assert_eq!(
        (drained, accepted, after, reads.get()),
        (
            (3, 1, 1),
            SwitchVerdict::Accepted,
            SwitchVerdict::NotFound,
            3
        ),
        "((取り出しの該当なし, 進行中, 読んだ回数), 受理, 受理の後, 読んだ回数の合計): {events:?}"
    );
}

// ---------------------------------------------------------------- 入口での特別な名前の解決

/// 乱数だけを外から渡す入口（目録は毎回読む）。
fn entry_with(
    world: &mut World,
    req: SwitchRequest,
    pick: impl FnOnce(usize) -> usize,
) -> SwitchVerdict {
    request_ghost_switch_with(world, req, pick, &mut None, &mut list_ghosts)
}

/// `fixture_root` の `A`／`B` へ切り替える kanade への要求。
fn change_to(
    root: &BasewareRoot,
    folder: &str,
    origin: ChangeOrigin,
    raise: bool,
) -> ChangeRequest {
    let (sakura_name, name) = match folder {
        "A" => ("さくら", "Alice"),
        _ => ("", folder),
    };
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: sakura_name.to_owned(),
            name: name.to_owned(),
            dir: absolute(&root.ghost_dir(folder)),
        },
        origin,
        raise_event: raise,
    }
}

/// `event` の記録 1 件の欄 `key` の Debug 表現（`%`／`?` で渡した値）。
fn field_of(events: &[CapturedEvent], event: &str, key: &str) -> Option<String> {
    events
        .iter()
        .find(|e| e.field_str("event") == Some(event))
        .and_then(|e| e.field(key).map(str::to_owned))
}

/// 受理: 送出は `want` の 1 件・予約の切替先は `want` のフォルダ・`ghost_switch_resolved`
/// （`to` がそのフォルダ・`position` が `position`）と `ghost_switch_requested` が 1 件ずつ。
fn assert_switched(
    (world, rx): (&World, &Receiver<KanadeMsg>),
    (verdict, events): (SwitchVerdict, &[CapturedEvent]),
    want: ChangeRequest,
    folder: &str,
    position: &str,
) {
    let reserved = world
        .get_non_send::<SwitchInFlight>()
        .map(|r| r.target.folder.clone());
    assert_eq!(
        (verdict, sent_changes(rx), reserved),
        (SwitchVerdict::Accepted, vec![want], Some(folder.to_owned())),
        "{events:?}"
    );
    assert_one_event(events, "ghost_switch_resolved", tracing::Level::INFO);
    assert_one_event(events, "ghost_switch_requested", tracing::Level::INFO);
    assert_eq!(
        (
            field_of(events, "ghost_switch_resolved", "to"),
            field_of(events, "ghost_switch_resolved", "position"),
        ),
        (Some(folder.to_owned()), Some(position.to_owned())),
        "(to, position)"
    );
}

/// 無視: 該当なし・送出 0 件・予約なし・`ghost_switch_resolved`／`requested` 0 件・
/// `ghost_switch_unknown` WARN 1 件で `reason` が `reason`。
fn assert_ignored(
    (world, rx): (&World, &Receiver<KanadeMsg>),
    (verdict, events): (SwitchVerdict, &[CapturedEvent]),
    reason: &str,
) {
    assert_eq!(
        (
            verdict,
            sent_changes(rx).len(),
            world.get_non_send::<SwitchInFlight>().is_some(),
            count_event(events, "ghost_switch_resolved"),
            count_event(events, "ghost_switch_requested"),
        ),
        (SwitchVerdict::NotFound, 0, false, 0, 0),
        "{events:?}"
    );
    assert_one_event(events, "ghost_switch_unknown", tracing::Level::WARN);
    assert_eq!(
        events
            .iter()
            .find(|e| e.field_str("event") == Some("ghost_switch_unknown"))
            .and_then(|e| e.field_str("reason")),
        Some(reason),
        "{events:?}"
    );
}

/// `random`（今 `A` なら候補は `B` だけ）と `sequential`（今 `A` → `B`・今 `B` → 末尾から先頭の
/// `A`）が入口で切替先へ解け、出どころと `raise_event` の真偽はそのまま kanade へ届く。
#[test]
fn random_and_sequential_resolve_to_a_target_at_the_entry() {
    let tmp = TempPath::new("ghost-switch-resolve");
    let root = fixture_root(&tmp);
    for (special, current, to, position) in [
        ("random", "A", "B", "None"),
        ("sequential", "A", "B", "Some(0)"),
        ("sequential", "B", "A", "Some(1)"),
    ] {
        for (raise, origin) in [
            (true, ChangeOrigin::Automatic),
            (false, ChangeOrigin::Manual),
        ] {
            let (mut world, rx) = world_at(&root, current);
            let (verdict, events) = capture(|| {
                entry_with(&mut world, request(name(special), raise, origin), |n| {
                    assert_eq!(
                        (special, n),
                        ("random", 1),
                        "乱数は random だけ・候補は B だけ"
                    );
                    0
                })
            });
            assert_switched(
                (&world, &rx),
                (verdict, &events),
                change_to(&root, to, origin, raise),
                to,
                position,
            );
        }
    }
}

/// 目録が空（`ghost/` の無い根）: `random`／`sequential` は解けず、理由の語はそれぞれ `random_empty`・`sequential_empty`。
#[test]
fn special_names_on_an_empty_catalog_are_ignored() {
    let tmp = TempPath::new("ghost-switch-empty-catalog");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    for (special, reason) in [
        ("random", "random_empty"),
        ("sequential", "sequential_empty"),
    ] {
        let (mut world, rx) = world_at(&root, "A");
        let (verdict, events) = capture(|| {
            entry_with(
                &mut world,
                request(name(special), true, ChangeOrigin::Automatic),
                no_pick,
            )
        });
        assert_ignored((&world, &rx), (verdict, &events), reason);
    }
}

/// `lastinstalled`: 記録なし・消えたゴースト・最後に書いた 1 件（前の `Zed` は置き換わる）・
/// 今のゴースト自身。受理のあとも記録は消えず、`last_installed_recorded` は書いた回数だけ残る。
#[test]
fn lastinstalled_switches_to_the_last_recorded_ghost() {
    let tmp = TempPath::new("ghost-switch-lastinstalled");
    let root = fixture_root(&tmp);
    for (records, raise, want) in [
        (&[][..], true, Err("lastinstalled_none")),
        (&["Gone"][..], true, Err("lastinstalled_missing")),
        (&["B"][..], false, Ok("B")),
        (&["Zed", "B"][..], true, Ok("B")),
        (&["A"][..], true, Ok("A")),
    ] {
        let (mut world, rx) = world_with_slot(&root);
        let (verdict, events) = capture(|| {
            for r in records {
                record_last_installed(&mut world, (*r).to_owned());
            }
            entry_with(
                &mut world,
                request(name("lastinstalled"), raise, ChangeOrigin::Automatic),
                no_pick,
            )
        });
        assert_eq!(
            (
                count_event(&events, "last_installed_recorded"),
                world
                    .get_resource::<LastInstalledGhost>()
                    .map(|r| r.0.clone()),
            ),
            (records.len(), records.last().map(|r| (*r).to_owned())),
            "{records:?}: (記録した回数, 残っている記録)"
        );
        match want {
            Ok(to) => assert_switched(
                (&world, &rx),
                (verdict, &events),
                change_to(&root, to, ChangeOrigin::Automatic, raise),
                to,
                "None",
            ),
            Err(reason) => assert_ignored((&world, &rx), (verdict, &events), reason),
        }
    }
}

/// 予約の判定が解決より先: 切替中の 2 通目の `random` は解かずに `Busy`。
#[test]
fn special_name_while_reserved_is_busy_without_resolving() {
    let tmp = TempPath::new("ghost-switch-busy-random");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);
    let first = request_ghost_switch(
        &mut world,
        request(name("Alice"), true, ChangeOrigin::Automatic),
    );
    let (second, events) = capture(|| {
        entry_with(
            &mut world,
            request(name("random"), true, ChangeOrigin::Automatic),
            no_pick,
        )
    });
    assert_eq!(
        (
            first,
            second,
            sent_changes(&rx).len(),
            count_event(&events, "ghost_switch_resolved"),
        ),
        (SwitchVerdict::Accepted, SwitchVerdict::Busy, 1, 0),
        "{events:?}"
    );
    assert_one_event(&events, "ghost_switch_busy", tracing::Level::WARN);
}

/// メニューのフォルダの名指しは特別な名前として解かない: 目録に無いフォルダ名 `random` は
/// 名指しの該当なし（`reason = "name"`）。
#[test]
fn menu_folder_named_random_is_not_resolved() {
    let tmp = TempPath::new("ghost-switch-folder-random");
    let root = fixture_root(&tmp);
    let (mut world, rx) = world_with_slot(&root);
    let (verdict, events) = capture(|| {
        entry_with(
            &mut world,
            request(folder("random"), false, ChangeOrigin::Manual),
            no_pick,
        )
    });
    assert_ignored((&world, &rx), (verdict, &events), "name");
}

/// 更新の実行中はメニューの要求だけを `Updating` で断り（送出 0 件）、台本（名前で指す）と
/// 手続き（フォルダで指す）の要求は受理する。
#[test]
fn updating_refuses_menu_requests_only() {
    let tmp = TempPath::new("ghost-switch-updating");
    let root = fixture_root(&tmp);
    for (spec, origin, want, sent) in [
        (
            folder("B"),
            ChangeOrigin::Manual,
            SwitchVerdict::Updating,
            0,
        ),
        (
            name("Alice"),
            ChangeOrigin::Automatic,
            SwitchVerdict::Accepted,
            1,
        ),
        (
            folder("B"),
            ChangeOrigin::Automatic,
            SwitchVerdict::Accepted,
            1,
        ),
    ] {
        let (mut world, rx) = world_with_slot(&root);
        world.insert_resource(BodySystems {
            update_busy: |_| true,
            ..NO_BODY
        });
        let (verdict, events) =
            capture(|| request_ghost_switch(&mut world, request(spec.clone(), false, origin)));
        assert_eq!(
            (
                verdict,
                sent_changes(&rx).len(),
                count_event(&events, "ghost_switch_updating")
            ),
            (want, sent, 1 - sent),
            "{spec:?} {origin:?}: {events:?}"
        );
    }
}
