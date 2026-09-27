//! `fn main` の据え付けと後始末・起動中の印（areka-P0-ghost-shell-balloon-switch task 8.5・11.1・
//! 要件 8.1・8.2・12.1〜12.5・12.12 ⑴〜⑶）のテスト。
//!
//! `main` は生きた `WinApp` を要するので、据え付け（[`install_boot_context`]・[`boot_first_ghost`]）と
//! `run()` の後に置き場と文脈から組むもの（[`after_run`]）と、降ろした後の印の始末
//! （[`settle_session_mark`]）を関数に括り出し、ここで踏む。起動中の印は、偽の SHIORI の土台で
//! 実際に起こして落とした一周と、判定の表の両方で見る。降ろすのは必ず有界に行う。
//! 強制終了は「後始末（印の始末）を通さずに降ろす」で見立てる（印が残ることは消す経路を
//! 通らないことと同じ・実物の強制終了は実機サインオフ）。
//!
//! 2026-09-27 に `main_halt_record_tests.rs`（置き換え前の「単独起動の失敗の書き換え」）を改名して
//! 新しい振る舞いへ書き換えた。

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use areka_ghost::BasewareRoot;
use areka_kanade::{BootOrigin, KanadeNotice, KanadeStopCause, ShioriFault, ShioriFaultKind};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, save_scope};
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::{AfterRun, MarkInputs, MarkVerdict, after_run, boot_first_ghost, finish_after_run};
use super::{first_boot_origin, install_boot_context, session_mark_verdict, settle_session_mark};
use crate::ConfigInputs;
use crate::alert::AlertScene;
use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_config::{BootContext, CurrentGhost, RootSource, resolve_boot_from};
use crate::boot_resolve::{
    BalloonDecision, BalloonRoute, DEFAULT_GHOST_FOLDER, GhostDecision, GhostRoute,
    read_last_ghost, read_session_mark, write_session_mark,
};
use crate::emo2_boot::ghost_switch_test_support::{
    BALLOON, FakeShiori, SwitchRig, standard_script,
};
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
        argv_session: ghost.route == GhostRoute::Argv,
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

/// `folder` を最後に起こした回の `OnBoot` の Reference6 以降（Ref6/7 が無ければ `None`）。
fn on_boot_tail(rig: &SwitchRig, folder: &str) -> Option<Vec<String>> {
    rig.calls(folder)
        .last()
        .and_then(|calls| {
            calls.iter().find_map(|c| match c {
                RecordedCall::Get { id, references } if id == "OnBoot" => {
                    Some(references.get(6..).map(<[String]>::to_vec))
                }
                _ => None,
            })
        })
        .flatten()
}

/// 土台の根の記憶の置き場（App スコープ）。
fn app_dir(rig: &SwitchRig) -> PathBuf {
    rig.root.dir().join("app-profile")
}

/// 目録の `folder` の descript の `name`（無ければフォルダ名）＝印に書かれるはずの名前。
fn name_of(rig: &SwitchRig, folder: &str) -> String {
    areka_ghost::catalog::list_ghosts(&rig.root)
        .into_iter()
        .find(|e| e.identity.folder == folder)
        .and_then(|e| e.identity.name)
        .unwrap_or_else(|| folder.to_owned())
}

/// 最後に使ったゴーストの記憶を書く（本番では初回の起動の直後に実行系の書き手が書く）。
fn plant_last_ghost(app_dir: &Path, folder: &str) {
    save_scope(
        PersistScope::App,
        &ScopeRoots {
            app: Some(app_dir.to_path_buf()),
            ..ScopeRoots::default()
        },
        &FsPersistIo,
        vec![(PersistKey::LastGhost, folder.to_owned())],
    );
}

