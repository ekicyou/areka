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

// ---------------------------------------------------------------------------
// task 4.2: 起動を止める口・後始末の口（要件 2.2・2.3・2.5・2.6・6.1・7.5）

fn alerts(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("alert"))
        .collect()
}

/// 抑止でも失敗が返り、`alert` の記録が 1 件（`suppressed=true`・題名「areka を起動できません」）・
/// `memory_write_failed` が 1 件（moment=boot）（要件 2.6）。
#[test]
fn stop_boot_with_suppressed_still_returns_the_error() {
    let f = failure(PersistKey::LastRunning, "denied");
    let (err, events) = capture_logs(|| stop_boot_with(f, true));

    assert_eq!(err.code(), windows::Win32::Foundation::E_FAIL);
    let a = alerts(&events);
    assert_eq!(a.len(), 1, "{events:?}");
    assert_eq!(a[0].field("suppressed"), Some("true"));
    assert_eq!(a[0].field_str("title"), Some("areka を起動できません"));
    let logs = memory_write_failed(&events);
    assert_eq!(logs.len(), 1, "{events:?}");
    assert_eq!(logs[0].field_str("moment"), Some("boot"));
}

/// 受け手は後始末で取り出せる。OS のセッションの終了を処理済みなら取り出さない。
#[test]
fn take_receiver_is_none_after_os_session_end() {
    let mut world = installed_world();
    world.insert_resource(crate::session_end::SessionEnded);
    assert!(take_receiver(&mut world).is_none());

    let mut world = installed_world();
    assert!(take_receiver(&mut world).is_some());
    assert!(take_receiver(&mut world).is_none(), "取り出しは 1 回だけ");
}

/// 記録だけの口: 印を消せない＋受け手の残り 1 件 → 記録 2 件・告知 0 件（要件 2.3 の例外）。
#[test]
fn log_only_logs_both_and_tells_nothing() {
    let mut world = installed_world();
    sender(&world)
        .unwrap()
        .send(failure(PersistKey::LastGhost, "late"))
        .unwrap();
    let clear = failure(PersistKey::LastRunning, "clear");

    let (_, events) = capture_logs(|| log_only(&mut world, Some(clear)));

    assert_eq!(memory_write_failed(&events).len(), 2, "{events:?}");
    assert!(alerts(&events).is_empty(), "{events:?}");
}

fn late_with(failures: Vec<PersistFailure>) -> Option<Receiver<PersistFailure>> {
    let (tx, rx) = channel();
    for f in failures {
        tx.send(f).unwrap();
    }
    Some(rx)
}

/// 出所の場面＋印を消せない＋残りの失敗 → 告げるのは出所の場面 1 件・記録は 2 件・`Ok`。
#[test]
fn tell_at_close_prefers_the_origin_scene() {
    let scene = AlertScene::StartupWindow {
        reason: "origin".to_owned(),
    };
    let (result, events) = capture_logs(|| {
        tell_at_close(
            Some(scene),
            Some(failure(PersistKey::LastRunning, "clear")),
            late_with(vec![failure(PersistKey::LastGhost, "late")]),
            true,
        )
    });

    assert!(result.is_ok());
    let a = alerts(&events);
    assert_eq!(a.len(), 1, "{events:?}");
    assert!(
        a[0].field("scene")
            .is_some_and(|s| s.contains("StartupWindow"))
    );
    assert_eq!(memory_write_failed(&events).len(), 2, "{events:?}");
}

/// 場面なし＋印を消せない → `Running` の告知 1 件で `Err`（抑止でも変わらない・要件 2.6）。
#[test]
fn tell_at_close_tells_the_clear_failure_as_running() {
    let (result, events) = capture_logs(|| {
        tell_at_close(
            None,
            Some(failure(PersistKey::LastRunning, "clear")),
            late_with(vec![failure(PersistKey::LastGhost, "late")]),
            true,
        )
    });

    assert_eq!(
        result.unwrap_err().code(),
        windows::Win32::Foundation::E_FAIL
    );
    let a = alerts(&events);
    assert_eq!(a.len(), 1, "{events:?}");
    let scene = a[0].field("scene").unwrap_or_default();
    assert!(
        scene.contains("Running") && scene.contains("clear"),
        "{scene}"
    );
}

/// 場面なし＋印は消せた＋残りの失敗 → 残りの最初の 1 件を `Running` で告げて `Err`。
#[test]
fn tell_at_close_tells_the_first_late_failure() {
    let (result, events) = capture_logs(|| {
        tell_at_close(
            None,
            None,
            late_with(vec![
                failure(PersistKey::LastGhost, "late1"),
                failure(PersistKey::LastGhost, "late2"),
            ]),
            true,
        )
    });

    assert!(result.is_err());
    let a = alerts(&events);
    assert_eq!(a.len(), 1, "{events:?}");
    assert!(a[0].field("scene").is_some_and(|s| s.contains("late1")));
    assert_eq!(memory_write_failed(&events).len(), 2, "{events:?}");
}

/// 何も無い → 告知 0 件で `Ok`（要件 6.1・7.5）。
#[test]
fn tell_at_close_with_nothing_tells_nothing() {
    let (result, events) = capture_logs(|| tell_at_close(None, None, late_with(vec![]), true));
    assert!(result.is_ok());
    assert!(alerts(&events).is_empty(), "{events:?}");
    assert!(memory_write_failed(&events).is_empty(), "{events:?}");
}
