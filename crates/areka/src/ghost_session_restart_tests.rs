//! 起こし直しの単位（`ghost_session`）の決定論テスト（areka-P0-ghost-restart-unit）。

use super::*;
use crate::placement::test_support::capture_logs;

/// 作業プール（`WintfTaskPool`）の無い素の World では、窓を作る関数は配置の準備に入らず
/// `TaskPoolMissing` で失敗し、`task_pool_missing` を error で残す（design Testing Strategy
/// 新規テスト 3・log-first の新しい判断分岐）。閉じた証を受ける `reopen_ghost_windows` も
/// 同じ関数へ委譲するので同じ失敗になる。
///
/// ゴーストの根は実在しない経路にしてある——有無の確認が準備より後ろへずれると
/// `Placement` の失敗が返って赤になる（「最初に確かめる」順序もここで固定する）。
/// 判定は集めてから 1 回（面ごとに止めると赤が 1 面しか見えない）。
#[test]
fn open_ghost_windows_without_task_pool_fails_before_preparing() {
    let mut world = World::new();
    let cfg = ConfigInputs {
        ghost_root: std::path::PathBuf::from("ghost_session_restart_tests/無い/ghost"),
        balloon_root: std::path::PathBuf::from("ghost_session_restart_tests/無い/balloon"),
    };

    let ((opened, reopened), events) = capture_logs(|| {
        let opened = open_ghost_windows(&mut world, &cfg);
        let closed = app_exit::close_windows_for_restart(&mut world);
        let reopened = reopen_ghost_windows(&mut world, &cfg, closed);
        (opened, reopened)
    });

    let missing_errors = events
        .iter()
        .filter(|e| {
            e.field_str("event") == Some("task_pool_missing") && e.level == tracing::Level::ERROR
        })
        .count();
    assert_eq!(
        (
            matches!(opened, Err(OpenWindowsError::TaskPoolMissing)),
            matches!(reopened, Err(OpenWindowsError::TaskPoolMissing)),
            missing_errors,
        ),
        (true, true, 2),
        "作業プールの欠落を最初に確かめて失敗していない: opened={:?} reopened={:?} events={events:?}",
        opened.as_ref().err(),
        reopened.as_ref().err(),
    );
}
