use super::deliver_dropped_files;
use crate::ecs::window::OnFilesDropped;
use crate::ecs::window_proc::lifecycle::DESPAWNED_SKIP_TAG;
use crate::ecs::world::EcsWorld;
use bevy_ecs::prelude::*;
use log_capture_kit::CapturedEvent;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::Level;
use windows::Win32::Foundation::LRESULT;

/// 受け手が呼ばれるたびに渡された一覧を積む（テスト専用）。
/// 資源が挿さっていないこと＝1 度も呼ばれていない。
#[derive(Resource, Default)]
struct Received(Vec<Vec<PathBuf>>);

fn record_drop(world: &mut World, _entity: Entity, paths: Vec<PathBuf>) {
    world
        .get_resource_or_insert_with(Received::default)
        .0
        .push(paths);
}

/// `OnFilesDropped` を差した entity と差さない entity を持つ World。
fn drop_world() -> (Rc<RefCell<EcsWorld>>, Entity, Entity) {
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (hooked, plain) = {
        let mut w = world.borrow_mut();
        let w = w.world_mut();
        let hooked = w.spawn(OnFilesDropped(record_drop)).id();
        let plain = w.spawn_empty().id();
        (hooked, plain)
    };
    (world, hooked, plain)
}

fn fake_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from(r"C:\drop\a.txt"),
        PathBuf::from(r"C:\drop\フォルダ"),
    ]
}

fn received(world: &Rc<RefCell<EcsWorld>>) -> Option<Vec<Vec<PathBuf>>> {
    world
        .borrow()
        .world()
        .get_resource::<Received>()
        .map(|r| r.0.clone())
}

/// `event` 欄が `name` の記録。
fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn warn_count(events: &[CapturedEvent]) -> usize {
    events.iter().filter(|e| e.level <= Level::WARN).count()
}

/// 要件 1.1・9.1: 部品を差した窓に偽の一覧 2 件を渡すと、受け手が同じ一覧で 1 回呼ばれ、
/// その前に `files_dropped`（`count = 2`）の `info!` がちょうど 1 行出る。
#[test]
fn deliver_calls_receiver_once_with_the_same_list() {
    let (world, hooked, _) = drop_world();

    let (ret, events) =
        log_capture_kit::capture(|| deliver_dropped_files(&world, hooked, fake_paths()));

    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(
        received(&world),
        Some(vec![fake_paths()]),
        "受け手が同じ一覧で 1 回だけ呼ばれていない"
    );
    let hits = events_named(&events, "files_dropped");
    assert_eq!(
        hits.len(),
        1,
        "files_dropped がちょうど 1 行でない: {events:?}"
    );
    assert_eq!(hits[0].level, Level::INFO);
    assert_eq!(hits[0].field("count"), Some("2"));
    assert_eq!(warn_count(&events), 0, "警告以上が出ている: {events:?}");
}

/// 要件 9.1: 部品の無い窓では呼ばず、`files_dropped_no_receiver` の `debug!` 1 行・警告 0。
#[test]
fn deliver_without_receiver_only_logs_debug() {
    let (world, _, plain) = drop_world();

    let (ret, events) =
        log_capture_kit::capture(|| deliver_dropped_files(&world, plain, fake_paths()));

    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(received(&world), None, "部品の無い窓で受け手が呼ばれた");
    let levels: Vec<Level> = events_named(&events, "files_dropped_no_receiver")
        .iter()
        .map(|e| e.level)
        .collect();
    assert_eq!(levels, vec![Level::DEBUG]);
    assert!(events_named(&events, "files_dropped").is_empty());
    assert_eq!(warn_count(&events), 0, "警告以上が出ている: {events:?}");
}

/// 要件 3.5・6.4: 破棄済みの entity（終了の直後に届いた投げ込み）では呼ばず、
/// `DESPAWNED_SKIP_TAG` の `debug!` 1 行で打ち切る。警告 0。
///
/// 「警告が無い」は捕捉が警告を見られるときにだけ意味を持つので、先にそれを示す
/// （`wm_close_skips_stale_entity_as_normal_teardown` と同じ自己証明の腕）。
#[test]
fn deliver_on_despawned_entity_skips_as_normal_teardown() {
    // ── 自己証明の腕: 捕捉は「在るとき」に警告を見られる ──
    let ((), probe) = log_capture_kit::capture(|| {
        tracing::warn!("[harness-selfcheck] 捕捉可能性の自己証明");
    });
    assert_eq!(
        warn_count(&probe),
        1,
        "捕捉が tracing の WARN を見られない。後段の「警告 0」が空虚になる: {probe:?}"
    );

    // ── 本体の腕 ──
    let (world, hooked, _) = drop_world();
    assert!(world.borrow_mut().world_mut().despawn(hooked));

    let (ret, events) =
        log_capture_kit::capture(|| deliver_dropped_files(&world, hooked, fake_paths()));

    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(
        received(&world),
        None,
        "破棄済みの entity で受け手が呼ばれた"
    );
    let skips: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(DESPAWNED_SKIP_TAG))
        .collect();
    assert_eq!(
        skips.len(),
        1,
        "破棄済みの打ち切りが 1 行でない: {events:?}"
    );
    assert_eq!(skips[0].level, Level::DEBUG);
    assert!(
        skips[0].message().contains("WM_DROPFILES"),
        "打ち切り行が自分の相（WM_DROPFILES）を名乗っていない: {:?}",
        skips[0]
    );
    assert_eq!(warn_count(&events), 0, "警告以上が出ている: {events:?}");
}

/// 要件 9.1: World の借用中は呼べない。`files_dropped_world_busy`（`count`）の `warn!` 1 行・panic なし。
#[test]
fn deliver_while_world_borrowed_warns_without_call() {
    let (world, hooked, _) = drop_world();

    let held = world.borrow_mut();
    let (ret, events) =
        log_capture_kit::capture(|| deliver_dropped_files(&world, hooked, fake_paths()));
    drop(held);

    assert_eq!(ret, Some(LRESULT(0)));
    assert_eq!(received(&world), None, "借用中に受け手が呼ばれた");
    let hits = events_named(&events, "files_dropped_world_busy");
    assert_eq!(
        hits.len(),
        1,
        "files_dropped_world_busy が 1 行でない: {events:?}"
    );
    assert_eq!(hits[0].level, Level::WARN);
    assert_eq!(hits[0].field("count"), Some("2"));
}
