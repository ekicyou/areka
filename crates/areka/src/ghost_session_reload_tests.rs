//! 実行系だけを外して付け直す口の決定論テスト（areka-P0-mcp-reload task 4.2・要件 4.2・7.3）。

use std::time::Duration;

use areka_kanade::CloseReason;
use sample_ghost_kit::SampleRoot;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::AppExit;

use super::*;
use crate::boot_resolve::{BalloonRoute, GhostRoute};
use crate::emo2_boot::runtime_boot;
use crate::emo2_boot::sample_test_support::acquire_emo2;
use crate::emo2_boot::spine::{Progress, ScriptedShioriHandle, SpineHarness, run_bounded_watching};

fn emo2_balloon(sample: &SampleRoot) -> PathBuf {
    sample
        .balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}

/// 偽の SHIORI（標準の台本）・時計なし・記憶の置き場なしの起動の結線の入力。
fn scripted_wiring(sample: &SampleRoot) -> (emo2_boot::Emo2BootInputs, ScriptedShioriHandle) {
    let (backend, handle) = SpineHarness::standard_backend("\\s[0]\\e");
    let wiring = emo2_boot::Emo2BootInputs {
        ghost_root: sample.folder().to_path_buf(),
        balloon_root: emo2_balloon(sample),
        shiori: areka_ghost::ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn areka_kanade::ShioriBackend>)
        })),
        ticker: areka_ghost::TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    };
    (wiring, handle)
}

/// 送出端の先に生きた kanade が居るか（空の照会を投げ、返事が来れば真）。
fn kanade_answers(kanade: &Sender<KanadeMsg>) -> bool {
    let (reply, receiver) = areka_actor::reply_channel();
    kanade
        .send(KanadeMsg::ResourceQuery { ids: vec![], reply })
        .is_ok()
        && receiver.recv_timeout(Duration::from_secs(30)).is_ok()
}

/// 有界に走らせる（偽の SHIORI の呼び出しの数を進みの目印にする）。
fn bounded(what: &str, shiori: &ScriptedShioriHandle, f: impl FnOnce() + Send + 'static) {
    let calls = || shiori.call_count();
    run_bounded_watching(what, Progress::Count(&calls), f);
}

/// 外して付け直す（要件 4.2・design Flow 3）: 結線ありで起こした単位から実行系だけを外すと、
/// seriko・loop ticker・seriko の送り手・束は単位に残り、kanade の送出端は外れる（2 度目は `None`）。
/// 外した実行系を降ろし、束から起こした新しい実行系を付けると、単位の送出端の先に生きた kanade
/// が居る（古い送出端の先は死んでいる＝写ったのは新しい方）。最後に単位を降ろすと seriko の join
/// が戻る（束を ① で落とさないと、束の seriko の送り手が inbox を生かし続けて戻らない）。
/// 判定は集めてから 1 回。
#[test]
fn split_keeps_window_side_and_attach_copies_new_kanade() {
    // SAFETY: 資産の焼き込み（WIC）に要る COM 初期化（既初期化の S_FALSE 等は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(AppExit::new());
    let (kanade_stop, kanade_stop_rx) = mpsc::channel::<KanadeNotice>();
    register_systems(&mut world, kanade_stop_rx);

    let sample = acquire_emo2();
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
        dir: emo2_balloon(&sample),
        folder: None,
    };
    let (wiring, shiori1) = scripted_wiring(&sample);
    let inputs = GhostBootInputs {
        wiring,
        helper_exe: PathBuf::from("ghost_session_reload_tests/使わない/helper.exe"),
        kanade_stop: kanade_stop.clone(),
    };
    let mut session = boot_ghost(&mut world, inputs, &descript, &ghost, &balloon);
    let wired = !session.logsink_fallback() && session.sink_sources().is_some();
    let shell = session.current_shell_folder();
    let old_kanade = session.kanade().cloned();

    // ── 外す ──
    let old = session.split_runtime();
    let after_split = (
        old.is_some(),
        session.kanade().is_none(),
        session.runtime().is_none(),
        session.seriko.is_some(),
        session.loop_ticker.is_some(),
        session.seriko_sink().is_some(),
        session.sink_sources().is_some(),
        session.split_runtime().is_none(),
    );
    if let Some(old) = old {
        bounded("外した実行系を降ろす", &shiori1, move || {
            old.shutdown(CloseReason::System)
                .expect("降ろすのは成功する");
        });
    }
    let old_dead = old_kanade.is_some_and(|k| !kanade_answers(&k));

    // ── 束から起こして付け直す ──
    let (wiring2, shiori2) = scripted_wiring(&sample);
    let sources = session.sink_sources().cloned().expect("束が残っている");
    let new = runtime_boot::boot_runtime(
        &mut world,
        wiring2,
        &sources,
        kanade_stop.clone(),
        shell.as_deref(),
    )
    .expect("束から新しい実行系が起きる");
    drop(sources);
    session.attach_runtime(new);
    let attached = (
        session.runtime().is_some(),
        session.kanade().is_some_and(kanade_answers),
    );

    // ── 降ろす（join が戻る） ──
    bounded("付け直した単位を降ろす", &shiori2, move || {
        session
            .shutdown(CloseReason::User { scope: 0 })
            .expect("降ろすのは成功する");
    });
    drop(kanade_stop);

    assert_eq!(
        (wired, after_split, old_dead, attached),
        (
            true,
            (true, true, true, true, true, true, true, true),
            true,
            (true, true)
        ),
        "外して付け直す口が設計どおりでない（結線ありで起きた・外した後 (外した実行系・送出端なし・\
         実行系なし・seriko・ticker・seriko の送り手・束・2 度目は None)・古い送出端の先が死んだ・\
         付け直した後 (実行系・送出端の先に生きた kanade)）"
    );
}

