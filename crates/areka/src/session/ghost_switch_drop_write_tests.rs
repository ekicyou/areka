//! 切替の降ろした直後の記憶を書けないときの決定論テスト（areka-P0-write-failure-notice task 5.3・要件 1.1・1.5・3.3）。
//!
//! 確かめること: アプリの記憶の置き場に普通のファイルが在って降ろした直後の記憶を書けないと、
//! 切替の予約を下ろし、最初の出所が `MemoryWriteFailed` になって終了が指示され、
//! 切替先を起こさない（窓を閉じない・起こさない・既定へ戻さない）。

use areka_kanade::ChangeHandoff;
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::fallback_tests::{count_event, root_with, switching_world};
use super::*;
use crate::app_exit::{ExitOrigin, FirstExit};

#[test]
fn switch_drop_write_failure_is_fatal_and_does_not_boot_target() {
    let tmp = TempPath::under_target("ghost-switch-drop-unwritable");
    let root = root_with(&tmp, &["A", "B", crate::boot_resolve::DEFAULT_GHOST_FOLDER]);
    let mut world = switching_world(&root, "B");
    // アプリの記憶の置き場がファイル＝書けない。
    let profile = world.resource::<BootContext>().app_profile_dir.clone();
    std::fs::write(&profile, "x").expect("置き場の位置にファイルを置く");

    let ((), events) = capture(|| switch_to(&mut world, ChangeHandoff { script: None }));

    let origin = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    assert!(
        matches!(origin, Some(ExitOrigin::MemoryWriteFailed(_))),
        "最初の出所: {origin:?}"
    );
    assert_eq!(
        (
            world.non_send::<AppExit>().is_requested(),
            world.get_non_send::<SwitchInFlight>().is_some(),
            count_event(&events, "switch_drop_record_degraded"),
            count_event(&events, "memory_write_failed"),
            count_event(&events, "windows_closed_for_restart"),
            count_event(&events, "ghost_switch_no_context"),
            count_event(&events, "ghost_switch_fatal"),
        ),
        (true, false, 1, 1, 0, 0, 0),
        "(終了の指示・予約・書けない記録・致命の記録・窓を閉じた・起こす試み・切替の致命): {events:?}"
    );
}
