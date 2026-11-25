//! 置き場の並びを記憶へ直に書けないときの決定論テスト。
//!
//! 確かめること: ゴーストが居ない World でアプリの記憶の置き場に普通のファイルが在ると、
//! `roots_memory_write_failed`（error・欄 dir・reason）と `memory_write_failed` を 1 件ずつ残し、
//! 最初の出所が `MemoryWriteFailed` になる。書き込みの前に書き換えた起動の文脈の並びは戻さない。

use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::*;
use crate::roots::roots_test_support::{roots_of, world};
use crate::session_tests::events_named;
use areka_session::app_exit::{ExitOrigin, FirstExit};

#[test]
fn apply_without_ghost_write_failure_is_fatal_and_keeps_new_list() {
    let tmp = TempPath::under_target("roots-edit-apply-fatal");
    let a = tmp.child("a");
    let b = tmp.child("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    // 記憶の置き場がファイル＝書けない。
    let profile = tmp.path().join("profile-is-a-file");
    std::fs::write(&profile, "x").unwrap();
    let mut world = world(&a, profile.clone());
    let roots = roots_of(&[&a, &b]);

    let ((), events) = capture(|| apply(&mut world, roots.clone(), None));

    let origin = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    assert!(
        matches!(origin, Some(ExitOrigin::MemoryWriteFailed(_))),
        "最初の出所: {origin:?}"
    );
    assert_eq!(
        world.resource::<BootContext>().roots,
        roots,
        "書き換えた並びは戻さない"
    );
    let failed: Vec<_> = events_named(&events, "roots_memory_write_failed");
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(failed[0].level, tracing::Level::ERROR);
    assert_eq!(
        failed[0].field("dir"),
        Some(profile.display().to_string().as_str())
    );
    assert!(failed[0].field("reason").is_some(), "欄 reason: {events:?}");
    assert_eq!(
        events_named(&events, "memory_write_failed").len(),
        1,
        "{events:?}"
    );
}