/// 名前の差し替え（要件 7.3）: 実行系があれば委ねて真を返し、`names()` が新しい名前を読む。
/// 実行系が無ければ偽。束は結線ありの腕だけが持つ（テスト用の組み立ては `None`）。
#[test]
fn set_names_delegates_to_runtime_or_returns_false() {
    let mut names = GhostNames::default();
    names.name = Some("読み直した名前".to_owned());

    let mut bare = GhostSession::for_test(None, PathBuf::from("ghost_session_reload_tests/根"));
    let bare_result = (bare.set_names(names.clone()), bare.sink_sources().is_none());

    // SAFETY: 資産の焼き込み（WIC）に要る COM 初期化（既初期化の S_FALSE 等は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(AppExit::new());
    let (kanade_stop, kanade_stop_rx) = mpsc::channel::<KanadeNotice>();
    register_systems(&mut world, kanade_stop_rx);
    let sample = acquire_emo2();
    let (wiring, shiori) = scripted_wiring(&sample);
    let ghost = GhostDecision {
        route: GhostRoute::Argv,
        dir: sample.folder().to_path_buf(),
        folder: None,
    };
    let balloon = BalloonDecision {
        route: BalloonRoute::Argv,
        dir: emo2_balloon(&sample),
        folder: None,
    };
    let mut session = boot_ghost(
        &mut world,
        GhostBootInputs {
            wiring,
            helper_exe: PathBuf::from("ghost_session_reload_tests/使わない/helper.exe"),
            kanade_stop,
        },
        &StartupDescriptValues {
            author_dpi: placement::AuthorDpi::DEFAULT,
            zorder_raw: None,
        },
        &ghost,
        &balloon,
    );
    let replaced = session.set_names(names.clone());
    let read_back = session.names().cloned();
    bounded("名前を替えた単位を降ろす", &shiori, move || {
        session
            .shutdown(CloseReason::User { scope: 0 })
            .expect("降ろすのは成功する");
    });

    assert_eq!(
        (bare_result, replaced, read_back),
        ((false, true), true, Some(names)),
        "名前の差し替えが実行系へ委ねられていない（実行系なし (偽・束なし)・実行系あり (真・読み返した名前)）"
    );
}