/// `fn main` と同じ形で起動の文脈を据え（argv の旗も）、初回の起動として起こす。
fn first_boot(rig: &mut SwitchRig, ghost: GhostDecision, origin: BootOrigin, argv_session: bool) {
    let cfg = ConfigInputs {
        ghost_root: ghost.dir.clone(),
        balloon_root: rig.root.balloon_dir(BALLOON),
    };
    let balloon = BalloonDecision {
        route: BalloonRoute::Companion,
        dir: cfg.balloon_root.clone(),
        folder: Some(BALLOON.to_owned()),
    };
    rig.world.insert_resource(BootContext {
        root: rig.root.clone(),
        app_profile_dir: app_dir(rig),
        helper_exe: PathBuf::from("main_session_mark_tests/使わない/helper.exe"),
        argv_session,
        current: CurrentGhost {
            cfg: cfg.clone(),
            ghost: ghost.clone(),
            balloon: balloon.clone(),
        },
    });
    boot_first_ghost(
        &mut rig.world,
        &cfg,
        origin,
        &StartupDescriptValues {
            author_dpi: AuthorDpi::DEFAULT,
            zorder_raw: None,
        },
        &ghost,
        &balloon,
    );
}

/// 土台の根の `folder` を記憶の経路で決まったゴーストとして初回の起動で起こす。
fn first_boot_memory(rig: &mut SwitchRig, folder: &str) {
    let ghost = GhostDecision {
        route: GhostRoute::Memory,
        dir: rig.root.ghost_dir(folder),
        folder: Some(folder.to_owned()),
    };
    first_boot(rig, ghost, BootOrigin::Plain, false);
}

/// `run()` の後の後始末を `fn main` と同じ順で踏む: 置き場から取り出し → 降ろす → 印の始末。
/// 戻りは（降ろせたか・印の判定）。
fn after_run_and_settle(rig: &mut SwitchRig) -> (bool, Option<MarkVerdict>) {
    let mut after = after_run(&mut rig.world);
    let down_ok = down(rig, &mut after);
    let verdict = after
        .mark
        .as_ref()
        .map(|mark| settle_session_mark(mark, true, down_ok));
    (down_ok, verdict)
}

/// `run()` の後に取り出した単位を置き場へ戻して有界に降ろす（土台の `shutdown` を使う）。
fn down(rig: &mut SwitchRig, after: &mut AfterRun) -> bool {
    rig.world.insert_non_send(GhostSlot(after.session.take()));
    rig.shutdown()
}

/// 同じ根で次に起動した結果。
#[derive(Debug, PartialEq, Eq)]
struct NextBoot {
    /// 起動前の解決で決まった経路とフォルダ。
    route: GhostRoute,
    folder: Option<String>,
    /// 起動前の解決の 4 つ目（印の値）と、それから組んだ由来。
    found: Option<String>,
    origin: BootOrigin,
    /// 起こした直後の印（起こす前に書かれる）。
    mark_after_boot: Option<String>,
    /// 起きたゴーストの `OnBoot` の Reference6 以降。
    on_boot_tail: Option<Vec<String>>,
    /// 有界に降ろせた。
    down: bool,
}

/// 同じ根で次に起動する（`fn main` と同じ手順: 起動前の解決 → 文脈 → 初回の起動）。
fn next_boot(rig: &mut SwitchRig) -> NextBoot {
    let app = app_dir(rig);
    let (_, ghost, _, found, _) = resolve_boot_from(
        Ok((rig.root.dir().to_path_buf(), RootSource::EnvVar)),
        &["areka.exe".to_owned()],
        &app,
        |n| panic!("無作為の段へ届いてはならない（候補 {n}）"),
    )
    .expect("決まる");
    rig.world.remove_resource::<FirstExit>();
    let folder = ghost
        .folder
        .clone()
        .expect("argv でないのでフォルダ名が在る");
    let origin = first_boot_origin(found.clone());
    first_boot(rig, ghost.clone(), origin.clone(), false);
    let mark_after_boot = read_session_mark(&app);
    let reached = rig.pump_until(|rig| booted(rig, &folder));
    let tail = reached.then(|| on_boot_tail(rig, &folder)).flatten();
    let down = rig.shutdown();
    NextBoot {
        route: ghost.route,
        folder: ghost.folder,
        found,
        origin,
        mark_after_boot,
        on_boot_tail: tail,
        down,
    }
}

