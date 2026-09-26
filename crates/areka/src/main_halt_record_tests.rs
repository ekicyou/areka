//! `fn main` の据え付けと後始末（areka-P0-ghost-shell-balloon-switch task 8.5・要件 6.8・8.1・8.2・
//! 10.14・11.11）のテスト。
//!
//! `main` は生きた `WinApp` を要するので、据え付け（[`install_boot_context`]・[`boot_first_ghost`]）と
//! `run()` の後に置き場と文脈から組むもの（[`after_run`]）を関数に括り出し、ここで踏む。
//! 単独起動の失敗の記憶の書き換えは、偽の SHIORI の土台で実際に起こして落とした一周と、
//! 置き場と文脈だけを組んだ World の両方で見る。降ろすのは必ず有界に行う。

use std::path::PathBuf;
use std::sync::mpsc;

use areka_ghost::BasewareRoot;
use areka_kanade::{BootOrigin, KanadeNotice, KanadeStopCause, ShioriFault, ShioriFaultKind};
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::{AfterRun, after_run, boot_first_ghost, finish_after_run, first_boot_origin};
use super::{install_boot_context, should_record_halt};
use crate::ConfigInputs;
use crate::alert::AlertScene;
use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_config::{BootContext, CurrentGhost, RootSource, resolve_boot_from};
use crate::boot_resolve::{
    BalloonDecision, BalloonRoute, DEFAULT_GHOST_FOLDER, GhostDecision, GhostRoute,
    read_last_ghost, read_last_halted, record_halt,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;
use crate::ghost_session::{GhostBootInputsSource, GhostSession, GhostSlot, StartupDescriptValues};
use crate::placement::AuthorDpi;

fn fault(kind: ShioriFaultKind) -> ShioriFault {
    ShioriFault {
        kind,
        reason: "テストの失敗".to_owned(),
    }
}

fn kanade_fault() -> ExitOrigin {
    ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault(
        ShioriFaultKind::ConnectFailed,
    )))
}

fn decision(route: GhostRoute, folder: Option<&str>) -> GhostDecision {
    GhostDecision {
        route,
        dir: PathBuf::from("root")
            .join("ghost")
            .join(folder.unwrap_or("argv")),
        folder: folder.map(str::to_owned),
    }
}

/// 根 `tmp`・今のゴースト `ghost`・置き場（`slot`）・最初の出所（`first`）を組んだ World。
fn world_with(
    tmp: &TempPath,
    ghost: GhostDecision,
    slot: Option<GhostSession>,
    first: Option<ExitOrigin>,
) -> World {
    let mut world = World::new();
    world.insert_resource(BootContext {
        root: BasewareRoot::new(tmp.path().to_path_buf()),
        app_profile_dir: tmp.child("app-profile"),
        helper_exe: tmp.child("shiori-host32-helper.exe"),
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: ghost.dir.clone(),
                balloon_root: tmp.child("balloon").join("b"),
            },
            ghost,
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: tmp.child("balloon").join("b"),
                folder: Some("b".to_owned()),
            },
        },
    });
    world.insert_non_send(GhostSlot(slot));
    if let Some(first) = first {
        world.insert_resource(FirstExit(first));
    }
    world
}

/// `folder` を 1 回起こした最後の記録に `basewareversion` まで届いたか（起動系列の終わり）。
fn booted(rig: &SwitchRig, folder: &str) -> bool {
    rig.calls(folder).last().is_some_and(|calls| {
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "basewareversion"))
    })
}

/// `run()` の後に取り出した単位を置き場へ戻して有界に降ろす（土台の `shutdown` を使う）。
fn down(rig: &mut SwitchRig, after: &mut AfterRun) -> bool {
    rig.world.insert_non_send(GhostSlot(after.session.take()));
    rig.shutdown()
}

// ---------------------------------------------------------------------------
// 据え付け
// ---------------------------------------------------------------------------

