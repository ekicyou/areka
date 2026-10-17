//! 別の出所で終わり始めた後に届いた書けない失敗のテスト（areka-P0-write-failure-notice
//! 最終検証の指摘・要件 1.1・1.2・2.3・2.5）。
//!
//! 最初の出所が別の物（`Escape` など）に決まった後の失敗を [`fatal`] へ渡すと、出所は
//! 最初の物のまま残り、記録だけ残って告知が出ない。汲み出しはそれを受け手に残し、後始末が
//! [`take_receiver`] → [`tell_at_close`] の降ろす途中の失敗として告げる。

use std::path::PathBuf;

use areka_sylphya::{PersistKey, PersistScope};
use wintf::AppExit;

use super::*;
use crate::app_exit::FirstExit;
use crate::placement::test_support::{LogEvent, capture_logs};

fn failure(reason: &str) -> PersistFailure {
    PersistFailure {
        scope: PersistScope::App,
        dir: Some(PathBuf::from(r"C:\areka\profile\areka")),
        keys: vec![PersistKey::LastRunning],
        cause: PersistFailureCause::Io(reason.to_owned()),
    }
}

fn installed_world() -> World {
    let mut world = World::new();
    world.insert_resource(Schedules::default());
    world.insert_non_send(AppExit::new());
    install(&mut world);
    world
}

fn count(events: &[LogEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// `Escape` で終わり始めた後の失敗: 汲み出しは記録も出所の差し替えもせず受け手に残す。
/// 後始末で取り出すと、告知 1 件（「記憶を書けません」）・記録 1 件・`Err(E_FAIL)`。
#[test]
fn a_failure_after_another_origin_is_told_at_close() {
    let mut world = installed_world();
    quit_app(&mut world, ExitOrigin::Escape);
    sender(&world).unwrap().send(failure("late")).unwrap();

    let (_, drained) = capture_logs(|| drain_system(&mut world));

    assert_eq!(count(&drained, "memory_write_failed"), 0, "{drained:?}");
    assert_eq!(
        world.get_resource::<FirstExit>().map(|f| f.0.clone()),
        Some(ExitOrigin::Escape),
    );

    let late = take_receiver(&mut world);
    let (told, events) = capture_logs(|| tell_at_close(None, None, late, true));

    assert_eq!(
        told.expect_err("告げたので失敗").code(),
        windows::Win32::Foundation::E_FAIL
    );
    assert_eq!(count(&events, "memory_write_failed"), 1, "{events:?}");
    let alerts: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("alert"))
        .collect();
    assert_eq!(alerts.len(), 1, "{events:?}");
    assert_eq!(
        alerts[0].field_str("title"),
        Some("areka の記憶を書けません")
    );
}

/// 最初の出所が書けない失敗なら、続く失敗は今までどおり汲み出して 1 件ずつ記録する。
#[test]
fn a_failure_after_a_write_failure_is_still_drained() {
    let mut world = installed_world();
    quit_app(&mut world, ExitOrigin::MemoryWriteFailed(failure("first")));
    sender(&world).unwrap().send(failure("second")).unwrap();

    let (_, events) = capture_logs(|| drain_system(&mut world));

    assert_eq!(count(&events, "memory_write_failed"), 1, "{events:?}");
    assert!(take_receiver(&mut world).unwrap().try_recv().is_err());
}

/// OS のセッションの終了の後は告げない（要件 2.3 の例外）ので、今までどおり汲み出して記録する。
#[test]
fn a_failure_after_os_session_end_is_drained_and_logged() {
    let mut world = installed_world();
    world.insert_resource(crate::session_end::SessionEnded);
    quit_app(&mut world, ExitOrigin::SessionEnd);
    sender(&world).unwrap().send(failure("late")).unwrap();

    let (_, events) = capture_logs(|| drain_system(&mut world));

    assert_eq!(count(&events, "memory_write_failed"), 1, "{events:?}");
    assert_eq!(
        world.get_resource::<FirstExit>().map(|f| f.0.clone()),
        Some(ExitOrigin::SessionEnd),
    );
}
