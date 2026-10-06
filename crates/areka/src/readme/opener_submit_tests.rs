//! 開く専用のスレッドと唯一の入口のテスト（areka-P0-open-external-tags task 3.4・要件 7.1・7.5・10.1）。
//!
//! 入口 `submit` は差し込んだ送り先（`Opener::from_sender`）で判定する。`Opener::spawn` で起こした
//! スレッドはテストのビルドでは OS を呼ばずに断る口を持つので、その記録に行き先が積まれるかを
//! 時間の上限つきで待って判定する（記録はプロセス共有なので件数や位置では判定しない）。

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use areka_ghost::BasewareRoot;
use areka_parsers::sakura::JUMP_TAG_CARRIER;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::super::destination::{Destination, classify};
use super::super::opener_test_support::{build_ghost_root, refused_files};
use super::{OpenJob, Opener, listed_name, submit};
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::log_history::TARGET_ERROR;

fn dest(url: &str) -> Destination {
    classify(JUMP_TAG_CARRIER, &[url])
        .expect("開く系")
        .expect("受理される")
}

fn boot_context(root: &BasewareRoot) -> BootContext {
    BootContext {
        root: root.clone(),
        app_profile_dir: root.dir().join("profile"),
        helper_exe: root.dir().join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("g"),
                balloon_root: root.balloon_dir("b"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Memory,
                dir: root.ghost_dir("g"),
                folder: Some("g".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("b"),
                folder: Some("b".to_owned()),
            },
        },
    }
}

/// 置き場にゴースト `g`（実行系なし）を据えた World。
fn world_with_ghost(root: &BasewareRoot) -> World {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        None,
        root.ghost_dir("g"),
    ))));
    world
}

/// 入口で捨てた行がちょうど 1 行・理由が `reason` であることを判定する。
fn assert_one_drop(events: &[CapturedEvent], reason: &str) -> CapturedEvent {
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "error! はちょうど 1 行: {events:?}");
    let ev = errors[0].clone();
    assert_eq!(ev.target, TARGET_ERROR);
    assert_eq!(ev.field_str("event"), Some("open_external_dropped"));
    assert_eq!(ev.field_str("reason"), Some(reason));
    assert_eq!(ev.field_str("kind"), Some("url"));
    assert!(ev.field("destination").is_some());
    assert!(ev.field("tag").is_some());
    ev
}

/// 置き場から名を決める（`get_active_ghost_list` と同じ: descript の `name`、空・無しなら
/// フォルダの絶対パス・末尾の区切りなし）。
#[test]
fn listed_name_prefers_descript_name_else_trimmed_folder() {
    let dir = Path::new(r"C:\a\ghost\g\");
    assert_eq!(listed_name(Some("G"), dir), "G");
    assert_eq!(listed_name(Some(""), dir), r"C:\a\ghost\g");
    assert_eq!(listed_name(None, dir), r"C:\a\ghost\g");
}

#[test]
fn submit_copies_the_context_and_sends_one_job() {
    let tmp = TempPath::new("open-ext-submit");
    let root = build_ghost_root(&tmp, ("g", "G"), &[], &[]);
    let mut world = world_with_ghost(&root);
    world.insert_resource(boot_context(&root));
    let (tx, rx) = mpsc::channel::<OpenJob>();
    world.insert_resource(Opener::from_sender(tx));

    let d = dest("https://example.invalid/a");
    let ((), events) = capture(|| submit(&world, d.clone()));
    assert!(
        events.iter().all(|e| e.level != tracing::Level::ERROR),
        "{events:?}"
    );

    let job = rx.try_recv().expect("1 件届く");
    assert!(rx.try_recv().is_err(), "1 件だけ");
    assert_eq!(job.destination, d);
    let ghost_dir = std::path::absolute(root.ghost_dir("g")).unwrap();
    assert_eq!(job.context.ghost_dir, ghost_dir);
    // 実行系の無い置き場は descript の名を持たないので、フォルダの絶対パスが名になる。
    assert_eq!(job.context.ghost, listed_name(None, &ghost_dir));
    assert_eq!(job.context.baseware, Some(root));
}

#[test]
fn submit_without_boot_context_sends_no_baseware() {
    let tmp = TempPath::new("open-ext-submit-noroot");
    let root = build_ghost_root(&tmp, ("g", "G"), &[], &[]);
    let mut world = world_with_ghost(&root);
    let (tx, rx) = mpsc::channel::<OpenJob>();
    world.insert_resource(Opener::from_sender(tx));

    submit(&world, dest("https://example.invalid/b"));
    assert_eq!(rx.try_recv().expect("1 件届く").context.baseware, None);
}

#[test]
fn submit_without_ghost_logs_one_error_and_sends_nothing() {
    for slot in [None, Some(GhostSlot(None))] {
        let mut world = World::new();
        if let Some(slot) = slot {
            world.insert_non_send(slot);
        }
        let (tx, rx) = mpsc::channel::<OpenJob>();
        world.insert_resource(Opener::from_sender(tx));

        let ((), events) = capture(|| submit(&world, dest("https://example.invalid/c")));
        let ev = assert_one_drop(&events, "no_ghost");
        assert_eq!(ev.field("ghost"), None, "ゴーストが居なければ名は無い");
        assert!(rx.try_recv().is_err(), "送らない");
    }
}

#[test]
fn submit_without_opener_logs_one_error_with_the_ghost_name() {
    let tmp = TempPath::new("open-ext-submit-noopener");
    let root = build_ghost_root(&tmp, ("g", "G"), &[], &[]);
    let world = world_with_ghost(&root);

    let ((), events) = capture(|| submit(&world, dest("https://example.invalid/d")));
    let ev = assert_one_drop(&events, "no_opener");
    let ghost_dir = std::path::absolute(root.ghost_dir("g")).unwrap();
    assert_eq!(
        ev.field_str("ghost").or(ev.field("ghost")),
        Some(listed_name(None, &ghost_dir).as_str())
    );
}

#[test]
fn submit_to_a_closed_thread_logs_one_error() {
    let tmp = TempPath::new("open-ext-submit-closed");
    let root = build_ghost_root(&tmp, ("g", "G"), &[], &[]);
    let mut world = world_with_ghost(&root);
    let (tx, rx) = mpsc::channel::<OpenJob>();
    drop(rx);
    world.insert_resource(Opener::from_sender(tx));

    let ((), events) = capture(|| submit(&world, dest("https://example.invalid/e")));
    assert_one_drop(&events, "disconnected");
}

/// テストのビルドで起こしたスレッドは OS を呼ばず、断る口の記録に積む（要件 10.1）。
#[test]
fn spawned_thread_in_test_build_records_and_refuses() {
    let tmp = TempPath::new("open-ext-spawn");
    let root = build_ghost_root(&tmp, ("g", "G"), &[], &[]);
    let mut world = world_with_ghost(&root);
    world.insert_resource(Opener::spawn().expect("スレッドを起こす"));

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let url = format!(
        "https://example.invalid/spawn-{}-{nanos}",
        std::process::id()
    );
    submit(&world, dest(&url));

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if refused_files().iter().any(|f| f == url.as_str()) {
            break;
        }
        assert!(Instant::now() < deadline, "断る口の記録に {url} が無い");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
