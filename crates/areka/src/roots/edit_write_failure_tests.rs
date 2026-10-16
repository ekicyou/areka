//! 置き場の並びを記憶へ直に書けないときの決定論テスト（areka-P0-write-failure-notice task 5.3・要件 1.1・1.5）。
//!
//! 確かめること: ゴーストが居ない World でアプリの記憶の置き場に普通のファイルが在ると、
//! `roots_memory_write_failed`（error・欄 dir・reason）と `memory_write_failed` を 1 件ずつ残し、
//! 最初の出所が `MemoryWriteFailed` になる。書き込みの前に書き換えた起動の文脈の並びは戻さない。

use areka_ghost::{BasewareRoot, BasewareRoots};
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::*;
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::boot_config::{ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};

fn world(root: &Path, app_profile_dir: PathBuf) -> World {
    let mut world = World::new();
    world.insert_resource(BootContext {
        roots: BasewareRoots::single(root.to_path_buf()),
        roots_origin: RootsOrigin::Default,
        app_profile_dir,
        helper_exe: root.join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.join(r"ghost\ga"),
                balloon_root: root.join(r"balloon\B"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.join(r"ghost\ga"),
                folder: Some("ga".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.join(r"balloon\B"),
                folder: Some("B".to_owned()),
            },
        },
    });
    world
}

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
    let roots = BasewareRoots::new(vec![BasewareRoot::new(a.clone()), BasewareRoot::new(b)]);

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
    let failed: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("roots_memory_write_failed"))
        .collect();
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(failed[0].level, tracing::Level::ERROR);
    assert_eq!(
        failed[0].field("dir"),
        Some(profile.display().to_string().as_str())
    );
    assert!(failed[0].field("reason").is_some(), "欄 reason: {events:?}");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("memory_write_failed"))
            .count(),
        1,
        "{events:?}"
    );
}