/// 据え付けのあと World に起動の文脈と起動入力の作り口が 1 つずつ在り、作り口は本番の入力
/// （helper の結線・渡した由来・停止通知の送出端の写し）を組む。
#[test]
fn install_puts_context_and_one_production_source() {
    let tmp = TempPath::new("halt-install");
    let mut world = World::new();
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    let ctx = world_with(&tmp, decision(GhostRoute::Memory, Some("A")), None, None)
        .remove_resource::<BootContext>()
        .expect("文脈");
    install_boot_context(&mut world, ctx, tx);

    let has_context = world.contains_resource::<BootContext>();
    let has_source = world.contains_non_send::<GhostBootInputsSource>();
    let cfg = ConfigInputs {
        ghost_root: tmp.child("ghost").join("B"),
        balloon_root: tmp.child("balloon").join("b"),
    };
    let inputs = (world.non_send::<GhostBootInputsSource>().0)(
        &cfg,
        BootOrigin::Halted {
            ghost_name: "A".to_owned(),
        },
    );
    let helper_wired = matches!(
        &inputs.wiring.shiori,
        areka_ghost::ShioriWiring::Helper { helper_exe } if *helper_exe == tmp.child("shiori-host32-helper.exe")
    );
    let origin = inputs.wiring.boot_origin.clone();
    let roots = (inputs.wiring.ghost_root.clone(), inputs.helper_exe.clone());
    let sent = inputs.kanade_stop.send(KanadeNotice::Steady).is_ok();
    let received = matches!(rx.try_recv(), Ok(KanadeNotice::Steady));

    assert_eq!(
        (
            has_context,
            has_source,
            helper_wired,
            origin,
            roots,
            sent,
            received
        ),
        (
            true,
            true,
            true,
            BootOrigin::Halted {
                ghost_name: "A".to_owned()
            },
            (
                cfg.ghost_root.clone(),
                tmp.child("shiori-host32-helper.exe")
            ),
            true,
            true,
        ),
        "据え付けが本番の作り口にならない（文脈・作り口・helper の結線・由来・根と helper・送出端が受け口へ届く）"
    );
}

/// 前回落ちた名前が在れば初回の起動の由来は「前回落ちた」、無ければ「ふつう」。
#[test]
fn first_boot_origin_follows_halted_name() {
    assert_eq!(
        (
            first_boot_origin(Some("A".to_owned())),
            first_boot_origin(None)
        ),
        (
            BootOrigin::Halted {
                ghost_name: "A".to_owned()
            },
            BootOrigin::Plain
        )
    );
}

// ---------------------------------------------------------------------------
// 単独起動の失敗の記憶（偽の SHIORI の土台で実際に起こして落とす）
// ---------------------------------------------------------------------------

