//! 切替の足場の待ちの檻（areka-P0-ghost-session-test-load-flake タスク 3.2・要件 2.1・3.1・3.2）。
//!
//! 檻 9: 足場の待ちが届かないとき、文言の元（[`WaitFailure`]）が「何を」に呼び出しの場所を持つ。
//! 檻 10: 足場の進みの目印は、偽の SHIORI の状態の問い合わせでは増えず、接続に失敗する作り口の回でも 1 つ増える。
//! どちらも実時間を待たない（檻 9 は偽の時計を渡し、檻 10 はゴーストを起こさず作り口を直に呼ぶ）。

use std::cell::Cell;
use std::time::{Duration, Instant};

use areka_ghost::ShioriWiring;
use areka_kanade::BootOrigin;

use super::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::WaitFailure;
use crate::ghost_session::GhostBootInputsSource;

/// 足場の作り口を `folder` について直に 1 回呼び、SHIORI の結線を返す（ゴーストは起こさない）。
fn call_factory(rig: &SwitchRig, folder: &str) -> ShioriWiring {
    let cfg = rig.cfg(folder);
    let inputs = (rig.world.non_send::<GhostBootInputsSource>().0)(&cfg, BootOrigin::Plain);
    inputs.wiring.shiori
}

/// 檻 9: 決して真にならない条件で `pump_until` の芯を、1 回ごとに 1 秒進む偽の時計で回す。
/// ゴーストを起こしていないので目印は増えず、偽の時計で `SPIN_WAIT` を越えた所で `［止まった］` になる。
/// その「何を」が、この檻の呼び出しの場所（ファイルと行）であること。
#[test]
fn a_rig_wait_that_never_arrives_names_the_callers_place() {
    let mut rig = SwitchRig::new(vec![("A", FakeShiori::ConnectFail)]);
    let clock = Cell::new(Instant::now());
    let tick = || {
        clock.set(clock.get() + Duration::from_secs(1));
        clock.get()
    };
    let line = line!() + 1;
    let result = rig.pump_until_with(tick, |_| {}, |_| false);
    let place = format!("{}:{line}", file!());
    match result {
        Err(WaitFailure::Stalled { what, moves, .. }) => {
            assert_eq!(what, place, "「何を」は pump_until を呼んだ場所");
            assert_eq!(moves, 0, "ゴーストを起こしていないので目印は増えない");
        }
        other => panic!("止まった待ちは Stalled で返る: {other:?}"),
    }
}

/// 檻 10: 状態の問い合わせでは目印が増えず、呼び出しでは増える。接続に失敗する回も作り口の 1 回に数える。
#[test]
fn the_rig_progress_ignores_status_and_counts_every_factory_call() {
    let rig = SwitchRig::new(vec![
        ("A", FakeShiori::Scripted(Box::new(|| standard_script("")))),
        ("B", FakeShiori::ConnectFail),
    ]);
    let probe = rig.progress_probe();
    assert_eq!(probe(), 0, "何も起こしていなければ 0");

    let ShioriWiring::Custom(connect) = call_factory(&rig, "A") else {
        panic!("台本つきの作り口は Custom の結線を返す");
    };
    assert_eq!(probe(), 1, "作り口の 1 回で 1 つ増える");
    let mut backend = connect().expect("台本つきの偽の SHIORI は接続できる");
    for _ in 0..5 {
        let _ = backend.status();
    }
    assert_eq!(
        rig.handle("A").call_count(),
        0,
        "状態の問い合わせは数えない"
    );
    assert_eq!(probe(), 1, "状態の問い合わせで目印は増えない");
    backend
        .notify("OnInitialize", &[], None)
        .expect("台本の OnInitialize");
    assert_eq!(probe(), 2, "状態の問い合わせでない呼び出しは 1 つ増える");

    let ShioriWiring::Custom(connect) = call_factory(&rig, "B") else {
        panic!("接続に失敗する作り口も Custom の結線を返す");
    };
    assert!(connect().is_err(), "B は接続に失敗する");
    assert_eq!(
        probe(),
        3,
        "接続に失敗する回（台帳に載らない）でも作り口の 1 回で 1 つ増える"
    );
}