/// 印が在る次の起動の期待: 既定が選ばれ、落ちた名前が `fallen`、起こすと印が既定の名前になり、
/// 既定の `OnBoot` に Ref6＝halt・Ref7＝`fallen` が載る。
fn expected_halted_next(rig: &SwitchRig, fallen: &str) -> NextBoot {
    NextBoot {
        route: GhostRoute::Default,
        folder: Some(DEFAULT_GHOST_FOLDER.to_owned()),
        found: Some(fallen.to_owned()),
        origin: BootOrigin::Halted {
            ghost_name: fallen.to_owned(),
        },
        mark_after_boot: Some(name_of(rig, DEFAULT_GHOST_FOLDER)),
        on_boot_tail: Some(vec!["halt".to_owned(), fallen.to_owned()]),
        down: true,
    }
}

/// A と既定ゴーストの 2 体の土台（既定は起動記録つき＝次の起動で `OnBoot` を直接送る）。
fn rig_with(a: FakeShiori) -> SwitchRig {
    let rig = SwitchRig::new(vec![
        ("A", a),
        (
            DEFAULT_GHOST_FOLDER,
            FakeShiori::Scripted(Box::new(|| standard_script("\\0emo2\\e"))),
        ),
    ]);
    rig.plant_boot_record(DEFAULT_GHOST_FOLDER);
    rig
}

fn scripted_a() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e")))
}

// ---------------------------------------------------------------------------
// 据え付け
// ---------------------------------------------------------------------------