/// 起動系列の途中で A が落ちる（接続に失敗）→ 後始末で既定へ書き換わり A が控えられる →
/// 同じ根で次に起動すると記憶の経路で既定が選ばれ、落ちた名前は 1 回だけ渡る →
/// その名前を由来にした初回の起動で既定の `OnBoot` に Ref6＝halt・Ref7＝A が載る。
#[test]
fn boot_fault_rewrites_memory_and_next_boot_picks_default_once() {
    let mut rig = SwitchRig::new(vec![
        ("A", FakeShiori::ConnectFail),
        (
            DEFAULT_GHOST_FOLDER,
            FakeShiori::Scripted(Box::new(|| standard_script("\\0emo2\\e"))),
        ),
    ]);
    rig.plant_boot_record(DEFAULT_GHOST_FOLDER);
    rig.boot("A");
    let app_dir = rig.world.resource::<BootContext>().app_profile_dir.clone();
    let exited = rig.pump_until(SwitchRig::exit_requested);

    let mut after = after_run(&mut rig.world);
    let halt = after.halt.clone();
    let scene_ghost = match &after.scene {
        Some(AlertScene::ShioriFault { ghost_name, .. }) => ghost_name.clone(),
        _ => None,
    };
    let fault_flag = after.fault;
    let down_a = down(&mut rig, &mut after);
    if let Some((dir, name)) = &halt {
        record_halt(dir, name);
    }
    let remembered = (read_last_ghost(&app_dir), read_last_halted(&app_dir));

    // 同じ根で次の起動解決（記憶の経路で既定・控えは 1 回だけ渡る）。
    let args = vec!["areka.exe".to_owned()];
    let resolve = || {
        resolve_boot_from(
            Ok((rig.root.dir().to_path_buf(), RootSource::EnvVar)),
            &args,
            &app_dir,
            |n| panic!("無作為の段へ届いてはならない（候補 {n}）"),
        )
        .expect("決まる")
    };
    let (cfg, ghost, balloon, halted, _) = resolve();
    let second = resolve().3;
    let next = (ghost.route, ghost.folder.clone(), halted.clone(), second);

    // その由来で既定を起こす（`fn main` の初回の起動と同じ手順）。
    rig.world.remove_resource::<FirstExit>();
    boot_first_ghost(
        &mut rig.world,
        &cfg,
        first_boot_origin(halted),
        &StartupDescriptValues {
            author_dpi: AuthorDpi::DEFAULT,
            zorder_raw: None,
        },
        &ghost,
        &balloon,
    );
    let reached = rig.pump_until(|rig| booted(rig, DEFAULT_GHOST_FOLDER));
    let on_boot_tail = rig
        .calls(DEFAULT_GHOST_FOLDER)
        .last()
        .and_then(|calls| {
            calls.iter().find_map(|c| match c {
                RecordedCall::Get { id, references } if id == "OnBoot" => {
                    Some(references.get(6..).map(<[String]>::to_vec))
                }
                _ => None,
            })
        })
        .flatten();
    let down_default = rig.shutdown();

    assert_eq!(
        (
            exited,
            halt,
            scene_ghost,
            fault_flag,
            down_a,
            remembered,
            next,
            reached,
            on_boot_tail,
            down_default
        ),
        (
            true,
            Some((app_dir.clone(), "A".to_owned())),
            Some("A".to_owned()),
            true,
            true,
            (Some(DEFAULT_GHOST_FOLDER.to_owned()), Some("A".to_owned())),
            (
                GhostRoute::Memory,
                Some(DEFAULT_GHOST_FOLDER.to_owned()),
                Some("A".to_owned()),
                None
            ),
            true,
            Some(vec!["halt".to_owned(), "A".to_owned()]),
            true,
        ),
        "単独起動の失敗が次回の既定の起動へ結ばない（終了の指示・控える組・告知の名前・終了コード 1・\
         降ろせた・記憶・次の解決（経路・フォルダ・名前・2 度目）・既定が起きた・OnBoot の Ref6/7・降ろせた）"
    );
}

/// 定常に入ったあとの失敗でも書き換える（起動系列の途中かどうかは見ない）。
#[test]
fn fault_after_steady_also_rewrites() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.boot("A");
    let reached = rig.pump_until(|rig| booted(rig, "A"));
    // 定常のあとに SHIORI の失敗で止まった（予約の無い停止通知が今日どおり終了を指示する形）。
    quit_app(
        &mut rig.world,
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault(ShioriFaultKind::Disconnected))),
    );
    let app_dir = rig.world.resource::<BootContext>().app_profile_dir.clone();
    let mut after = after_run(&mut rig.world);
    let halt = after.halt.clone();
    let down_ok = down(&mut rig, &mut after);

    assert_eq!(
        (reached, halt, down_ok),
        (true, Some((app_dir, "A".to_owned())), true),
        "定常のあとの失敗で書き換えの組が出ない（届いた・控える組・降ろせた）"
    );
}

// ---------------------------------------------------------------------------
// 書き換えない側（既定・argv・切替の途中の致命・Fault 以外）
// ---------------------------------------------------------------------------

/// 既定ゴースト自身の失敗・argv の起動は書き換えない（0 回・`info!` が 1 件ずつ）。
#[test]
fn default_ghost_and_argv_are_not_rewritten() {
    let tmp = TempPath::new("halt-skip");
    let cases = [
        decision(GhostRoute::Default, Some(DEFAULT_GHOST_FOLDER)),
        decision(GhostRoute::Memory, Some(DEFAULT_GHOST_FOLDER)),
        decision(GhostRoute::Argv, None),
    ];
    let mut halts = Vec::new();
    let mut infos = 0;
    for ghost in cases {
        let mut world = world_with(
            &tmp,
            ghost.clone(),
            Some(GhostSession::for_test(None, ghost.dir.clone())),
            Some(kanade_fault()),
        );
        let (after, events) = capture(|| after_run(&mut world));
        halts.push((after.halt, after.fault));
        infos += events
            .iter()
            .filter(|e| {
                e.level == tracing::Level::INFO
                    && e.field_str("event") == Some("halt_record_skipped")
            })
            .count();
    }
    let app_dir = tmp.child("app-profile");
    assert_eq!(
        (
            halts,
            infos,
            read_last_ghost(&app_dir),
            read_last_halted(&app_dir)
        ),
        (vec![(None, true); 3], 3, None, None),
        "既定・argv で書き換えの組が出る（組と終了コード・info の件数・記憶）"
    );
}

