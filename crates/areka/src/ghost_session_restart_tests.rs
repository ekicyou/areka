//! 起こし直しの単位（`ghost_session`）の決定論テスト（areka-P0-ghost-restart-unit）。

use std::rc::Rc;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use areka_kanade::CloseReason;
use sample_ghost_kit::SampleRoot;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::AppExit;
use wintf::ecs::{Input, Update};

use super::*;
use crate::boot_resolve::{BalloonRoute, GhostRoute};
use crate::emo2_boot::frame::KanadeNoticeRx;
use crate::emo2_boot::sample_test_support::acquire_emo2;
use crate::emo2_boot::spine::{SpineHarness, run_bounded};
use crate::input_events::user_break::UserBreakWiring;
use crate::menu::{Frame, ItemBody, MenuContext, MenuItem, MenuWiring};
use crate::placement::spawn::GhostWindowMarker;
use crate::placement::test_support::capture_logs;
use crate::readme::ReadmeWiring;

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

/// 作業プールの閉包を回す `Input` 段を、`done` が真になるまで（10 秒まで）回す。
/// 閉包は作業プールの別スレッドから届くので、届くまで待つ（有界・期限切れは `false`）。
fn run_input_until(world: &mut World, done: impl Fn(&mut World) -> bool) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        world.run_schedule(Input);
        if done(world) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::yield_now();
    }
}

fn ghost_window_count(world: &mut World) -> usize {
    world
        .query_filtered::<(), bevy_ecs::query::With<GhostWindowMarker>>()
        .iter(world)
        .count()
}

/// 作業プールの閉包が届いたことの目印（準備だけで窓が生えないことの対照）。
#[derive(bevy_ecs::resource::Resource)]
struct SentinelArrived;

/// 窓を作る手順は「準備」と「投函」に分かれる（要件 4.10・6.1・design「ghost_session.rs」）。
///
/// 準備（[`prepare_ghost_windows`]）だけでは窓は 1 枚も生えない——準備の後に作業プールへ
/// 投げた目印の閉包が `Input` 段で届いても、窓は 0 枚のまま。投函
/// （[`commit_ghost_windows`]）の後、次の `Input` 段で窓が生える（emo2 はスコープ 2 つ＝
/// キャラ窓とバルーン窓で 4 枚）。
/// 判定は集めてから 1 回。
#[test]
fn prepare_spawns_no_window_until_commit() {
    // SAFETY: 資産の採寸（WIC）に要る COM 初期化（既初期化の S_FALSE 等は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let sample = acquire_emo2();
    let cfg = ConfigInputs {
        ghost_root: sample.folder().to_path_buf(),
        balloon_root: emo2_balloon(&sample),
    };
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(WintfTaskPool::new());
    world.resource_mut::<Schedules>().add_systems(
        Input,
        wintf::ecs::widget::bitmap_source::systems::drain_task_pool_commands,
    );

    let prepared = prepare_ghost_windows(&mut world, &cfg).expect("emo2 の窓の準備は通る");
    world
        .resource::<WintfTaskPool>()
        .spawn(|tx: CommandSender| async move {
            let _ = tx.send(Box::new(|world: &mut World| {
                world.insert_resource(SentinelArrived);
            }));
        });
    let sentinel_arrived =
        run_input_until(&mut world, |w| w.contains_resource::<SentinelArrived>());
    let after_prepare = ghost_window_count(&mut world);

    commit_ghost_windows(&mut world, prepared);
    let spawned = run_input_until(&mut world, |w| ghost_window_count(w) > 0);
    let after_commit = ghost_window_count(&mut world);

    assert_eq!(
        (sentinel_arrived, after_prepare, spawned, after_commit),
        (true, 0, true, 4),
        "準備だけで窓が生えた／投函の後に窓が生えない（目印が届いた・準備後の窓の数・\
         投函後に生えた・投函後の窓の数）"
    );
}