/// 据え付けのあと World に起動の文脈と起動入力の作り口が 1 つずつ在り、作り口は本番の入力
/// （helper の結線・渡した由来・停止通知の送出端の写し）を組む。
#[test]
fn install_puts_context_and_one_production_source() {
    let tmp = TempPath::new("mark-install");
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

/// 印の値が在れば初回の起動の由来は「前回落ちた」、無ければ「ふつう」。
#[test]
fn first_boot_origin_follows_session_mark() {
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
// きれいな終わりの判定の表（要件 12.2・12.3・12.5・12.12 ⑴）
// ---------------------------------------------------------------------------

/// 上から順に: argv のセッションは触らない → 出所なし → 失敗の出所 → `run()` の失敗 →
/// 降ろす処理の失敗 → それ以外は消す。
#[test]
fn session_mark_verdict_table() {
    use KanadeStopCause::{CloseSilent, DeadlineExceeded, Forced, Quit};
    let fatal = ExitOrigin::GhostFallbackFailed(fault(ShioriFaultKind::Internal));
    let clean = [
        ExitOrigin::KanadeStopped(Quit),
        ExitOrigin::KanadeStopped(Forced),
        ExitOrigin::KanadeStopped(CloseSilent),
        ExitOrigin::KanadeStopped(DeadlineExceeded),
        ExitOrigin::Escape,
        ExitOrigin::Smoke,
        ExitOrigin::OsClose,
    ];
    let quit = ExitOrigin::KanadeStopped(Quit);
    let mut got: Vec<MarkVerdict> = clean
        .iter()
        .map(|o| session_mark_verdict(Some(o), false, true, true))
        .collect();
    got.extend([
        session_mark_verdict(Some(&kanade_fault()), false, true, true),
        session_mark_verdict(Some(&fatal), false, true, true),
        session_mark_verdict(None, false, true, true),
        session_mark_verdict(Some(&quit), false, false, true),
        session_mark_verdict(Some(&quit), false, true, false),
        // 失敗の出所は run() と降ろす処理の失敗より先に理由になる。
        session_mark_verdict(Some(&kanade_fault()), false, false, false),
        // argv で始まったプロセスは出所・成否によらず触らない（切替の後の致命も）。
        session_mark_verdict(Some(&quit), true, true, true),
        session_mark_verdict(Some(&fatal), true, true, true),
        session_mark_verdict(None, true, false, false),
    ]);
    let mut want = vec![MarkVerdict::Clear; clean.len()];
    want.extend([
        MarkVerdict::Keep("fault"),
        MarkVerdict::Keep("switch_fatal"),
        MarkVerdict::Keep("no_exit_origin"),
        MarkVerdict::Keep("run_failed"),
        MarkVerdict::Keep("down_failed"),
        MarkVerdict::Keep("fault"),
        MarkVerdict::Untouched,
        MarkVerdict::Untouched,
        MarkVerdict::Untouched,
    ]);
    assert_eq!(got, want);
}

/// 判定の始末: 消すなら印が消え、残すなら理由つきの `info!(session_mark_kept)`、触らないなら
/// `debug!(session_mark_untouched_argv)`。どれも記憶の置き場の印をそれ以外に変えない。
#[test]
fn settle_session_mark_applies_the_verdict() {
    let tmp = TempPath::new("mark-settle");
    let mut outcomes = Vec::new();
    for (argv_session, first, level, event) in [
        (
            false,
            Some(ExitOrigin::Smoke),
            tracing::Level::INFO,
            "session_mark_cleared",
        ),
        (
            false,
            Some(kanade_fault()),
            tracing::Level::INFO,
            "session_mark_kept",
        ),
        (
            true,
            Some(ExitOrigin::Smoke),
            tracing::Level::DEBUG,
            "session_mark_untouched_argv",
        ),
    ] {
        let app_profile_dir = tmp.child(event);
        write_session_mark(&app_profile_dir, "A");
        let mark = MarkInputs {
            app_profile_dir: app_profile_dir.clone(),
            argv_session,
            first,
        };
        let (verdict, events) = capture(|| settle_session_mark(&mark, true, true));
        let logged = events
            .iter()
            .filter(|e| e.level == level && e.field_str("event") == Some(event))
            .count();
        let reason = events
            .iter()
            .find(|e| e.field_str("event") == Some("session_mark_kept"))
            .and_then(|e| e.field_str("reason").map(str::to_owned));
        outcomes.push((verdict, logged, reason, read_session_mark(&app_profile_dir)));
    }
    assert_eq!(
        outcomes,
        vec![
            (MarkVerdict::Clear, 1, None, None),
            (
                MarkVerdict::Keep("fault"),
                1,
                Some("fault".to_owned()),
                Some("A".to_owned())
            ),
            (MarkVerdict::Untouched, 1, None, Some("A".to_owned())),
        ],
        "判定の始末が崩れた（判定・記録の件数・残す理由・印）"
    );
}

// ---------------------------------------------------------------------------
// 印の一周（偽の SHIORI の土台で実際に起こして落とす・要件 12.1〜12.4・12.12 ⑵）
// ---------------------------------------------------------------------------

/// 起動系列の途中で A が落ちる（接続に失敗）→ 印＝A が残る（最後のゴーストは A のままでも読まれない）
/// → 同じ根の次の起動で既定が選ばれ、落ちた名前が A、起こすと印が既定の名前になり、
/// 既定の `OnBoot` に Ref6＝halt・Ref7＝A が載る。告知と終了コード 1 は今日どおり。
#[test]
fn boot_fault_keeps_mark_and_next_boot_is_default_with_halt() {
    let mut rig = rig_with(FakeShiori::ConnectFail);
    let app = app_dir(&rig);
    let mark_before = read_session_mark(&app);
    first_boot_memory(&mut rig, "A");
    let mark_at_boot = read_session_mark(&app);
    let exited = rig.pump_until(SwitchRig::exit_requested);

    let mut after = after_run(&mut rig.world);
    let scene_ghost = match &after.scene {
        Some(AlertScene::ShioriFault { ghost_name, .. }) => ghost_name.clone(),
        _ => None,
    };
    let fault_flag = after.fault;
    let down_a = down(&mut rig, &mut after);
    let verdict = after
        .mark
        .as_ref()
        .map(|mark| settle_session_mark(mark, true, down_a));
    let mark_kept = read_session_mark(&app);
    plant_last_ghost(&app, "A");
    let next = next_boot(&mut rig);

    assert_eq!(
        (
            mark_before,
            mark_at_boot,
            exited,
            scene_ghost,
            fault_flag,
            down_a,
            verdict,
            mark_kept,
            read_last_ghost(&app)
        ),
        (
            None,
            Some("A".to_owned()),
            true,
            Some("A".to_owned()),
            true,
            true,
            Some(MarkVerdict::Keep("fault")),
            Some("A".to_owned()),
            Some("A".to_owned()),
        ),
        "起動系列の途中の失敗で印が残らない（起こす前・起こした直後・終了の指示・告知の名前・\
         終了コード 1・降ろせた・判定・残った印・最後のゴースト）"
    );
    assert_eq!(next, expected_halted_next(&rig, "A"), "次の起動");
}

/// 定常に入ったあとの失敗でも印が残り、次の起動は既定＋Ref6/7（起動系列の途中かどうかは見ない）。
#[test]
fn fault_after_steady_keeps_mark() {
    let mut rig = rig_with(scripted_a());
    first_boot_memory(&mut rig, "A");
    let reached = rig.pump_until(|rig| booted(rig, "A"));
    // 定常のあとに SHIORI の失敗で止まった（予約の無い停止通知が今日どおり終了を指示する形）。
    quit_app(
        &mut rig.world,
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault(ShioriFaultKind::Disconnected))),
    );
    let settled = after_run_and_settle(&mut rig);
    plant_last_ghost(&app_dir(&rig), "A");
    let next = next_boot(&mut rig);

    assert_eq!(
        (reached, settled),
        (true, (true, Some(MarkVerdict::Keep("fault")))),
        "定常のあとの失敗で印が残らない（届いた・降ろせた・判定）"
    );
    assert_eq!(next, expected_halted_next(&rig, "A"), "次の起動");
}

