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

// ---------------------------------------------------------------------------
// task 6.2: 印の 4 つの形（要件 3.1〜3.4・7.2・7.6）。置き場は普通のファイルにして書けなくする。

/// 置き場の位置に普通のファイルを置く（書けない）。
fn unwritable(dir: &std::path::Path) {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir).expect("置き場のフォルダを外す");
    }
    std::fs::write(dir, "x").expect("置き場の位置に普通のファイルを置く");
}

/// 置き場をフォルダへ戻す（書ける）。
fn writable(dir: &std::path::Path) {
    if dir.is_file() {
        std::fs::remove_file(dir).expect("置き場の位置のファイルを外す");
    }
    std::fs::create_dir_all(dir).expect("置き場のフォルダを作る");
}

/// 最初の出所が `MemoryWriteFailed` か。
fn first_is_write_failed(world: &World) -> bool {
    matches!(
        world.get_resource::<FirstExit>().map(|f| &f.0),
        Some(ExitOrigin::MemoryWriteFailed(_))
    )
}

/// 置き場（`GhostSlot`）が空か（据えていないのも空）。
fn slot_empty(world: &World) -> bool {
    world
        .get_non_send::<GhostSlot>()
        .is_none_or(|slot| slot.0.is_none())
}

fn reservation(
    stage: crate::emo2_boot::ghost_switch::SwitchStage,
) -> crate::emo2_boot::ghost_switch::SwitchInFlight {
    use crate::emo2_boot::ghost_switch::{PrevGhost, SwitchInFlight, SwitchTarget};
    SwitchInFlight {
        target: SwitchTarget {
            dir: "ghost/B".into(),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: "ghost/A".into(),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage,
        boot_event: None,
        copy: crate::emo2_boot::switch_copy::SwitchCopy::None,
    }
}

/// ⑴ 起動の時点（由来 `Plain`）・⑵ 前回の印が残る起動（由来 `Halted`）で印を書けない
/// → `boot_first_ghost` が App・起動中の印の失敗を返し、ゴーストを起こさない（置き場が空・要件 3.1・3.2）。
/// 作り口は据えていないので、起こしに進めば panic で赤になる。
#[test]
fn boot_mark_unwritable_does_not_boot_for_plain_and_halted() {
    let origins = [
        areka_kanade::BootOrigin::Plain,
        areka_kanade::BootOrigin::Halted {
            ghost_name: "落ちたゴースト".to_owned(),
        },
    ];
    for origin in origins {
        let tmp = TempPath::under_target("mark-write-boot");
        let mut world = world_with(&tmp, ExitOrigin::Escape);
        world.remove_non_send::<GhostSlot>();
        let (cfg, ghost, balloon, app) = {
            let ctx = world.resource::<BootContext>();
            (
                ctx.current.cfg.clone(),
                ctx.current.ghost.clone(),
                ctx.current.balloon.clone(),
                ctx.app_profile_dir.clone(),
            )
        };
        unwritable(&app);
        let descript = crate::ghost_session::StartupDescriptValues {
            author_dpi: crate::placement::AuthorDpi::DEFAULT,
            zorder_raw: None,
            default_surfaces: crate::placement::DefaultSurfaces::builtin(),
        };
        let label = format!("{origin:?}");
        let boot = super::boot_first_ghost(&mut world, &cfg, origin, &descript, &ghost, &balloon);
        let failure = boot.expect_err(&format!("{label}: 印を書けない起動は失敗を返す"));
        assert_eq!(
            (
                failure.scope,
                failure.keys.contains(&PersistKey::LastRunning),
                slot_empty(&world)
            ),
            (PersistScope::App, true, true),
            "{label}: (スコープ・起動中の印の key・置き場が空)"
        );
    }
}

/// ⑶ 定常の印の投函が失敗 → 汲み出しで最初の出所が `MemoryWriteFailed`・予約が無い／
/// 切替の降ろした直後に書けない → 切替先を起こさず同じ（要件 3.3）。
#[test]
fn switch_marks_unwritable_end_with_write_failed_and_no_reservation() {
    use crate::emo2_boot::ghost_switch::{
        SwitchInFlight, SwitchStage, WelcomeAttempt, on_notice, switch_to,
    };
    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.wire_app_memory();
    rig.boot("A");
    let app = rig.app_dir();
    let publisher = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.runtime())
        .expect("A の実行系が起きている")
        .sylphya_publisher()
        .clone();
    publisher.barrier().expect("起動の書き込みが済む");

    // 定常の印（投函）: 机へ届くまでは終わらせず、汲み出しで終わる。
    unwritable(&app);
    rig.world
        .insert_non_send(reservation(SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        }));
    on_notice(&mut rig.world, areka_kanade::KanadeNotice::Steady);
    publisher.barrier().expect("記憶の書き手は生きている");
    let before_drain = first_is_write_failed(&rig.world);
    rig.world.run_schedule(wintf::ecs::Update);
    assert_eq!(
        (
            before_drain,
            first_is_write_failed(&rig.world),
            rig.world.get_non_send::<SwitchInFlight>().is_some()
        ),
        (false, true, false),
        "定常の印: (汲み出しの前の出所・汲み出しの後の出所・予約)"
    );

    // 降ろした直後（直）: 書ける置き場で降ろしてから書けなくする。
    writable(&app);
    assert!(rig.shutdown(), "A を降ろせる");
    rig.world.remove_resource::<FirstExit>();
    unwritable(&app);
    rig.world.insert_non_send(reservation(SwitchStage::SendOff));
    switch_to(&mut rig.world, areka_kanade::ChangeHandoff { script: None });
    assert_eq!(
        (
            first_is_write_failed(&rig.world),
            rig.world.get_non_send::<SwitchInFlight>().is_some(),
            slot_empty(&rig.world)
        ),
        (true, false, true),
        "降ろした直後: (出所・予約・置き場が空＝切替先を起こさない)"
    );
}

/// ⑷ 終わるときに印を消せない → 後始末の 2 つ目が失敗・途中の告知が 1 件で `Err`（要件 3.4）。
#[test]
fn clear_mark_unwritable_tells_running_once() {
    let tmp = TempPath::under_target("mark-write-clear");
    let app = tmp.child("app-profile");
    unwritable(&app);
    let (verdict, failed) = super::settle_session_mark(
        &super::MarkInputs {
            app_profile_dir: app,
            argv_session: false,
            first: Some(ExitOrigin::Escape),
            logsink_fallback: false,
        },
        Teardown {
            run_ok: true,
            down_ok: true,
            shiori_cut: false,
        },
    );
    assert_eq!(verdict, MarkVerdict::Clear);
    let failed = failed.expect("印を消せない失敗が返る");
    let (told, events) = log_capture_kit::capture(|| {
        crate::write_failure::tell_at_close(None, Some(failed), None, true)
    });
    let alerts: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("alert"))
        .collect();
    assert_eq!(
        (told.is_err(), alerts.len()),
        (true, 1),
        "(終了コード 1・告知の数): {events:?}"
    );
    let scene = alerts[0].field("scene").unwrap_or_default();
    assert!(
        scene.contains("Running") && scene.contains("LastRunning"),
        "途中の告知で、起動中の印: {scene}"
    );
}