/// 投函の時点で作業プールが無ければ、閉包を捨てた上で `task_pool_missing` を error で残す
/// （黙って捨てない・log-first の判断分岐）。
#[test]
fn commit_without_task_pool_logs_error() {
    let mut world = World::new();
    let prepared = PreparedWindows {
        descript: StartupDescriptValues {
            author_dpi: placement::AuthorDpi::DEFAULT,
            zorder_raw: None,
        },
        spawn: Box::new(|_: &mut World| {}),
    };
    let ((), events) = capture_logs(|| commit_ghost_windows(&mut world, prepared));
    let missing_errors = events
        .iter()
        .filter(|e| {
            e.field_str("event") == Some("task_pool_missing") && e.level == tracing::Level::ERROR
        })
        .count();
    assert_eq!(
        missing_errors, 1,
        "作業プールの欠落が記録されない: {events:?}"
    );
}

/// 偽の SHIORI（標準の台本）・検体の複製・時計なし・記憶の置き場なしで入力の束を組む。
/// 台本は呼ぶたびに新しい（周ごとに `OnInitialize` から `Unload` までを 1 本ずつ消費する）。
fn scripted_inputs(sample: &SampleRoot, kanade_stop: Sender<KanadeNotice>) -> GhostBootInputs {
    let (backend, _handle) = SpineHarness::standard_backend("\\s[0]\\e");
    GhostBootInputs {
        wiring: emo2_boot::Emo2BootInputs {
            ghost_root: sample.folder().to_path_buf(),
            balloon_root: emo2_balloon(sample),
            shiori: areka_ghost::ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn areka_kanade::ShioriBackend>)
            })),
            ticker: areka_ghost::TickerMode::Disabled,
            app_profile_dir: None,
            boot_origin: areka_kanade::BootOrigin::Plain,
        },
        // 結線ありの腕を通すので使われない（fallback に落ちたら判定の説明書の面が赤になる）。
        helper_exe: PathBuf::from("ghost_session_restart_tests/使わない/helper.exe"),
        kanade_stop,
    }
}

fn emo2_balloon(sample: &SampleRoot) -> PathBuf {
    sample
        .balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}

/// 1 周分の結線（argv で決まった体の決定を合成する・design 議題 7）。
fn boot_round(
    world: &mut World,
    sample: &SampleRoot,
    kanade_stop: Sender<KanadeNotice>,
) -> GhostSession {
    let descript = StartupDescriptValues {
        author_dpi: placement::AuthorDpi::DEFAULT,
        zorder_raw: None,
    };
    let ghost = GhostDecision {
        route: GhostRoute::Argv,
        dir: sample.folder().to_path_buf(),
        folder: None,
    };
    let balloon = BalloonDecision {
        route: BalloonRoute::Argv,
        dir: emo2_balloon(sample),
        folder: None,
    };
    boot_ghost(
        world,
        scripted_inputs(sample, kanade_stop),
        &descript,
        &ghost,
        &balloon,
    )
}

/// 降ろすのを有界に行う（seriko の join を含む・hang したら期限切れで赤）。
fn shutdown_bounded(what: &str, session: GhostSession) {
    run_bounded(what, Duration::from_secs(20), move || {
        session
            .shutdown(CloseReason::User { scope: 0 })
            .expect("降ろすのは成功する");
    });
}

/// `Input`／`Update`／`FrameFinalize` の各段に載っている系の数。
fn systems_lens(world: &World) -> [usize; 3] {
    let schedules = world.resource::<Schedules>();
    [
        schedules.get(Input).map_or(0, |s| s.systems_len()),
        schedules.get(Update).map_or(0, |s| s.systems_len()),
        schedules.get(FrameFinalize).map_or(0, |s| s.systems_len()),
    ]
}

