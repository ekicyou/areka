// =============================================================================
// 終わる相（`ghost_quit_system`）の据え付けの決定論テスト
//
// 毎フレームの相（`frame_system`）の登録先のテストは `areka-runtime` の
// `frame_schedule_tests.rs` が持つ。ここは本番の据え付けの関数 `wire_kanade_stop` を
// 空の World の `Schedules` へ直に呼び、終わる相が `Update` に載って停止通知 1 件で終了を
// 指示することと、毎フレームの相より前に走ることを見る（GPU も窓も要らない）。
// =============================================================================

use std::sync::mpsc;

use areka_kanade::{KanadeNotice, KanadeStopCause, KanadeStopped};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings, Schedules};
use wintf::ecs::Update;

use crate::app_exit::{ExitOrigin, FirstExit};
use crate::session::ghost_quit::{ghost_quit_system, wire_kanade_stop};
use areka_runtime::frame::frame_system;
use std::any::TypeId;

/// 毎フレームの相を登録しない World（順序の相手が居ない形）でも、`wire_kanade_stop` の登録は
/// bevy に受け入れられ、停止通知 1 件 → `Update` 1 回で終了が指示され最初の出所が残る。
///
/// 順序の相手（`frame_system`）が schedule に無い登録の形は bevy の挙動に依る 1 点なので、
/// 版が変わってこの形が拒まれたら（schedule の組み立てで落ちたら）ここが赤くなる。
/// 受け口の据え付けも `wire_kanade_stop` に任せるので、据えなくなれば終了が指示されず赤くなる。
#[test]
fn ghost_quit_system_without_the_frame_system_quits_on_one_notice() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(wintf::AppExit::new());
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    wire_kanade_stop(&mut world, rx);
    assert_eq!(
        world
            .resource::<Schedules>()
            .get(Update)
            .expect("登録後は `Update` の段が在る")
            .systems_len(),
        1,
        "`Update` に載るのは終了相 1 本だけ（毎フレームの相は載せない）"
    );

    tx.send(KanadeNotice::Stopped(KanadeStopped {
        cause: KanadeStopCause::Quit,
        handoff: None,
    }))
    .expect("停止通知を投函できる");
    world.run_schedule(Update);

    assert!(
        world
            .get_non_send::<wintf::AppExit>()
            .expect("受け口を挿してある")
            .is_requested(),
        "停止通知 1 件 → `Update` 1 回で終了が指示される"
    );
    assert_eq!(
        world.get_resource::<FirstExit>().map(|f| f.0.clone()),
        Some(ExitOrigin::KanadeStopped(KanadeStopCause::Quit)),
        "最初の出所が残る"
    );
}

/// 本番と同じ順（毎フレームの相の登録の後に `wire_kanade_stop`）で登録すると、`Update` の実行順で
/// 終了相が毎フレームの相より前に来る。順の指定が抜けると、止まったゴーストの毎フレームの相が
/// 終了の前にもう 1 回走る。
#[test]
fn ghost_quit_system_runs_before_the_frame_system() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    areka_runtime::register_frame_system(&mut world);
    let (_tx, rx) = mpsc::channel::<KanadeNotice>();
    wire_kanade_stop(&mut world, rx);

    let mut schedules = world
        .remove_resource::<Schedules>()
        .expect("段の表を据えてある");
    let update = schedules.get_mut(Update).expect("`Update` の段が在る");
    // 排他の相どうしは順の指定が無いと曖昧になるので、組み立てを曖昧さで落とす。
    update.set_build_settings(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Error,
        ..Default::default()
    });
    update
        .initialize(&mut world)
        .expect("`Update` を曖昧さなしに組める（終了相と毎フレームの相に順の指定がある）");
    let order: Vec<TypeId> = update
        .systems()
        .expect("組んだ後は実行順を読める")
        .map(|(_, system)| system.system_type())
        .collect();
    let at = |id: TypeId| order.iter().position(|o| *o == id);
    let quit = at(IntoSystem::into_system(ghost_quit_system).system_type()).expect("終了相が載る");
    let frame =
        at(IntoSystem::into_system(frame_system).system_type()).expect("毎フレームの相が載る");
    assert!(
        quit < frame,
        "終了相が毎フレームの相より後に走る: {order:?}"
    );
}