/// 切替の途中の致命（既定へ戻せなかった）: 書き換えず、告知は起こそうとしていた既定ゴーストの
/// 名前と場所で組む（置き場は空・文脈の今のゴーストは最後に起きた A のまま）。
#[test]
fn switch_fatal_names_the_default_ghost_and_does_not_rewrite() {
    let tmp = TempPath::new("halt-fatal");
    let mut world = world_with(
        &tmp,
        decision(GhostRoute::Switched, Some("A")),
        None,
        Some(ExitOrigin::GhostFallbackFailed(fault(
            ShioriFaultKind::Internal,
        ))),
    );
    let after = after_run(&mut world);
    let expected_root =
        std::path::absolute(tmp.child("ghost").join(DEFAULT_GHOST_FOLDER)).expect("絶対パス");
    let scene = after.scene.map(|s| match s {
        AlertScene::ShioriFault {
            ghost_name,
            ghost_root,
            ..
        } => (ghost_name, ghost_root),
        other => panic!("SHIORI の失敗の場面でない: {other:?}"),
    });
    assert_eq!(
        (after.session.is_none(), scene, after.halt, after.fault),
        (
            true,
            Some((Some(DEFAULT_GHOST_FOLDER.to_owned()), expected_root)),
            None,
            true
        ),
        "致命の告知が既定ゴーストを指さないか書き換えの組が出る（置き場・告知・組・終了コード 1）"
    );
}

/// Fault 以外の終了（smoke・メニューの終了）は告知も書き換えも無く、終了コード 0 のまま。
#[test]
fn non_fault_exit_keeps_exit_code_zero() {
    let tmp = TempPath::new("halt-non-fault");
    let mut outcomes = Vec::new();
    for first in [
        Some(ExitOrigin::Smoke),
        Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
        None,
    ] {
        let ghost = decision(GhostRoute::Memory, Some("A"));
        let mut world = world_with(
            &tmp,
            ghost.clone(),
            Some(GhostSession::for_test(None, ghost.dir.clone())),
            first,
        );
        let after = after_run(&mut world);
        let code_zero = finish_after_run(Ok(()), after.fault, || Ok(())).is_ok();
        outcomes.push((
            after.session.is_some(),
            after.scene.is_none(),
            after.halt,
            code_zero,
        ));
    }
    assert_eq!(
        outcomes,
        vec![(true, true, None, true); 3],
        "Fault 以外の終了が変わった（置き場の単位・告知なし・組なし・終了コード 0）"
    );
}

/// 判断そのもの: 書くのは kanade の停止の Fault で・argv でなく・既定でないときだけ。
#[test]
fn should_record_halt_decision_table() {
    let a = decision(GhostRoute::Memory, Some("A"));
    let switched = decision(GhostRoute::Switched, Some("B"));
    let default = decision(GhostRoute::Default, Some(DEFAULT_GHOST_FOLDER));
    let argv = decision(GhostRoute::Argv, None);
    let fatal = ExitOrigin::GhostFallbackFailed(fault(ShioriFaultKind::Internal));
    let got = [
        should_record_halt(Some(&kanade_fault()), &a),
        should_record_halt(Some(&kanade_fault()), &switched),
        should_record_halt(Some(&kanade_fault()), &default),
        should_record_halt(Some(&kanade_fault()), &argv),
        should_record_halt(Some(&fatal), &a),
        should_record_halt(Some(&ExitOrigin::Escape), &a),
        should_record_halt(None, &a),
    ];
    assert_eq!(got, [true, true, false, false, false, false, false]);
}
