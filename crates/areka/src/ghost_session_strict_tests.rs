//! 厳格な起動の入口と起動入力の作り口の決定論テスト（areka-P0-ghost-shell-balloon-switch task 6.3）。

use areka_kanade::{BootOrigin, ChangedFrom};

use super::*;
use crate::boot_resolve::{BalloonRoute, GhostRoute};
use crate::input_events::MouseWiring;
use crate::menu::MenuWiring;
use crate::placement::test_support::capture_logs;

/// 結線が成立しない入力（ゴーストの根もバルーンの根も実在しない）。`wire_emo2_boot` は資産の
/// 組立で失敗し World に触れない。fallback の腕へ倒れれば LogSink の起動が試みられ、その起点
/// 不在が `warn!` で残る。
fn unwirable_inputs() -> GhostBootInputs {
    let (kanade_stop, _rx) = mpsc::channel::<KanadeNotice>();
    GhostBootInputs {
        wiring: emo2_boot::Emo2BootInputs {
            ghost_root: PathBuf::from("ghost_session_strict_tests/無い/ghost"),
            balloon_root: PathBuf::from("ghost_session_strict_tests/無い/balloon"),
            shiori: areka_ghost::ShioriWiring::Helper {
                helper_exe: PathBuf::from("ghost_session_strict_tests/使わない/helper.exe"),
            },
            ticker: areka_ghost::TickerMode::Disabled,
            app_profile_dir: None,
            boot_origin: BootOrigin::Plain,
        },
        helper_exe: PathBuf::from("ghost_session_strict_tests/使わない/helper.exe"),
        kanade_stop,
    }
}

fn decisions() -> (StartupDescriptValues, GhostDecision, BalloonDecision) {
    (
        StartupDescriptValues {
            author_dpi: placement::AuthorDpi::DEFAULT,
            zorder_raw: None,
        },
        GhostDecision {
            route: GhostRoute::Switched,
            dir: PathBuf::from("ghost_session_strict_tests/無い/ghost"),
            folder: None,
        },
        BalloonDecision {
            route: BalloonRoute::Argv,
            dir: PathBuf::from("ghost_session_strict_tests/無い/balloon"),
            folder: None,
        },
    )
}

/// fallback の腕（LogSink の起動）を試みた証の本文（起点不在の `warn!`・その他の失敗の `error!`・成立の `info!`）。
fn fallback_attempts(events: &[crate::placement::test_support::LogEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            let m = e.message();
            m.contains("ghost 結線層の起動") || m.contains("LogSink フォールバックで起動しました")
        })
        .count()
}

/// 厳格な入口（要件 6.1）: 結線が成立しなければ `Err` を返し、fallback の骨格を起こさない。
///
/// 対照として同じ入力を今日どおりの入口（[`boot_ghost`]）へ通すと fallback の腕へ倒れ、
/// LogSink の起動を試みた記録が 1 件残る——厳格な入口ではそれが 0 件で、結線ありの腕が挿す
/// 窓ごとの状態（マウス・メニュー）も World に無い。判定は集めてから 1 回。
#[test]
fn strict_boot_returns_err_without_falling_back() {
    let (descript, ghost, balloon) = decisions();

    let mut strict_world = World::new();
    let (strict, strict_events) = capture_logs(|| {
        boot_ghost_strict(
            &mut strict_world,
            unwirable_inputs(),
            &descript,
            &ghost,
            &balloon,
        )
    });
    let strict_is_err = matches!(strict, Err(BootWiringFailed));
    let strict_state = (
        strict_world.get_non_send::<MouseWiring>().is_some(),
        strict_world.get_non_send::<MenuWiring>().is_some(),
    );

    let mut lenient_world = World::new();
    let (lenient, lenient_events) = capture_logs(|| {
        boot_ghost(
            &mut lenient_world,
            unwirable_inputs(),
            &descript,
            &ghost,
            &balloon,
        )
    });
    let lenient_has_runtime = lenient.kanade().is_some();
    // 置き場（NonSend）へ置いて読み返す。実行系が無くても起動に渡した根は読める。
    lenient_world.insert_non_send(GhostSlot(Some(lenient)));
    let slot_dir = lenient_world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(|s| s.ghost_dir().to_path_buf());

    assert_eq!(
        (
            strict_is_err,
            fallback_attempts(&strict_events),
            strict_state,
            fallback_attempts(&lenient_events),
            lenient_has_runtime,
            slot_dir,
        ),
        (
            true,
            0,
            (false, false),
            1,
            false,
            Some(PathBuf::from("ghost_session_strict_tests/無い/ghost")),
        ),
        "厳格な入口が失敗を返さない／fallback へ倒れた（厳格の Err・厳格の fallback の記録・\
         厳格の窓ごとの状態 (マウス, メニュー)・対照の fallback の記録・対照の実行系・置き場の根）: \
         strict={strict_events:?} lenient={lenient_events:?}"
    );
}

