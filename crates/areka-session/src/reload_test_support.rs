//! 読み直しのテストが分け合う小道具（`#[test]` を持たない）。

use std::path::Path;
use std::sync::mpsc;

use areka_actor::reply_channel;
use areka_ghost::BasewareRoot;
use areka_runtime::shell_balloon_resolve::SkinCandidate;
use areka_runtime::skin_switch::{SkinSwitchInFlight, SkinSwitchStage};
use bevy_ecs::world::World;

use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::shell_balloon_switch::SkinKind;

/// 起動の文脈（今のゴースト `A`・今のバルーン `kaku`・置き場は `temp` の下）。
pub fn boot_context(temp: &Path, root: &BasewareRoot) -> BootContext {
    BootContext {
        roots: areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
        roots_origin: crate::roots::RootsOrigin::EnvVar,
        app_profile_dir: temp.join("profile"),
        helper_exe: temp.join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("A"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("A"),
                folder: Some("A".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    }
}

/// 他人のシェル／バルーンの切替の予約を直に置く（利用者の `\![change,shell,…]` の代わり）。
pub fn plant_skin_switch(world: &mut World, kind: SkinKind, dir: &Path) {
    world.insert_non_send(SkinSwitchInFlight {
        kind,
        target: SkinCandidate {
            dir: dir.to_path_buf(),
            folder: "planted".to_owned(),
            name: None,
            hidden: false,
        },
        stage: SkinSwitchStage::Waiting {
            gap: Some(reply_channel().1),
            gap_result: None,
            build: mpsc::channel().1,
            built: None,
        },
    });
}
