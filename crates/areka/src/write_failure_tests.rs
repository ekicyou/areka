//! 致命の机の決定論テスト（areka-P0-write-failure-notice task 4.1・要件 1.1〜1.3・1.5・2.5）。

use std::path::PathBuf;

use areka_sylphya::{PersistKey, PersistScope};
use wintf::AppExit;

use super::*;
use crate::app_exit::FirstExit;
use crate::placement::test_support::{LogEvent, capture_logs};

fn failure(key: PersistKey, reason: &str) -> PersistFailure {
    PersistFailure {
        scope: PersistScope::App,
        dir: Some(PathBuf::from(r"C:\areka\profile\areka")),
        keys: vec![key],
        cause: PersistFailureCause::Io(reason.to_owned()),
    }
}

/// 据え付け済みの World（`Schedules` と終了の受け口つき）。
fn installed_world() -> World {
    let mut world = World::new();
    world.insert_resource(Schedules::default());
    world.insert_non_send(AppExit::new());
    install(&mut world);
    world
}

fn memory_write_failed(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("memory_write_failed"))
        .collect()
}

/// 失敗 1 件の汲み出し: 最初の出所が `MemoryWriteFailed`・`memory_write_failed` が 1 件
/// （`error!`・欄 scope・what・dir・reason・moment=running）。
#[test]
fn drain_one_failure_quits_with_the_new_origin_and_logs_once() {
    let mut world = installed_world();
    let first = failure(
        PersistKey::LastRunning,
        "アクセスが拒否されました。 (os error 5)",
    );
    sender(&world)
        .expect("据え付け後は送り手が在る")
        .send(first.clone())
        .unwrap();

    let (_, events) = capture_logs(|| drain_system(&mut world));

    assert_eq!(
        world.get_resource::<FirstExit>().map(|f| f.0.clone()),
        Some(ExitOrigin::MemoryWriteFailed(first.clone())),
    );
    let logs = memory_write_failed(&events);
    assert_eq!(
        logs.len(),
        1,
        "memory_write_failed はちょうど 1 件: {events:?}"
    );
    let log = logs[0];
    assert_eq!(log.level, tracing::Level::ERROR);
    assert!(log.field("scope").is_some_and(|s| s.contains("App")));
    assert!(
        log.field("what")
            .is_some_and(|w| w.contains(&remembered_text(&first)))
    );
    assert!(log.field("dir").is_some_and(|d| d.contains("profile")));
    assert!(
        log.field("reason")
            .is_some_and(|r| r.contains("os error 5"))
    );
    assert_eq!(log.field_str("moment"), Some("running"));
}

/// 失敗 2 件の汲み出し: 最初の出所は 1 件目のまま（要件 2.5）・記録は 1 件ずつ。
#[test]
fn drain_two_failures_keeps_the_first_origin() {
    let mut world = installed_world();
    let first = failure(PersistKey::LastRunning, "first");
    let second = failure(PersistKey::LastRunning, "second");
    let tx = sender(&world).expect("送り手");
    tx.send(first.clone()).unwrap();
    tx.send(second).unwrap();

    let (_, events) = capture_logs(|| drain_system(&mut world));

    assert_eq!(
        world.get_resource::<FirstExit>().map(|f| f.0.clone()),
        Some(ExitOrigin::MemoryWriteFailed(first)),
    );
    assert_eq!(memory_write_failed(&events).len(), 2, "{events:?}");
}

/// 据えていない World: 送り手は無く、汲み出しは何もしない。
#[test]
fn drain_without_install_does_nothing() {
    let mut world = World::new();
    assert!(sender(&world).is_none());
    drain_system(&mut world);
    assert!(world.get_resource::<FirstExit>().is_none());
}