/// 同じプロセスで 2 周する（design Testing Strategy 新規テスト 1・要件 6.1・3.2・1.6・2.2〜2.4）。
///
/// 1 周目を降ろして全窓を閉じ、2 周目を結線した後に、⑴ 系が二重に登録されていない
/// ⑵ 状態が 2 周目のものへ載せ替わっている（説明書の経路が 2 周目の根の下・メニューの登記が
/// 組込 2 項目と「ゴースト」枠だけ・停止通知の受け口と中断の旗の受信端が生きた送出端につながっている）
/// ⑶ 終了が指示されていない、を確かめる。
///
/// 「つながっている」は `try_recv` を `Err` が出るまで回して最後が `Empty`（1 周目の受信端は
/// 未読の値を残したまま送出端が落ちて `Disconnected` になる）。停止通知の受け口は
/// プロセスに 1 つなので、テストが持つ送出端の写しを落とした後は 2 周目の kanade の送出端
/// だけがこれを生かす。`MouseWiring` 等は生死を副作用無しに問う口が無い（0 面）が、
/// 上の面と同じ結線ありの腕の直線の手順で挿される。判定は集めてから 1 回。
#[test]
fn boots_twice_in_one_process_without_double_registration() {
    // SAFETY: 資産の焼き込み（WIC）に要る COM 初期化（既初期化の S_FALSE 等は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(AppExit::new());
    world.spawn(GhostWindowMarker);
    world.spawn(GhostWindowMarker);

    let (kanade_stop_tx, kanade_stop_rx) = mpsc::channel::<KanadeNotice>();
    register_systems(&mut world, kanade_stop_rx);
    let lens_before = systems_lens(&world);

    // ── 1 周目 ──
    let sample1 = acquire_emo2();
    let session1 = boot_round(&mut world, &sample1, kanade_stop_tx.clone());
    // 1 周目のメニューへ余分な登記（組込の使わない枠へ・残れば登記の一覧に見える）。
    crate::menu::register(
        &mut world,
        Frame::Shell,
        Rc::new(|_: &World, _: &MenuContext| MenuItem {
            label: "1 周目の余分な登記".to_owned(),
            caption_resource: None,
            enabled: true,
            checked: None,
            body: ItemBody::Action(Rc::new(|_: &mut World, _: &MenuContext| {})),
        }),
    );
    // 1 周目が結線ありの腕を通ったことの証（fallback に落ちると説明書・メニューの状態が
    // 挿されず、2 周目の「載せ替わった」面が素通りで緑になる）。
    let round1_readme_under_root1 = world
        .get_non_send::<ReadmeWiring>()
        .is_some_and(|w| w.path().starts_with(sample1.folder()));
    let round1_frames = world
        .get_non_send::<MenuWiring>()
        .map(|w| w.registry.registered_frames());
    shutdown_bounded("1 周目を降ろす", session1);
    // 窓を作る側は通さないので証は読んで落とす（消費先の型検査は本番の呼び手側で効く）。
    let closed = app_exit::close_windows_for_restart(&mut world);
    let closed_count = closed.closed();

    // ── 2 周目（新しい台本・新しい複製） ──
    let sample2 = acquire_emo2();
    let session2 = boot_round(&mut world, &sample2, kanade_stop_tx.clone());
    drop(kanade_stop_tx);

    // ── 判定（集めてから 1 回） ──
    let readme = world
        .get_non_send::<ReadmeWiring>()
        .map(|w| w.path().to_path_buf());
    let frames = world
        .get_non_send::<MenuWiring>()
        .map(|w| w.registry.registered_frames());
    let kanade_stop_live = {
        let rx = &world.non_send::<KanadeNoticeRx>().0;
        loop {
            match rx.try_recv() {
                Ok(_) => continue,
                Err(err) => break err == TryRecvError::Empty,
            }
        }
    };
    let user_break_live = world
        .get_non_send_mut::<UserBreakWiring>()
        .is_some_and(|mut w| w.flag_source_connected());
    let exit_requested = world.non_send::<AppExit>().is_requested();
    let actual = (
        round1_readme_under_root1,
        round1_frames,
        systems_lens(&world),
        readme
            .as_deref()
            .is_some_and(|p| p.starts_with(sample2.folder())),
        frames,
        kanade_stop_live,
        user_break_live,
        exit_requested,
    );

    // ── 後片付け（有界） ──
    shutdown_bounded("2 周目を降ろす", session2);
    let _ = app_exit::close_windows_for_restart(&mut world);

    assert_eq!(
        actual,
        (
            true,
            Some(vec![
                Frame::Ghost,
                Frame::Shell,
                Frame::Readme,
                Frame::Close
            ]),
            lens_before,
            true,
            Some(vec![Frame::Ghost, Frame::Readme, Frame::Close]),
            true,
            true,
            false,
        ),
        "2 周目が 1 周目と同じ姿で起きていない（1 周目の説明書が 1 周目の根の下・1 周目の登記の一覧・\
         系の数 [Input, Update, FrameFinalize]・説明書が \
         2 周目の根の下・登記の一覧・停止通知の受け口が生きている・中断の旗が生きている・終了の指示）: \
         readme={readme:?} root1={:?} root2={:?} closed={closed_count}",
        sample1.folder(),
        sample2.folder(),
    );
}