/// 後始末を通さずに捨てた（強制終了の見立て）: 印は書いたまま残り、次の起動は既定＋Ref6/7。
#[test]
fn dropped_without_cleanup_keeps_mark() {
    let mut rig = rig_with(scripted_a());
    first_boot_memory(&mut rig, "A");
    let reached = rig.pump_until(|rig| booted(rig, "A"));
    // 印の始末を通さずに降ろすだけ（強制終了では後始末が走らない）。
    let down_a = rig.shutdown();
    plant_last_ghost(&app_dir(&rig), "A");
    let next = next_boot(&mut rig);

    assert_eq!((reached, down_a), (true, true), "届いた・降ろせた");
    assert_eq!(next, expected_halted_next(&rig, "A"), "次の起動");
}

/// 既定ゴースト自身の失敗: 印＝既定の名前が残り、次も既定で落ちた名前は既定の名前。
#[test]
fn default_ghost_fault_keeps_its_own_name() {
    let mut rig = SwitchRig::new(vec![(DEFAULT_GHOST_FOLDER, FakeShiori::ConnectFail)]);
    let app = app_dir(&rig);
    let default_name = name_of(&rig, DEFAULT_GHOST_FOLDER);
    let ghost = GhostDecision {
        route: GhostRoute::Default,
        dir: rig.root.ghost_dir(DEFAULT_GHOST_FOLDER),
        folder: Some(DEFAULT_GHOST_FOLDER.to_owned()),
    };
    first_boot(&mut rig, ghost, BootOrigin::Plain, false);
    let exited = rig.pump_until(SwitchRig::exit_requested);
    let settled = after_run_and_settle(&mut rig);
    let (_, next, _, found, _) = resolve_boot_from(
        Ok((rig.root.dir().to_path_buf(), RootSource::EnvVar)),
        &["areka.exe".to_owned()],
        &app,
        |n| panic!("無作為の段へ届いてはならない（候補 {n}）"),
    )
    .expect("決まる");

    assert_eq!(
        (exited, settled, next.route, next.folder, found),
        (
            true,
            (true, Some(MarkVerdict::Keep("fault"))),
            GhostRoute::Default,
            Some(DEFAULT_GHOST_FOLDER.to_owned()),
            Some(default_name),
        ),
        "既定ゴースト自身の失敗の次の起動が崩れた（終了の指示・降ろせた・判定・経路・フォルダ・落ちた名前）"
    );
}

/// きれいな終わり（メニューの「終了」相当）: 降ろした後に印が消え、次の起動は記憶どおりで由来はふつう。
#[test]
fn clean_exit_clears_mark_and_next_boot_follows_memory() {
    let mut rig = rig_with(scripted_a());
    first_boot_memory(&mut rig, "A");
    let reached = rig.pump_until(|rig| booted(rig, "A"));
    quit_app(
        &mut rig.world,
        ExitOrigin::KanadeStopped(KanadeStopCause::Quit),
    );
    let settled = after_run_and_settle(&mut rig);
    let app = app_dir(&rig);
    let mark_after_exit = read_session_mark(&app);
    plant_last_ghost(&app, "A");
    let next = next_boot(&mut rig);

    assert_eq!(
        (reached, settled, mark_after_exit),
        (true, (true, Some(MarkVerdict::Clear)), None),
        "きれいな終わりで印が消えない（届いた・降ろせた・判定・印）"
    );
    assert_eq!(
        next,
        NextBoot {
            route: GhostRoute::Memory,
            folder: Some("A".to_owned()),
            found: None,
            origin: BootOrigin::Plain,
            mark_after_boot: Some(name_of(&rig, "A")),
            on_boot_tail: None,
            down: true,
        },
        "次の起動"
    );
}

