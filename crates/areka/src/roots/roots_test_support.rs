//! `roots/` のテストが分け合う小道具（`#[test]` を持たない）。

use std::path::{Path, PathBuf};

use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_session::boot_config::BootContext;
use areka_session::reload::test_support::boot_context;
use areka_session::roots::RootsOrigin;
use bevy_ecs::world::World;
use log_capture_kit::CapturedEvent;

/// 根を与えた並びの順に並べた置き場の並び。
pub(super) fn roots_of(dirs: &[&Path]) -> BasewareRoots {
    BasewareRoots::new(
        dirs.iter()
            .map(|d| BasewareRoot::new(d.to_path_buf()))
            .collect(),
    )
}

/// 記録のうち欄 `event` が `name` のもの。
pub(super) fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 根 `root` 1 つの並び（出所は既定）と、アプリの記憶の置き場 `app_profile_dir` を持つ起動の文脈を入れた World。
pub(super) fn world(root: &Path, app_profile_dir: PathBuf) -> World {
    let mut world = World::new();
    world.insert_resource(BootContext {
        roots_origin: RootsOrigin::Default,
        app_profile_dir,
        ..boot_context(root, &BasewareRoot::new(root.to_path_buf()))
    });
    world
}
