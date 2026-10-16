//! フォルダを選ぶ画面の受け口の決定論テスト（要件 6.3・6.5・6.9・design「RootsDesk」）。
//!
//! 本物の画面は出さない: 選ぶ口の代わり（閉包）を差し込み、選ぶスレッドの終わりを `join` で待って
//! から UI スレッドの系 [`drain`] を 1 回回す。記録は選ぶスレッドの外で [`send_pick`] を直に呼んで見る。

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;
use windows::Win32::Foundation::E_FAIL;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::roots::RootsOrigin;
use areka_ghost::{BasewareRoot, BasewareRoots};

/// 選ぶ口の代わりが止まっている間の待ちの上限（固まったときに赤で止めるためだけの値）。
const BOUND: Duration = Duration::from_secs(10);

/// 根 1 つ（`a`）の並びの起動の文脈と受け口を据えた World と、足す候補の根 `b`。
fn rig(name: &str) -> (TempPath, World, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(name);
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    for dir in [&a, &b] {
        std::fs::create_dir_all(dir.join("ghost")).unwrap();
    }
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(BootContext {
        roots: roots_of(&[&a]),
        roots_origin: RootsOrigin::Default,
        app_profile_dir: tmp.path().join("profile"),
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
    install(&mut world);
    (tmp, world, a, b)
}

fn roots_of(dirs: &[&Path]) -> BasewareRoots {
    BasewareRoots::new(
        dirs.iter()
            .map(|d| BasewareRoot::new(d.to_path_buf()))
            .collect(),
    )
}

/// 代わりの選ぶ口で画面を出し、スレッドの終わりを待ってから受ける系を回す。
fn pick_and_drain(world: &mut World, result: Result<Option<PathBuf>, PickError>) -> Option<String> {
    let handle = start_pick(world, false, move || result).expect("選ぶスレッドが起きる");
    let name = handle.thread().name().map(str::to_owned);
    handle.join().expect("選ぶスレッドは panic しない");
    drain(world);
    name
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 決めたフォルダは `roots-pick` のスレッドから受け口へ届き、UI スレッドで並びの末尾に入る（要件 6.3）。
#[test]
fn chosen_folder_is_appended_at_end() {
    let (_tmp, mut world, a, b) = rig("roots-desk-chosen");
    let name = pick_and_drain(&mut world, Ok(Some(b.clone())));
    assert_eq!(name.as_deref(), Some("roots-pick"));
    assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a, &b]));
    assert!(can_pick(&world), "スレッドが終わると旗が降りる");
}

/// 取り消しと失敗は何も送らず、並びは変わらない（要件 6.5・6.9）。
#[test]
fn cancel_and_failure_leave_roots_unchanged() {
    for result in [Ok(None), Err(PickError::Com(E_FAIL))] {
        let label = format!("{result:?}");
        let (_tmp, mut world, a, _b) = rig("roots-desk-unchanged");
        pick_and_drain(&mut world, result);
        assert_eq!(
            world.resource::<BootContext>().roots,
            roots_of(&[&a]),
            "{label}"
        );
        assert!(can_pick(&world), "{label}");
    }
}

/// 取り消しは info `roots_pick_cancelled`、失敗は error `roots_pick_failed` を 1 件残して送らない。
#[test]
fn pick_results_are_logged_and_only_a_choice_is_sent() {
    let chosen = PathBuf::from(r"C:\areka-roots-desk-test\b");
    let cases: [(Result<Option<PathBuf>, PickError>, &str, Level, usize); 3] = [
        (Ok(None), "roots_pick_cancelled", Level::INFO, 0),
        (
            Err(PickError::Com(E_FAIL)),
            "roots_pick_failed",
            Level::ERROR,
            0,
        ),
        (Ok(Some(chosen.clone())), "", Level::INFO, 1),
    ];
    for (result, event, level, sent) in cases {
        let label = format!("{result:?}");
        let (tx, rx) = mpsc::channel();
        let ((), events) = capture(|| send_pick(result, &tx));
        assert_eq!(rx.try_iter().count(), sent, "{label}");
        if event.is_empty() {
            assert!(
                events.iter().all(|e| e.level > Level::WARN),
                "{label}: {events:?}"
            );
        } else {
            let levels: Vec<Level> = events_named(&events, event)
                .iter()
                .map(|e| e.level)
                .collect();
            assert_eq!(levels, vec![level], "{label}: {events:?}");
        }
    }
}

/// 画面が出ている間は選べず、2 つ目の画面は出さない（代わりの選ぶ口は呼ばれない）。
#[test]
fn second_dialog_is_not_shown_while_picking() {
    let (_tmp, mut world, a, _b) = rig("roots-desk-second");
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let handle = start_pick(&mut world, false, move || {
        let _ = entered_tx.send(());
        let _ = release_rx.recv_timeout(BOUND);
        Ok(None)
    })
    .expect("選ぶスレッドが起きる");
    entered
        .recv_timeout(BOUND)
        .expect("選ぶ口の代わりが呼ばれる");

    let during = can_pick(&world);
    let (called_tx, called) = mpsc::channel();
    let (second, events) = capture(|| {
        start_pick(&mut world, false, move || {
            let _ = called_tx.send(());
            Ok(None)
        })
    });
    release.send(()).expect("選ぶ口の代わりは待っている");
    handle.join().expect("選ぶスレッドは panic しない");
    drain(&mut world);

    assert!(!during, "出ている間は選べない");
    assert!(second.is_none(), "2 つ目のスレッドは起きない");
    assert!(called.try_recv().is_err(), "2 つ目の代わりは呼ばれない");
    assert_eq!(events_named(&events, "roots_pick_busy").len(), 1);
    assert!(can_pick(&world), "終わった後は選べる");
    assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a]));
}

/// 告知を抑える設定のときは画面を出さず、取り消しと同じ（info・理由 `no_alert`）で並びは変わらない。
#[test]
fn suppressed_shows_no_dialog_and_counts_as_cancel() {
    let (_tmp, mut world, a, b) = rig("roots-desk-suppressed");
    let (called_tx, called) = mpsc::channel();
    let (handle, events) = capture(|| {
        start_pick(&mut world, true, move || {
            let _ = called_tx.send(());
            Ok(Some(b))
        })
    });
    drain(&mut world);

    assert!(handle.is_none(), "スレッドは起きない");
    assert!(called.try_recv().is_err(), "選ぶ口は呼ばれない");
    let cancelled = events_named(&events, "roots_pick_cancelled");
    assert_eq!(cancelled.len(), 1, "{events:?}");
    assert_eq!(cancelled[0].level, Level::INFO);
    assert_eq!(cancelled[0].field_str("reason"), Some("no_alert"));
    assert!(can_pick(&world), "旗は立たない");
    assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a]));
}