/// 作り口（[`GhostBootInputsSource`]）に渡した由来が、起動の結線の入力（`Emo2BootInputs`）へ
/// そのまま載る（要件 8.6）。本番の組み立て（[`GhostBootInputs::production`]）を閉じた作り口で、
/// 3 つの由来をそれぞれ通す。
#[test]
fn origin_given_to_source_reaches_wiring_inputs() {
    let (kanade_stop, _rx) = mpsc::channel::<KanadeNotice>();
    let helper_exe = PathBuf::from("ghost_session_strict_tests/使わない/helper.exe");
    let source = GhostBootInputsSource(Box::new(move |cfg, origin| {
        GhostBootInputs::production(cfg, helper_exe.clone(), kanade_stop.clone(), origin)
    }));
    let cfg = ConfigInputs {
        ghost_root: PathBuf::from("ghost_session_strict_tests/ghost/B"),
        balloon_root: PathBuf::from("ghost_session_strict_tests/balloon/b"),
    };
    let origins = vec![
        BootOrigin::Plain,
        BootOrigin::ChangedFrom(ChangedFrom {
            sakura_name: "さくら".to_owned(),
            script: "\\0またね\\e".to_owned(),
            name: "A".to_owned(),
            dir: "C:/root/ghost/A".to_owned(),
        }),
        BootOrigin::Halted {
            ghost_name: "A".to_owned(),
        },
    ];

    let carried: Vec<BootOrigin> = origins
        .iter()
        .map(|origin| (source.0)(&cfg, origin.clone()).wiring.boot_origin)
        .collect();

    assert_eq!(carried, origins, "作り口に渡した由来が結線の入力へ載らない");
}

/// LogSink へ倒れた旗（要件 4.1・4.2・4.7・4.8）: 旗が立つのは fallback の腕だけで、倒れた先の
/// 起動の成否を問わない。結線ありの腕（切替の経路と同じ `boot_wired`）とテスト用の組み立ては偽。
///
/// 倒れた先が成功する形は、ゴーストの根は本物・バルーンの根だけが無い入力で作る（結線は資産の
/// 組立で失敗し、LogSink の起動は mount が通るので成功する。SHIORI は使わない helper の経路で
/// 接続に失敗し、kanade は非同期に止まる）。このとき LogSink の腕の App スコープの置き場は結線の
/// 入力のもの＝起動の直後の「最後に使ったゴースト」はその置き場へ書かれる（実行体の隣の既定の
/// 置き場へは書かない）。書き込みは降ろすときに反映されるので、降ろした後に読む。
///
/// # 非空虚性
/// fallback の腕で旗を立てないと 1・2 番目が偽で赤。App スコープの置き場を結線の入力から写さないと
/// 既定の置き場（`default_app_profile_dir`）へ書かれ、読み返しが `None` で赤。
#[test]
fn logsink_arm_sets_fallback_flag_and_uses_wiring_app_dir() {
    use crate::boot_resolve::read_last_ghost;
    use crate::emo2_boot::ghost_switch_test_support::{BALLOON, FakeShiori, SwitchRig};

    let (descript, ghost, balloon) = decisions();

    // 倒れた先も失敗する形（根がどちらも無い）。
    let mut failed_world = World::new();
    let failed = boot_ghost(
        &mut failed_world,
        unwirable_inputs(),
        &descript,
        &ghost,
        &balloon,
    );
    let failed_flag = (failed.logsink_fallback(), failed.runtime().is_some());

    // 倒れた先が成功する形（バルーンの根だけが無い）。
    let mut rig = SwitchRig::new(vec![("A", FakeShiori::ConnectFail)]);
    let app = rig.root.dir().join("fallback-app-profile");
    let ghost_a = GhostDecision {
        route: GhostRoute::Memory,
        dir: rig.root.ghost_dir("A"),
        folder: Some("A".to_owned()),
    };
    let balloon_missing = BalloonDecision {
        route: BalloonRoute::Companion,
        dir: rig.root.dir().join("balloon").join("無い"),
        folder: Some(BALLOON.to_owned()),
    };
    let (kanade_stop, _stop_rx) = mpsc::channel::<KanadeNotice>();
    let inputs = GhostBootInputs {
        wiring: emo2_boot::Emo2BootInputs {
            ghost_root: ghost_a.dir.clone(),
            balloon_root: balloon_missing.dir.clone(),
            shiori: areka_ghost::ShioriWiring::Helper {
                helper_exe: PathBuf::from("ghost_session_strict_tests/使わない/helper.exe"),
            },
            ticker: areka_ghost::TickerMode::Disabled,
            app_profile_dir: Some(app.clone()),
            boot_origin: BootOrigin::Plain,
        },
        helper_exe: PathBuf::from("ghost_session_strict_tests/使わない/helper.exe"),
        kanade_stop,
    };
    let fallen = boot_ghost(
        &mut rig.world,
        inputs,
        &descript,
        &ghost_a,
        &balloon_missing,
    );
    let fallen_flag = (fallen.logsink_fallback(), fallen.runtime().is_some());
    rig.world.insert_non_send(GhostSlot(Some(fallen)));
    let down = rig.shutdown();

    // 結線ありの腕（切替と同じ経路・同じ土台で起こし直す）とテスト用の組み立て。
    rig.boot("A");
    let wired_flag = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(GhostSession::logsink_fallback);
    let wired_down = rig.shutdown();
    let for_test = GhostSession::for_test(None, PathBuf::from("ghost_session_strict_tests/ghost"))
        .logsink_fallback();

    assert_eq!(
        (
            failed_flag,
            fallen_flag,
            down,
            read_last_ghost(&app),
            wired_flag,
            wired_down,
            for_test,
        ),
        (
            (true, false),
            (true, true),
            true,
            Some("A".to_owned()),
            Some(false),
            true,
            false,
        ),
        "LogSink へ倒れた旗か倒れた先の App スコープの置き場が崩れた（倒れた先も失敗 (旗, 実行系)・\
         倒れた先は成功 (旗, 実行系)・降ろせた・結線の入力の置き場の最後のゴースト・結線ありの旗・\
         降ろせた・テスト用の組み立ての旗）"
    );
}
