//! 記憶を書けないときの起動中の印と終わり方（areka-P0-write-failure-notice 要件 2.2・3.4・7.2・D12）の
//! テスト。`main` は生きた `WinApp` を要するので、`run()` の後に組むもの（[`after_run`]）と
//! 終了コードの判断（[`finish_after_run`]）を踏む。

use std::path::PathBuf;

use areka_sylphya::{PersistFailure, PersistFailureCause, PersistKey, PersistScope};
use bevy_ecs::world::World;
use temp_path_kit::TempPath;

use super::{MarkVerdict, Teardown, after_run, finish_after_run, session_mark_verdict};
use crate::ConfigInputs;
use crate::alert::{AlertScene, WriteMoment};
use crate::app_exit::{ExitOrigin, FirstExit};
use crate::boot_config::{BootContext, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};

/// 根 `tmp`・ゴースト A・置き場に単位 1 つ・最初の出所 `first` を組んだ World。
fn world_with(tmp: &TempPath, first: ExitOrigin) -> World {
    let ghost_dir = PathBuf::from("root").join("ghost").join("A");
    let balloon_dir = tmp.child("balloon").join("b");
    let mut world = World::new();
    world.insert_resource(BootContext {
        roots: areka_ghost::BasewareRoots::single(tmp.path().to_path_buf()),
        roots_origin: crate::roots::RootsOrigin::EnvVar,
        app_profile_dir: tmp.child("app-profile"),
        helper_exe: tmp.child("shiori-host32-helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: ghost_dir.clone(),
                balloon_root: balloon_dir.clone(),
            },
            ghost: GhostDecision {
                route: GhostRoute::Memory,
                dir: ghost_dir.clone(),
                folder: Some("A".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: balloon_dir,
                folder: Some("b".to_owned()),
            },
        },
    });
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(None, ghost_dir))));
    world.insert_resource(FirstExit(first));
    world
}

/// 記憶を書けなかった終わり（出所 `MemoryWriteFailed`・要件 2.2・D12）: 途中の「記憶を書けない」
/// 告知を組み、終了コードは 1、印は消す（降ろす手順は正規の道を通り、ゴーストは落ちていない）。
#[test]
fn memory_write_failed_exit_alerts_running_and_exits_nonzero() {
    let tmp = TempPath::under_target("mark-write-failed");
    let failure = PersistFailure {
        scope: PersistScope::Ghost,
        dir: Some(tmp.child("ghost-profile")),
        keys: vec![PersistKey::BootCount],
        cause: PersistFailureCause::Io("アクセスが拒否されました。".to_owned()),
    };
    let mut world = world_with(&tmp, ExitOrigin::MemoryWriteFailed(failure.clone()));
    let after = after_run(&mut world);
    let end = Teardown {
        run_ok: true,
        down_ok: true,
        shiori_cut: false,
    };
    let verdict = after
        .mark
        .as_ref()
        .map(|m| session_mark_verdict(m.first.as_ref(), m.argv_session, false, end));
    let code_nonzero = finish_after_run(Ok(()), after.fault, || Ok(())).is_err();
    assert_eq!(
        (after.scene, verdict, code_nonzero),
        (
            Some(AlertScene::MemoryWrite {
                failure,
                moment: WriteMoment::Running
            }),
            Some(MarkVerdict::Clear),
            true
        ),
        "記憶を書けなかった終わりが崩れた（途中の告知・印の判定・終了コード 1）"
    );
}