/// argv で始まったプロセス（要件 12.5）: 印を読まず書かず消さない（前から在った印がそのまま残る）。
#[test]
fn argv_session_neither_reads_writes_nor_clears_mark() {
    let mut rig = rig_with(FakeShiori::ConnectFail);
    let app = app_dir(&rig);
    write_session_mark(&app, "前の印");
    let argv_dir = rig.root.ghost_dir("A");
    let found = resolve_boot_from(
        Ok((rig.root.dir().to_path_buf(), RootSource::EnvVar)),
        &["areka.exe".to_owned(), argv_dir.display().to_string()],
        &app,
        |n| panic!("無作為の段へ届いてはならない（候補 {n}）"),
    )
    .expect("argv で決まる")
    .3;
    let ghost = GhostDecision {
        route: GhostRoute::Argv,
        dir: argv_dir,
        folder: None,
    };
    first_boot(&mut rig, ghost, BootOrigin::Plain, true);
    let mark_at_boot = read_session_mark(&app);
    let exited = rig.pump_until(SwitchRig::exit_requested);
    let settled = after_run_and_settle(&mut rig);

    assert_eq!(
        (
            found,
            mark_at_boot,
            exited,
            settled,
            read_session_mark(&app)
        ),
        (
            None,
            Some("前の印".to_owned()),
            true,
            (true, Some(MarkVerdict::Untouched)),
            Some("前の印".to_owned()),
        ),
        "argv の起動が印に触れた（解決の戻り・起こした直後・終了の指示・降ろせた＋判定・後始末の後）"
    );
}

// ---------------------------------------------------------------------------
// 告知と終了コード（切替の途中の致命・失敗でない終了）
// ---------------------------------------------------------------------------

/// 切替の途中の致命（既定へ戻せなかった）: 告知は起こそうとしていた既定ゴーストの名前と場所で
/// 組み（置き場は空・文脈の今のゴーストは最後に起きた A のまま）、印は残す（`switch_fatal`）。
#[test]
fn switch_fatal_names_the_default_ghost_and_keeps_mark() {
    let tmp = TempPath::new("mark-fatal");
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
    let verdict = after
        .mark
        .as_ref()
        .map(|mark| session_mark_verdict(mark.first.as_ref(), mark.argv_session, true, true));
    assert_eq!(
        (after.session.is_none(), scene, verdict, after.fault),
        (
            true,
            Some((Some(DEFAULT_GHOST_FOLDER.to_owned()), expected_root)),
            Some(MarkVerdict::Keep("switch_fatal")),
            true
        ),
        "致命の告知が既定ゴーストを指さないか印を消す（置き場・告知・判定・終了コード 1）"
    );
}

/// Fault 以外の終了（smoke・メニューの終了・出所なし）は告知が無く終了コード 0 のまま。
/// 印の材料は文脈の置き場と最初の出所を運ぶ。
#[test]
fn non_fault_exit_keeps_exit_code_zero() {
    let tmp = TempPath::new("mark-non-fault");
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
            first.clone(),
        );
        let after = after_run(&mut world);
        let code_zero = finish_after_run(Ok(()), after.fault, || Ok(())).is_ok();
        let mark = after
            .mark
            .map(|m| (m.app_profile_dir, m.argv_session, m.first));
        outcomes.push((
            after.session.is_some(),
            after.scene.is_none(),
            mark,
            code_zero,
        ));
    }
    let app = tmp.child("app-profile");
    assert_eq!(
        outcomes,
        vec![
            (
                true,
                true,
                Some((app.clone(), false, Some(ExitOrigin::Smoke))),
                true
            ),
            (
                true,
                true,
                Some((
                    app.clone(),
                    false,
                    Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit))
                )),
                true
            ),
            (true, true, Some((app, false, None)), true),
        ],
        "Fault 以外の終了が変わった（置き場の単位・告知なし・印の材料・終了コード 0）"
    );
}
